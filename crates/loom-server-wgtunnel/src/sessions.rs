// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use crate::config::WgTunnelConfig;
use crate::devices::DeviceService;
use crate::error::{Result, WgError};
use crate::ip_allocator::IpAllocator;
use crate::peer_stream::{PeerEvent, PeerNotifier};
use crate::weavers::WeaverWgService;
use base64::prelude::*;
use chrono::{DateTime, Utc};
use loom_wgtunnel_common::DerpMap;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::net::Ipv6Addr;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
	pub id: Uuid,
	pub device_id: Uuid,
	pub weaver_id: Uuid,
	pub client_ip: Ipv6Addr,
	pub created_at: DateTime<Utc>,
	pub last_handshake_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
	pub weaver_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
	pub session_id: Uuid,
	pub client_ip: Ipv6Addr,
	pub weaver_ip: Ipv6Addr,
	pub weaver_public_key: String,
	pub derp_map: DerpMap,
}

#[derive(Debug, Clone)]
struct SessionRow {
	id: String,
	device_id: String,
	weaver_id: String,
	client_ip: String,
	created_at: String,
	last_handshake_at: Option<String>,
}

impl TryFrom<SessionRow> for Session {
	type Error = WgError;

	fn try_from(row: SessionRow) -> Result<Self> {
		Ok(Session {
			id: row
				.id
				.parse()
				.map_err(|_| WgError::Internal("invalid session id".to_string()))?,
			device_id: row
				.device_id
				.parse()
				.map_err(|_| WgError::Internal("invalid device id".to_string()))?,
			weaver_id: row
				.weaver_id
				.parse()
				.map_err(|_| WgError::Internal("invalid weaver id".to_string()))?,
			client_ip: row
				.client_ip
				.parse()
				.map_err(|_| WgError::IpAllocation("invalid client IP".to_string()))?,
			created_at: parse_datetime(&row.created_at)?,
			last_handshake_at: row
				.last_handshake_at
				.as_ref()
				.map(|s| parse_datetime(s))
				.transpose()?,
		})
	}
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
	DateTime::parse_from_rfc3339(s)
		.map(|dt| dt.with_timezone(&Utc))
		.or_else(|_| {
			chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
				.map(|ndt| ndt.and_utc())
				.map_err(|_| WgError::Internal(format!("invalid datetime: {s}")))
		})
}

#[derive(Clone)]
pub struct SessionService {
	db: SqlitePool,
	ip_allocator: Arc<IpAllocator>,
	peer_notifier: Arc<PeerNotifier>,
	device_service: DeviceService,
	weaver_service: WeaverWgService,
	config: Arc<WgTunnelConfig>,
}

impl SessionService {
	pub fn new(
		db: SqlitePool,
		ip_allocator: Arc<IpAllocator>,
		peer_notifier: Arc<PeerNotifier>,
		device_service: DeviceService,
		weaver_service: WeaverWgService,
		config: Arc<WgTunnelConfig>,
	) -> Self {
		Self {
			db,
			ip_allocator,
			peer_notifier,
			device_service,
			weaver_service,
			config,
		}
	}

	#[instrument(skip(self), fields(%device_id, %weaver_id))]
	pub async fn create(&self, device_id: Uuid, weaver_id: Uuid) -> Result<CreateSessionResponse> {
		let device = self
			.device_service
			.get(device_id)
			.await?
			.ok_or(WgError::DeviceNotFound)?;

		if device.revoked_at.is_some() {
			return Err(WgError::DeviceRevoked);
		}

		let weaver = self
			.weaver_service
			.get(weaver_id)
			.await?
			.ok_or(WgError::WeaverNotFound)?;

		let existing = self.get_by_device_weaver(device_id, weaver_id).await?;
		if existing.is_some() {
			return Err(WgError::SessionAlreadyExists);
		}

		let session_id = Uuid::new_v4();
		let client_ip = self.ip_allocator.allocate_client_ip(session_id).await?;

		sqlx::query(
			"INSERT INTO wg_sessions (id, device_id, weaver_id, client_ip, created_at)
             VALUES (?, ?, ?, ?, datetime('now'))",
		)
		.bind(session_id.to_string())
		.bind(device_id.to_string())
		.bind(weaver_id.to_string())
		.bind(client_ip.to_string())
		.execute(&self.db)
		.await?;

		let event = PeerEvent::PeerAdded {
			public_key: BASE64_STANDARD.encode(device.public_key),
			allowed_ip: format!("{}/128", client_ip),
			session_id: session_id.to_string(),
		};
		self.peer_notifier.notify_peer_added(weaver_id, event).await;

		let derp_map = self.config.get_derp_map().cloned().unwrap_or_default();

		Ok(CreateSessionResponse {
			session_id,
			client_ip,
			weaver_ip: weaver.assigned_ip,
			weaver_public_key: weaver.public_key_base64(),
			derp_map,
		})
	}

