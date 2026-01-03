// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::path::PathBuf;

use thiserror::Error;

use crate::types::StitchId;

#[derive(Debug, Error)]
pub enum SpoolError {
	#[error("not a spool repository")]
	NotASpoolRepo,

	#[error("stitch not found: {0:?}")]
	StitchNotFound(StitchId),

	#[error("tangled path: {path}")]
	Tangled { path: PathBuf },

	#[error("pin already exists: {0}")]
	PinExists(String),

	#[error("nothing to unpick")]
	NothingToUnpick,

	#[error("git error: {0}")]
	Git(#[from] Box<gix::open::Error>),

	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SpoolError>;
