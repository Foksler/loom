// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Job scheduling library for Loom applications.
//!
//! This crate provides a simple job scheduling abstraction that can be
//! integrated with monitoring systems like loom-crons.

pub mod error;
pub mod runner;

pub use error::{JobError, Result};
pub use runner::{Job, JobRunner};
