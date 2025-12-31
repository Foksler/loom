// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Integration tests for loom-server.
//!
//! Comprehensive test suite covering:
//! - Query detection from LLM output
//! - Query handler flows and timeouts
//! - Query manager concurrency and state
//! - Security validation and hardening
//! - End-to-end LLM → Query → Response flows

pub mod support;

mod auth_integration_tests;
mod authz_threads_tests;
mod authz_orgs_tests;
mod authz_admin_tests;
mod authz_users_tests;
mod end_to_end_tests;
mod query_detection_tests;
mod query_handler_tests;
mod query_integration_test;
mod query_manager_integration_tests;
mod query_metrics_integration_tests;
mod query_security_tests;
mod query_tracing_tests;
mod share_tests;
mod tracing_integration_tests;
