// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Error types for the clips system.

use thiserror::Error;

/// Result type alias for clips operations.
pub type Result<T> = std::result::Result<T, ClipsError>;

/// Errors that can occur in clips operations.
#[derive(Debug, Error)]
pub enum ClipsError {
	/// Clip not found.
	#[error("clip not found: {0}")]
	NotFound(String),

	/// Clip already exists with this name for this owner.
	#[error("clip already exists: {owner}/{name}")]
	AlreadyExists { owner: String, name: String },

	/// Invalid clip name.
	#[error("invalid clip name: {0}")]
	InvalidName(String),

	/// Database error.
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),

	/// Git operation error.
	#[error("git error: {0}")]
	Git(String),

	/// IO error.
	#[error("io error: {0}")]
	Io(#[from] std::io::Error),

	/// JSON serialization/deserialization error.
	#[error("json error: {0}")]
	Json(#[from] serde_json::Error),

	/// UUID parsing error.
	#[error("invalid UUID: {0}")]
	InvalidUuid(#[from] uuid::Error),

	/// Invalid datetime format.
	#[error("invalid datetime: {0}")]
	InvalidDateTime(String),

	/// Parse error.
	#[error("parse error: {0}")]
	Parse(String),

	/// Permission denied.
	#[error("permission denied")]
	PermissionDenied,
}
