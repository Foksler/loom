// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Core types for the clips system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Unique identifier for a clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClipId(pub uuid::Uuid);

impl ClipId {
	/// Create a new random ClipId using UUIDv7.
	pub fn new() -> Self {
		Self(uuid::Uuid::now_v7())
	}
}

impl Default for ClipId {
	fn default() -> Self {
		Self::new()
	}
}

impl fmt::Display for ClipId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl FromStr for ClipId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(uuid::Uuid::parse_str(s)?))
	}
}

/// User identifier (matches loom-server-auth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(pub uuid::Uuid);

impl fmt::Display for UserId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<uuid::Uuid> for UserId {
	fn from(id: uuid::Uuid) -> Self {
		Self(id)
	}
}

/// Organization identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrgId(pub uuid::Uuid);

impl fmt::Display for OrgId {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<uuid::Uuid> for OrgId {
	fn from(id: uuid::Uuid) -> Self {
		Self(id)
	}
}

/// Visibility level for a clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClipVisibility {
	/// Only the owner can view.
	#[default]
	Private,
	/// Anyone in the org can view.
	Internal,
	/// Anyone with the link can view.
	Public,
}

impl fmt::Display for ClipVisibility {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Private => write!(f, "private"),
			Self::Internal => write!(f, "internal"),
			Self::Public => write!(f, "public"),
		}
	}
}

impl FromStr for ClipVisibility {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"private" => Ok(Self::Private),
			"internal" => Ok(Self::Internal),
			"public" => Ok(Self::Public),
			_ => Err(format!("invalid visibility: {}", s)),
		}
	}
}

/// A code clip (snippet) with optional description and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
	/// Unique identifier.
	pub id: ClipId,
	/// Owner username or organization name.
	pub owner: String,
	/// Clip name (unique per owner).
	pub name: String,
	/// Optional description.
	pub description: Option<String>,
	/// Visibility level.
	pub visibility: ClipVisibility,
	/// User who created the clip.
	pub created_by: UserId,
	/// Optional organization that owns the clip.
	pub org_id: Option<OrgId>,
	/// Whether this is a fork.
	pub is_fork: bool,
	/// Original clip ID if forked.
	pub forked_from: Option<ClipId>,
	/// Number of files in the clip.
	pub file_count: u32,
	/// Total size in bytes.
	pub size_bytes: u64,
	/// Primary language (detected from files).
	pub language: Option<String>,
	/// Creation timestamp.
	pub created_at: DateTime<Utc>,
	/// Last update timestamp.
	pub updated_at: DateTime<Utc>,
}

impl Clip {
	/// Create a new clip with default values.
	pub fn new(owner: String, name: String, created_by: UserId) -> Self {
		let now = Utc::now();
		Self {
			id: ClipId::new(),
			owner,
			name,
			description: None,
			visibility: ClipVisibility::default(),
			created_by,
			org_id: None,
			is_fork: false,
			forked_from: None,
			file_count: 0,
			size_bytes: 0,
			language: None,
			created_at: now,
			updated_at: now,
		}
	}
}

/// A file within a clip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipFile {
	/// File path within the clip.
	pub path: String,
	/// File content (may be redacted).
	pub content: String,
	/// Size in bytes.
	pub size_bytes: u64,
	/// Whether the content has been redacted.
	pub is_redacted: bool,
	/// Detected language/syntax.
	pub language: Option<String>,
}

/// A revision (commit) in a clip's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipRevision {
	/// Git commit SHA.
	pub sha: String,
	/// Author name.
	pub author_name: String,
	/// Author email.
	pub author_email: String,
	/// Commit timestamp (ISO 8601).
	pub timestamp: String,
	/// Commit message.
	pub message: String,
}
