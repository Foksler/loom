// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use crate::error::{Result, WgError};
use sqlx::SqlitePool;
use std::net::Ipv6Addr;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::instrument;
use uuid::Uuid;

const WEAVER_SUBNET_BASE: u128 = 0xfd7a_115c_a1e0_0001_0000_0000_0000_0000;
const CLIENT_SUBNET_BASE: u128 = 0xfd7a_115c_a1e0_0002_0000_0000_0000_0000;
const SUBNET_HOST_MASK: u128 = 0x0000_0000_0000_0000_FFFF_FFFF_FFFF_FFFF;

pub struct IpAllocator {
	db: SqlitePool,
	weaver_counter: AtomicU64,
	client_counter: AtomicU64,
}

impl IpAllocator {
	pub async fn new(db: SqlitePool) -> Result<Self> {
		let allocator = Self {
			db,
			weaver_counter: AtomicU64::new(1),
			client_counter: AtomicU64::new(1),
		};
		allocator.initialize_counters().await?;
		Ok(allocator)
	}

	async fn initialize_counters(&self) -> Result<()> {
		let weaver_max = self.get_max_host_number("weaver").await?;
		let client_max = self.get_max_host_number("client").await?;

		self.weaver_counter.store(weaver_max + 1, Ordering::SeqCst);
		self.client_counter.store(client_max + 1, Ordering::SeqCst);

		Ok(())
	}

	async fn get_max_host_number(&self, allocation_type: &str) -> Result<u64> {
		let ips: Vec<(String,)> = sqlx::query_as(
			"SELECT ip FROM wg_ip_allocations WHERE allocation_type = ? AND released_at IS NULL",
		)
		.bind(allocation_type)
		.fetch_all(&self.db)
		.await?;

		let mut max_host: u64 = 0;
		for (ip_str,) in ips {
			if let Ok(addr) = ip_str.parse::<Ipv6Addr>() {
				let addr_u128: u128 = addr.into();
				let host = (addr_u128 & SUBNET_HOST_MASK) as u64;
				if host > max_host {
					max_host = host;
				}
			}
		}

		Ok(max_host)
	}

	#[instrument(skip(self), fields(%weaver_id))]
	pub async fn allocate_weaver_ip(&self, weaver_id: Uuid) -> Result<Ipv6Addr> {
		let existing: Option<(String,)> =
			sqlx::query_as("SELECT ip FROM wg_ip_allocations WHERE entity_id = ? AND released_at IS NULL")
				.bind(weaver_id.to_string())
				.fetch_optional(&self.db)
				.await?;

		if let Some((ip_str,)) = existing {
			return ip_str
				.parse()
				.map_err(|_| WgError::IpAllocation("invalid stored IP".to_string()));
		}

		let host = self.weaver_counter.fetch_add(1, Ordering::SeqCst);
		if host as u128 > SUBNET_HOST_MASK {
			return Err(WgError::IpAllocation("weaver IP pool exhausted".to_string()));
		}

		let addr = Ipv6Addr::from(WEAVER_SUBNET_BASE | (host as u128));
		let ip_str = addr.to_string();

		sqlx::query(
			"INSERT INTO wg_ip_allocations (ip, allocation_type, entity_id, allocated_at)
             VALUES (?, 'weaver', ?, datetime('now'))",
		)
		.bind(&ip_str)
		.bind(weaver_id.to_string())
		.execute(&self.db)
		.await?;

		Ok(addr)
	}

	#[instrument(skip(self), fields(%session_id))]
	pub async fn allocate_client_ip(&self, session_id: Uuid) -> Result<Ipv6Addr> {
		let existing: Option<(String,)> =
			sqlx::query_as("SELECT ip FROM wg_ip_allocations WHERE entity_id = ? AND released_at IS NULL")
				.bind(session_id.to_string())
				.fetch_optional(&self.db)
				.await?;

		if let Some((ip_str,)) = existing {
			return ip_str
				.parse()
				.map_err(|_| WgError::IpAllocation("invalid stored IP".to_string()));
		}

		let host = self.client_counter.fetch_add(1, Ordering::SeqCst);
		if host as u128 > SUBNET_HOST_MASK {
			return Err(WgError::IpAllocation("client IP pool exhausted".to_string()));
		}

		let addr = Ipv6Addr::from(CLIENT_SUBNET_BASE | (host as u128));
		let ip_str = addr.to_string();

		sqlx::query(
			"INSERT INTO wg_ip_allocations (ip, allocation_type, entity_id, allocated_at)
             VALUES (?, 'client', ?, datetime('now'))",
		)
		.bind(&ip_str)
		.bind(session_id.to_string())
		.execute(&self.db)
		.await?;

		Ok(addr)
	}

	#[instrument(skip(self), fields(%ip))]
	pub async fn release_ip(&self, ip: Ipv6Addr) -> Result<()> {
		sqlx::query("UPDATE wg_ip_allocations SET released_at = datetime('now') WHERE ip = ?")
			.bind(ip.to_string())
			.execute(&self.db)
			.await?;

		Ok(())
	}
}
