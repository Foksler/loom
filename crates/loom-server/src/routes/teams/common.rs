// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for team management.

pub use loom_server_api::teams::*;

use crate::impl_api_error_response;

impl_api_error_response!(TeamErrorResponse);