	async fn get_by_device_weaver(
		&self,
		device_id: Uuid,
		weaver_id: Uuid,
	) -> Result<Option<Session>> {
		let row: Option<(String, String, String, String, String, Option<String>)> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
             FROM wg_sessions WHERE device_id = ? AND weaver_id = ?",
		)
		.bind(device_id.to_string())
		.bind(weaver_id.to_string())
		.fetch_optional(&self.db)
		.await?;

		match row {
			Some((id, device_id, weaver_id, client_ip, created_at, last_handshake_at)) => {
				let session = SessionRow {
					id,
					device_id,
					weaver_id,
					client_ip,
					created_at,
					last_handshake_at,
				}
				.try_into()?;
				Ok(Some(session))
			}
			None => Ok(None),
		}
	}

	#[instrument(skip(self), fields(%device_id))]
	pub async fn list_for_device(&self, device_id: Uuid) -> Result<Vec<Session>> {
		let rows: Vec<(String, String, String, String, String, Option<String>)> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
             FROM wg_sessions WHERE device_id = ?
             ORDER BY created_at DESC",
		)
		.bind(device_id.to_string())
		.fetch_all(&self.db)
		.await?;

		rows
			.into_iter()
			.map(
				|(id, device_id, weaver_id, client_ip, created_at, last_handshake_at)| {
					SessionRow {
						id,
						device_id,
						weaver_id,
						client_ip,
						created_at,
						last_handshake_at,
					}
					.try_into()
				},
			)
			.collect()
	}

	#[instrument(skip(self), fields(%weaver_id))]
	pub async fn list_for_weaver(&self, weaver_id: Uuid) -> Result<Vec<Session>> {
		let rows: Vec<(String, String, String, String, String, Option<String>)> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
             FROM wg_sessions WHERE weaver_id = ?
             ORDER BY created_at DESC",
		)
		.bind(weaver_id.to_string())
		.fetch_all(&self.db)
		.await?;

		rows
			.into_iter()
			.map(
				|(id, device_id, weaver_id, client_ip, created_at, last_handshake_at)| {
					SessionRow {
						id,
						device_id,
						weaver_id,
						client_ip,
						created_at,
						last_handshake_at,
					}
					.try_into()
				},
			)
			.collect()
	}

	#[instrument(skip(self), fields(%session_id))]
	pub async fn terminate(&self, session_id: Uuid) -> Result<()> {
		let session = self
			.get(session_id)
			.await?
			.ok_or(WgError::SessionNotFound)?;

		let device = self.device_service.get(session.device_id).await?;

		self.ip_allocator.release_ip(session.client_ip).await?;

		let result = sqlx::query("DELETE FROM wg_sessions WHERE id = ?")
			.bind(session_id.to_string())
			.execute(&self.db)
			.await?;

		if result.rows_affected() == 0 {
			return Err(WgError::SessionNotFound);
		}

		if let Some(device) = device {
			let event = PeerEvent::PeerRemoved {
				public_key: BASE64_STANDARD.encode(device.public_key),
				session_id: session_id.to_string(),
			};
			self
				.peer_notifier
				.notify_peer_removed(session.weaver_id, event)
				.await;
		}

		Ok(())
	}

	#[instrument(skip(self), fields(%session_id))]
	pub async fn get(&self, session_id: Uuid) -> Result<Option<Session>> {
		let row: Option<(String, String, String, String, String, Option<String>)> = sqlx::query_as(
			"SELECT id, device_id, weaver_id, client_ip, created_at, last_handshake_at
             FROM wg_sessions WHERE id = ?",
		)
		.bind(session_id.to_string())
		.fetch_optional(&self.db)
		.await?;

		match row {
			Some((id, device_id, weaver_id, client_ip, created_at, last_handshake_at)) => {
				let session = SessionRow {
					id,
					device_id,
					weaver_id,
					client_ip,
					created_at,
					last_handshake_at,
				}
				.try_into()?;
				Ok(Some(session))
			}
			None => Ok(None),
		}
	}

	#[instrument(skip(self), fields(%session_id))]
	pub async fn update_handshake(&self, session_id: Uuid) -> Result<()> {
		sqlx::query("UPDATE wg_sessions SET last_handshake_at = datetime('now') WHERE id = ?")
			.bind(session_id.to_string())
			.execute(&self.db)
			.await?;

		Ok(())
	}
}
