//! Integration tests for loom-server.
//!
//! Phase 2 comprehensive test suite covering:
//! - Query detection from LLM output
//! - Query handler flows and timeouts
//! - Query manager concurrency and state
//! - Security validation and hardening
//! - End-to-end LLM → Query → Response flows

mod end_to_end_tests;
mod query_detection_tests;
mod query_handler_tests;
mod query_integration_test;
mod query_manager_integration_tests;
mod query_metrics_integration_tests;
mod query_security_tests;
mod query_tracing_tests;
mod tracing_integration_tests;
