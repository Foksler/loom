// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! MCP session state management.
//!
//! Sessions are optional per the MCP spec - stateless operation is fully supported.
//! This module provides session tracking for clients that send the Mcp-Session-Id header.

use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
	time::{Duration, Instant},
};
use uuid::Uuid;

/// Session timeout duration (1 hour).
const SESSION_TIMEOUT: Duration = Duration::from_secs(3600);

/// Maximum number of concurrent sessions.
const MAX_SESSIONS: usize = 10000;

/// MCP session state.
#[derive(Debug, Clone)]
pub struct McpSession {
	pub id: String,
	pub user_id: String,
	pub initialized: bool,
	pub created_at: Instant,
	pub last_accessed: Instant,
	pub protocol_version: Option<String>,
	pub client_name: Option<String>,
	pub client_version: Option<String>,
}

impl McpSession {
	/// Create a new session.
	pub fn new(user_id: String) -> Self {
		let now = Instant::now();
		Self {
			id: Uuid::new_v4().to_string(),
			user_id,
			initialized: false,
			created_at: now,
			last_accessed: now,
			protocol_version: None,
			client_name: None,
			client_version: None,
		}
	}

	/// Check if the session has expired.
	pub fn is_expired(&self) -> bool {
		self.last_accessed.elapsed() > SESSION_TIMEOUT
	}

	/// Touch the session to update last accessed time.
	pub fn touch(&mut self) {
		self.last_accessed = Instant::now();
	}

	/// Mark the session as initialized.
	pub fn mark_initialized(
		&mut self,
		protocol_version: String,
		client_name: String,
		client_version: Option<String>,
	) {
		self.initialized = true;
		self.protocol_version = Some(protocol_version);
		self.client_name = Some(client_name);
		self.client_version = client_version;
		self.touch();
	}
}

/// Thread-safe session store.
#[derive(Debug, Default)]
pub struct McpSessionStore {
	sessions: RwLock<HashMap<String, McpSession>>,
}

impl McpSessionStore {
	/// Create a new session store.
	pub fn new() -> Self {
		Self {
			sessions: RwLock::new(HashMap::new()),
		}
	}

	/// Create a new session for a user.
	pub fn create_session(&self, user_id: String) -> McpSession {
		let session = McpSession::new(user_id);
		let session_id = session.id.clone();

		let mut sessions = self.sessions.write().expect("RwLock poisoned");

		// Clean up expired sessions if we're at capacity
		if sessions.len() >= MAX_SESSIONS {
			sessions.retain(|_, s| !s.is_expired());
		}

		sessions.insert(session_id, session.clone());
		session
	}

	/// Get a session by ID, returning None if not found or expired.
	pub fn get_session(&self, session_id: &str) -> Option<McpSession> {
		let sessions = self.sessions.read().expect("RwLock poisoned");
		sessions.get(session_id).and_then(|s| {
			if s.is_expired() {
				None
			} else {
				Some(s.clone())
			}
		})
	}

	/// Update a session.
	pub fn update_session(&self, session: McpSession) {
		let mut sessions = self.sessions.write().expect("RwLock poisoned");
		sessions.insert(session.id.clone(), session);
	}

	/// Touch a session to update its last accessed time.
	pub fn touch_session(&self, session_id: &str) -> bool {
		let mut sessions = self.sessions.write().expect("RwLock poisoned");
		if let Some(session) = sessions.get_mut(session_id) {
			if !session.is_expired() {
				session.touch();
				return true;
			}
		}
		false
	}

	/// Remove a session.
	pub fn remove_session(&self, session_id: &str) -> Option<McpSession> {
		let mut sessions = self.sessions.write().expect("RwLock poisoned");
		sessions.remove(session_id)
	}

	/// Get the number of active sessions.
	pub fn session_count(&self) -> usize {
		let sessions = self.sessions.read().expect("RwLock poisoned");
		sessions.values().filter(|s| !s.is_expired()).count()
	}

	/// Clean up expired sessions.
	pub fn cleanup_expired(&self) -> usize {
		let mut sessions = self.sessions.write().expect("RwLock poisoned");
		let before = sessions.len();
		sessions.retain(|_, s| !s.is_expired());
		before - sessions.len()
	}
}

/// Create a shared session store.
pub fn create_session_store() -> Arc<McpSessionStore> {
	Arc::new(McpSessionStore::new())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_session_creation() {
		let session = McpSession::new("user-123".into());
		assert!(!session.initialized);
		assert!(!session.is_expired());
		assert_eq!(session.user_id, "user-123");
	}

	#[test]
	fn test_session_mark_initialized() {
		let mut session = McpSession::new("user-123".into());
		session.mark_initialized(
			"2025-11-25".into(),
			"test-client".into(),
			Some("1.0.0".into()),
		);
		assert!(session.initialized);
		assert_eq!(session.protocol_version, Some("2025-11-25".into()));
		assert_eq!(session.client_name, Some("test-client".into()));
	}

	#[test]
	fn test_session_store_create_and_get() {
		let store = McpSessionStore::new();
		let session = store.create_session("user-123".into());

		let retrieved = store.get_session(&session.id);
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap().user_id, "user-123");
	}

	#[test]
	fn test_session_store_touch() {
		let store = McpSessionStore::new();
		let session = store.create_session("user-123".into());

		assert!(store.touch_session(&session.id));
		assert!(!store.touch_session("nonexistent"));
	}

	#[test]
	fn test_session_store_remove() {
		let store = McpSessionStore::new();
		let session = store.create_session("user-123".into());

		let removed = store.remove_session(&session.id);
		assert!(removed.is_some());

		let retrieved = store.get_session(&session.id);
		assert!(retrieved.is_none());
	}

	#[test]
	fn test_session_count() {
		let store = McpSessionStore::new();
		assert_eq!(store.session_count(), 0);

		store.create_session("user-1".into());
		store.create_session("user-2".into());
		assert_eq!(store.session_count(), 2);
	}
}
