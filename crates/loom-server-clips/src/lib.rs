// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Code clips/snippets management for Loom.
//!
//! This crate provides the server-side implementation for the clips system,
//! including:
//!
//! - Repository layer for database operations
//! - Git bare repository management for clip storage
//! - Secret redaction when reading file content

pub mod error;
pub mod git;
pub mod store;
pub mod types;

pub use error::{ClipsError, Result};
pub use git::ClipsGitStore;
pub use store::{ClipsRepository, ClipsStore, SqliteClipsRepository};
pub use types::{Clip, ClipFile, ClipId, ClipRevision, ClipVisibility, OrgId, UserId};
