// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use serde::{Deserialize, Serialize};

use crate::filter::AuditFilterConfig;

const DEFAULT_QUEUE_CAPACITY: usize = 10000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
	pub enabled: bool,
	pub retention_days: Option<i64>,
	pub global_filter: AuditFilterConfig,
	pub queue: QueueConfig,
}

impl Default for AuditConfig {
	fn default() -> Self {
		Self {
			enabled: true,
			retention_days: None,
			global_filter: AuditFilterConfig::default(),
			queue: QueueConfig::default(),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
	pub capacity: usize,
	pub overflow_policy: QueueOverflowPolicy,
}

impl Default for QueueConfig {
	fn default() -> Self {
		Self {
			capacity: DEFAULT_QUEUE_CAPACITY,
			overflow_policy: QueueOverflowPolicy::default(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum QueueOverflowPolicy {
	#[default]
	DropNewest,
	DropOldest,
	Block,
}
