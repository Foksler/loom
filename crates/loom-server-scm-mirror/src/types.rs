// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
	GitHub,
	GitLab,
}

impl Platform {
	pub fn as_str(&self) -> &'static str {
		match self {
			Platform::GitHub => "github",
			Platform::GitLab => "gitlab",
		}
	}

	pub fn parse(s: &str) -> Option<Self> {
		match s.to_lowercase().as_str() {
			"github" => Some(Platform::GitHub),
			"gitlab" => Some(Platform::GitLab),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushMirror {
	pub id: Uuid,
	pub repo_id: Uuid,
	pub remote_url: String,
	pub credential_key: String,
	pub enabled: bool,
	pub last_pushed_at: Option<DateTime<Utc>>,
	pub last_error: Option<String>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorBranchRule {
	pub mirror_id: Uuid,
	pub pattern: String,
	pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMirror {
	pub id: Uuid,
	pub platform: Platform,
	pub external_owner: String,
	pub external_repo: String,
	pub repo_id: Uuid,
	pub last_synced_at: Option<DateTime<Utc>>,
	pub last_accessed_at: Option<DateTime<Utc>>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreatePushMirror {
	pub repo_id: Uuid,
	pub remote_url: String,
	pub credential_key: String,
	pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct CreateExternalMirror {
	pub platform: Platform,
	pub external_owner: String,
	pub external_repo: String,
	pub repo_id: Uuid,
}
