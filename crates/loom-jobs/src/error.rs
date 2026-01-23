// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use thiserror::Error;

/// Errors that can occur during job execution.
#[derive(Debug, Error)]
pub enum JobError {
	#[error("Job failed: {0}")]
	Failed(String),

	#[error("Job was cancelled")]
	Cancelled,

	#[error("Job execution error: {0}")]
	Execution(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Result type for job operations.
pub type Result<T> = std::result::Result<T, JobError>;
