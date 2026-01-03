// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use crate::error::{Result, WgError};
use crate::ip_allocator::IpAllocator;
use base64::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::net::Ipv6Addr;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaverWg {
	pub weaver_id: Uuid,
	pub public_key: [u8; 32],
	pub assigned_ip: Ipv6Addr,
	pub derp_home_region: Option<u16>,
	pub endpoint: Option<String>,
	pub registered_at: DateTime<Utc>,
	pub last_seen_at: Option<DateTime<Utc>>,
}

impl WeaverWg {
	pub fn public_key_base64(&self) -> String {
		BASE64_STANDARD.encode(self.public_key)
	}
}

#[derive(Debug, Clone)]
struct WeaverRow {
	weaver_id: String,
	public_key: Vec<u8>,
	assigned_ip: String,
	derp_home_region: Option<i64>,
	endpoint: Option<String>,
	registered_at: String,
	last_seen_at: Option<String>,
}

impl TryFrom<WeaverRow> for WeaverWg {
	type Error = WgError;

	fn try_from(row: WeaverRow) -> Result<Self> {
		let public_key: [u8; 32] = row
			.public_key
			.try_into()
			.map_err(|_| WgError::InvalidPublicKey("invalid key length".to_string()))?;

		let assigned_ip: Ipv6Addr = row
			.assigned_ip
			.parse()
			.map_err(|_| WgError::IpAllocation("invalid assigned IP".to_string()))?;

		Ok(WeaverWg {
			weaver_id: row
				.weaver_id
				.parse()
				.map_err(|_| WgError::Internal("invalid weaver id".to_string()))?,
			public_key,
			assigned_ip,
			derp_home_region: row.derp_home_region.map(|r| r as u16),
			endpoint: row.endpoint,
			registered_at: parse_datetime(&row.registered_at)?,
			last_seen_at: row.last_seen_at.as_ref().map(|s| parse_datetime(s)).transpose()?,
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
pub struct WeaverWgService {
	db: SqlitePool,
	ip_allocator: Arc<IpAllocator>,
}

impl WeaverWgService {
	pub fn new(db: SqlitePool, ip_allocator: Arc<IpAllocator>) -> Self {
		Self { db, ip_allocator }
	}

	#[instrument(skip(self, public_key), fields(%weaver_id, derp_region = ?derp_region))]
	pub async fn register(
		&self,
		weaver_id: Uuid,
		public_key: [u8; 32],
		derp_region: Option<u16>,
	) -> Result<WeaverWg> {
		let existing = self.get(weaver_id).await?;
		if existing.is_some() {
			return Err(WgError::WeaverAlreadyRegistered);
		}

		let assigned_ip = self.ip_allocator.allocate_weaver_ip(weaver_id).await?;
		let now = Utc::now();

		sqlx::query(
			"INSERT INTO wg_weavers (weaver_id, public_key, assigned_ip, derp_home_region, registered_at)
             VALUES (?, ?, ?, ?, datetime('now'))",
		)
		.bind(weaver_id.to_string())
		.bind(public_key.as_slice())
		.bind(assigned_ip.to_string())
		.bind(derp_region.map(|r| r as i64))
		.execute(&self.db)
		.await?;

		Ok(WeaverWg {
			weaver_id,
			public_key,
			assigned_ip,
			derp_home_region: derp_region,
			endpoint: None,
			registered_at: now,
			last_seen_at: None,
		})
	}

	#[instrument(skip(self), fields(%weaver_id))]
	pub async fn get(&self, weaver_id: Uuid) -> Result<Option<WeaverWg>> {
		let row: Option<(String, Vec<u8>, String, Option<i64>, Option<String>, String, Option<String>)> =
			sqlx::query_as(
				"SELECT weaver_id, public_key, assigned_ip, derp_home_region, endpoint, registered_at, last_seen_at
                 FROM wg_weavers WHERE weaver_id = ?",
			)
			.bind(weaver_id.to_string())
			.fetch_optional(&self.db)
			.await?;

		match row {
			Some((weaver_id, public_key, assigned_ip, derp_home_region, endpoint, registered_at, last_seen_at)) => {
				let weaver = WeaverRow {
					weaver_id,
					public_key,
					assigned_ip,
					derp_home_region,
					endpoint,
					registered_at,
					last_seen_at,
				}
				.try_into()?;
				Ok(Some(weaver))
			}
			None => Ok(None),
		}
	}

	#[instrument(skip(self), fields(%weaver_id))]
	pub async fn unregister(&self, weaver_id: Uuid) -> Result<()> {
		let weaver = self.get(weaver_id).await?;

		if let Some(w) = weaver {
			self.ip_allocator.release_ip(w.assigned_ip).await?;
		}

		let result = sqlx::query("DELETE FROM wg_weavers WHERE weaver_id = ?")
			.bind(weaver_id.to_string())
			.execute(&self.db)
			.await?;

		if result.rows_affected() == 0 {
			return Err(WgError::WeaverNotFound);
		}

		Ok(())
	}

	#[instrument(skip(self), fields(%weaver_id, %endpoint))]
	pub async fn update_endpoint(&self, weaver_id: Uuid, endpoint: &str) -> Result<()> {
		let result = sqlx::query("UPDATE wg_weavers SET endpoint = ? WHERE weaver_id = ?")
			.bind(endpoint)
			.bind(weaver_id.to_string())
			.execute(&self.db)
			.await?;

		if result.rows_affected() == 0 {
			return Err(WgError::WeaverNotFound);
		}

		Ok(())
	}

	#[instrument(skip(self), fields(%weaver_id))]
	pub async fn update_last_seen(&self, weaver_id: Uuid) -> Result<()> {
		sqlx::query("UPDATE wg_weavers SET last_seen_at = datetime('now') WHERE weaver_id = ?")
			.bind(weaver_id.to_string())
			.execute(&self.db)
			.await?;

		Ok(())
	}
}
