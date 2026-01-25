// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! WhatsApp server integration for Loom.
//!
//! This crate provides:
//! - SQLite repository for WhatsApp entities
//! - Message processing service
//! - Conversation tracking with 24-hour windows
//! - OTP management for phone linking

pub mod conversation;
pub mod repository;
pub mod service;

pub use conversation::ConversationTracker;
pub use repository::WhatsAppRepository;
pub use service::WhatsAppService;

/// Verify a plain text token against an Argon2id hash.
pub fn verify_token(token: &str, hash: &str) -> bool {
	use argon2::{Argon2, PasswordHash, PasswordVerifier};
	let parsed_hash = match PasswordHash::new(hash) {
		Ok(h) => h,
		Err(_) => return false,
	};
	Argon2::default()
		.verify_password(token.as_bytes(), &parsed_hash)
		.is_ok()
}
