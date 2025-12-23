// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

mod config;
mod error;
mod generator;
mod git;
mod service;

pub use config::AutoCommitConfig;
pub use error::AutoCommitError;
pub use generator::CommitMessageGenerator;
pub use git::{GitClient, GitDiff};
pub use service::{AutoCommitResult, AutoCommitService, CompletedToolInfo};
