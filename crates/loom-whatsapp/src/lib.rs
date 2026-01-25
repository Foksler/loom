// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! WhatsApp Business Cloud API client and types for Loom.
//!
//! This crate provides:
//! - Types for webhook payloads (inbound messages, statuses)
//! - Types for sending messages
//! - Configuration with secret management
//! - Webhook signature verification
//! - HTTP client for WhatsApp Cloud API

pub mod client;
pub mod config;
pub mod error;
pub mod types;
pub mod webhook;

pub use client::WhatsAppClient;
pub use config::WhatsAppConfig;
pub use error::WhatsAppError;
pub use types::*;
pub use webhook::verify_webhook_signature;
