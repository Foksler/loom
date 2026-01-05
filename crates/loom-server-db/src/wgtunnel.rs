// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! WireGuard tunnel repository for database operations.
//!
//! This module provides database access for WireGuard tunnel components:
//! - Weaver registration and management
//! - Device registration and management
//! - Session tracking
//! - IP allocation

use sqlx::sqlite::SqlitePool;
use std::net::Ipv6Addr;
use uuid::Uuid;

use crate::error::DbError;

pub type WeaverRowTuple = (
	String,
	Vec<u8>,
	String,
	Option<i64>,
	Option<String>,
	String,
	Option<String>,
);

pub type DeviceRowTuple = (
	String,
	String,
	Vec<u8>,
	Option<String>,
	String,
	Option<String>,
	Option<String>,
);

pub type SessionRowTuple = (String, String, String, String, String, Option<String>);

pub type IpAllocationRow = (String,);

/// Repository for WireGuard tunnel database operations.
#[derive(Clone)]
pub struct WgTunnelRepository {
	pool: SqlitePool,
}

impl WgTunnelRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	// =========================================================================
	// Weaver Operations
	// =========================================================================

	#[tracing::instrument(skip(self, public_key), fields(%weaver_id))]
	pub async fn insert_weaver(
		&self,
		weaver_id: Uuid,
		public_key: &[u8],
		assigned_ip: Ipv6Addr,
		derp_region: Option<u16>,
	) -> Result<(), DbError> {
		sqlx::query(
			"INSERT INTO wg_weavers (weaver_id, public_key, assigned_ip, derp_home_region, registered_at)
			 VALUES (?, ?, ?, ?, datetime('now'))",
		)
		.bind(weaver_id.to_string())
		.bind(public_key)
		.bind(assigned_ip.to_string())
		.bind(derp_region.map(|r| r as i64))
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), fields(%weaver_id))]
	pub async fn get_weaver(&self, weaver_id: Uuid) -> Result<Option<WeaverRowTuple>, DbError> {
		let row: Option<WeaverRowTuple> = sqlx::query_as(
			"SELECT weaver_id, public_key, assigned_ip, derp_home_region, endpoint, registered_at, last_seen_at
			 FROM wg_weavers WHERE weaver_id = ?",
		)
		.bind(weaver_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		Ok(row)
	}

	#[tracing::instrument(skip(self), fields(%weaver_id))]
	pub async fn delete_weaver(&self, weaver_id: Uuid) -> Result<u64, DbError> {
		let result = sqlx::query("DELETE FROM wg_weavers WHERE weaver_id = ?")
			.bind(weaver_id.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected())
	}

	#[tracing::instrument(skip(self), fields(%weaver_id, %endpoint))]
	pub async fn update_weaver_endpoint(
		&self,
		weaver_id: Uuid,
		endpoint: &str,
	) -> Result<u64, DbError> {
		let result = sqlx::query("UPDATE wg_weavers SET endpoint = ? WHERE weaver_id = ?")
			.bind(endpoint)
			.bind(weaver_id.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected())
	}

	#[tracing::instrument(skip(self), fields(%weaver_id))]
	pub async fn update_weaver_last_seen(&self, weaver_id: Uuid) -> Result<u64, DbError> {
		let result =
			sqlx::query("UPDATE wg_weavers SET last_seen_at = datetime('now') WHERE weaver_id = ?")
				.bind(weaver_id.to_string())
				.execute(&self.pool)
				.await?;

		Ok(result.rows_affected())
	}

	// =========================================================================
	// Device Operations
	// =========================================================================

	#[tracing::instrument(skip(self, public_key), fields(%id, %user_id))]
	pub async fn insert_device(
		&self,
		id: Uuid,
		user_id: Uuid,
		public_key: &[u8],
		name: Option<&str>,
	) -> Result<(), DbError> {
		sqlx::query(
			"INSERT INTO wg_devices (id, user_id, public_key, name, created_at)
			 VALUES (?, ?, ?, ?, datetime('now'))",
		)
		.bind(id.to_string())
		.bind(user_id.to_string())
		.bind(public_key)
		.bind(name)
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), fields(%user_id))]
	pub async fn list_devices_for_user(&self, user_id: Uuid) -> Result<Vec<DeviceRowTuple>, DbError> {
		let rows: Vec<DeviceRowTuple> = sqlx::query_as(
			"SELECT id, user_id, public_key, name, created_at, last_seen_at, revoked_at
			 FROM wg_devices WHERE user_id = ? AND revoked_at IS NULL
			 ORDER BY created_at DESC",
		)
		.bind(user_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		Ok(rows)
	}

	#[tracing::instrument(skip(self), fields(%id))]
	pub async fn get_device(&self, id: Uuid) -> Result<Option<DeviceRowTuple>, DbError> {
		let row: Option<DeviceRowTuple> = sqlx::query_as(
			"SELECT id, user_id, public_key, name, created_at, last_seen_at, revoked_at
			 FROM wg_devices WHERE id = ?",
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		Ok(row)
	}

	#[tracing::instrument(skip(self, public_key))]
	pub async fn get_device_by_public_key(
		&self,
		public_key: &[u8],
	) -> Result<Option<DeviceRowTuple>, DbError> {
		let row: Option<DeviceRowTuple> = sqlx::query_as(
			"SELECT id, user_id, public_key, name, created_at, last_seen_at, revoked_at
			 FROM wg_devices WHERE public_key = ?",
		)
		.bind(public_key)
		.fetch_optional(&self.pool)
		.await?;

		Ok(row)
	}

	#[tracing::instrument(skip(self), fields(%id, %user_id))]
	pub async fn revoke_device(&self, id: Uuid, user_id: Uuid) -> Result<u64, DbError> {
		let result = sqlx::query(
			"UPDATE wg_devices SET revoked_at = datetime('now')
			 WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
		)
		.bind(id.to_string())
		.bind(user_id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(result.rows_affected())
	}

	#[tracing::instrument(skip(self), fields(%id))]
	pub async fn update_device_last_seen(&self, id: Uuid) -> Result<u64, DbError> {
		let result = sqlx::query("UPDATE wg_devices SET last_seen_at = datetime('now') WHERE id = ?")
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected())
	}

	// =========================================================================
	// Session Operations
	// =========================================================================

	#[tracing::instrument(skip(self), fields(%id, %device_id, %weaver_id))]
	pub async fn insert_session(
		&self,
		id: Uuid,
		device_id: Uuid,
		weaver_id: Uuid,
		client_ip: Ipv6Addr,
	) -> Result<(), DbError> {
		sqlx::query(
			"INSERT INTO wg_sessions (id, device_id, weaver_id, client_ip, created_at)
			 VALUES (?, ?, ?, ?, datetime('now'))",
		)
		.bind(id.to_string())
		.bind(device_id.to_string())
		.bind(weaver_id.to_string())
		.bind(client_ip.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), fields(%device_id, %weaver_id))]
	pub async fn get_session_by_device_weaver(
		&self,
		device_id: Uuid,
		weaver_id: Uuid,
	) -> Result<Option<SessionRowTuple>, DbError> {
		let row: Option<SessionRowTuple> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
			 FROM wg_sessions WHERE device_id = ? AND weaver_id = ?",
		)
		.bind(device_id.to_string())
		.bind(weaver_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		Ok(row)
	}

	#[tracing::instrument(skip(self), fields(%device_id))]
	pub async fn list_sessions_for_device(
		&self,
		device_id: Uuid,
	) -> Result<Vec<SessionRowTuple>, DbError> {
		let rows: Vec<SessionRowTuple> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
			 FROM wg_sessions WHERE device_id = ?
			 ORDER BY created_at DESC",
		)
		.bind(device_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		Ok(rows)
	}

	#[tracing::instrument(skip(self), fields(%weaver_id))]
	pub async fn list_sessions_for_weaver(
		&self,
		weaver_id: Uuid,
	) -> Result<Vec<SessionRowTuple>, DbError> {
		let rows: Vec<SessionRowTuple> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
			 FROM wg_sessions WHERE weaver_id = ?
			 ORDER BY created_at DESC",
		)
		.bind(weaver_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		Ok(rows)
	}

	#[tracing::instrument(skip(self), fields(%id))]
	pub async fn get_session(&self, id: Uuid) -> Result<Option<SessionRowTuple>, DbError> {
		let row: Option<SessionRowTuple> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
			 FROM wg_sessions WHERE id = ?",
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		Ok(row)
	}

	#[tracing::instrument(skip(self), fields(%id))]
	pub async fn delete_session(&self, id: Uuid) -> Result<u64, DbError> {
		let result = sqlx::query("DELETE FROM wg_sessions WHERE id = ?")
			.bind(id.to_string())
			.execute(&self.pool)
			.await?;

		Ok(result.rows_affected())
	}

	#[tracing::instrument(skip(self), fields(%id))]
	pub async fn update_session_handshake(&self, id: Uuid) -> Result<u64, DbError> {
		let result =
			sqlx::query("UPDATE wg_sessions SET last_handshake_at = datetime('now') WHERE id = ?")
				.bind(id.to_string())
				.execute(&self.pool)
				.await?;

		Ok(result.rows_affected())
	}

	// =========================================================================
	// IP Allocation Operations
	// =========================================================================

	#[tracing::instrument(skip(self))]
	pub async fn get_allocated_ips_by_type(
		&self,
		allocation_type: &str,
	) -> Result<Vec<IpAllocationRow>, DbError> {
		let rows: Vec<IpAllocationRow> = sqlx::query_as(
			"SELECT ip FROM wg_ip_allocations WHERE allocation_type = ? AND released_at IS NULL",
		)
		.bind(allocation_type)
		.fetch_all(&self.pool)
		.await?;

		Ok(rows)
	}

	#[tracing::instrument(skip(self), fields(%entity_id))]
	pub async fn get_allocation_for_entity(
		&self,
		entity_id: Uuid,
	) -> Result<Option<IpAllocationRow>, DbError> {
		let row: Option<IpAllocationRow> = sqlx::query_as(
			"SELECT ip FROM wg_ip_allocations WHERE entity_id = ? AND released_at IS NULL",
		)
		.bind(entity_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		Ok(row)
	}

	#[tracing::instrument(skip(self), fields(%ip, %entity_id))]
	pub async fn insert_ip_allocation(
		&self,
		ip: &str,
		allocation_type: &str,
		entity_id: Uuid,
	) -> Result<(), DbError> {
		sqlx::query(
			"INSERT INTO wg_ip_allocations (ip, allocation_type, entity_id, allocated_at)
			 VALUES (?, ?, ?, datetime('now'))",
		)
		.bind(ip)
		.bind(allocation_type)
		.bind(entity_id.to_string())
		.execute(&self.pool)
		.await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), fields(%ip))]
	pub async fn release_ip(&self, ip: Ipv6Addr) -> Result<u64, DbError> {
		let result =
			sqlx::query("UPDATE wg_ip_allocations SET released_at = datetime('now') WHERE ip = ?")
				.bind(ip.to_string())
				.execute(&self.pool)
				.await?;

		Ok(result.rows_affected())
	}
}
