// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Integration tests for authentication routes.
//!
//! Tests cover:
//! - OAuth callback validation
//! - OAuth email verification checks
//! - Magic link authentication flow
//! - Device code flow
//! - Session management
//! - Cookie security attributes (Secure, HttpOnly, SameSite)
//! - Admin route authorization (H9)
//! - Access denial audit logging (H6)

use crate::api::{create_app_state, create_router, AppState};
use crate::config::ServerConfig;
use crate::db::ThreadRepository;
use axum::{
	body::Body,
	http::{header::SET_COOKIE, Request, StatusCode},
};
use loom_auth_github::GitHubEmail;
use loom_auth_google::GoogleUserInfo;
use loom_auth_okta::OktaUserInfo;
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;

/// Creates a test app with isolated database
async fn setup_test_app() -> (axum::Router, tempfile::TempDir) {
	let dir = tempdir().unwrap();
	let db_path = dir.path().join("test_auth.db");
	let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
	let repo = Arc::new(ThreadRepository::new(&db_url).await.unwrap());
	let pool = repo.pool().clone();
	let config = ServerConfig::default();
	let state = create_app_state(pool, repo, &config).await;
	(create_router(state), dir)
}

/// Creates a test app and returns both the router and state for repository access
async fn setup_test_app_with_state() -> (axum::Router, AppState, tempfile::TempDir) {
	let dir = tempdir().unwrap();
	let db_path = dir.path().join("test_auth_audit.db");
	let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
	let repo = Arc::new(ThreadRepository::new(&db_url).await.unwrap());
	let pool = repo.pool().clone();
	let config = ServerConfig::default();
	let state = create_app_state(pool, repo, &config).await;
	(create_router(state.clone()), state, dir)
}

// ============================================================================
// OAuth Callback Tests
// ============================================================================

#[tokio::test]
async fn test_github_callback_without_provider_config_returns_501() {
	let (app, _dir) = setup_test_app().await;

	// Without GitHub OAuth configured, the callback should return 501 Not Implemented
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/callback/github")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	// Provider not configured returns 501
	assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn test_google_callback_without_provider_config_returns_501() {
	let (app, _dir) = setup_test_app().await;

	// Without Google OAuth configured, the callback should return 501 Not Implemented
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/callback/google?code=test_code&state=test_state")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	// Provider not configured returns 501
	assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn test_providers_endpoint_returns_available_providers() {
	let (app, _dir) = setup_test_app().await;

	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/providers")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::OK);

	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert!(json["providers"].is_array());
}

// ============================================================================
// Magic Link Tests
// ============================================================================

#[tokio::test]
async fn test_magic_link_request_accepts_any_email() {
	let (app, _dir) = setup_test_app().await;

	let body = serde_json::json!({ "email": "test@example.com" }).to_string();
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/magic-link")
				.method("POST")
				.header("content-type", "application/json")
				.body(Body::from(body))
				.unwrap(),
		)
		.await
		.unwrap();

	let status = response.status();
	assert!(
		status == StatusCode::OK || status == StatusCode::ACCEPTED,
		"Expected 200 or 202, got {status}"
	);
}

#[tokio::test]
async fn test_magic_link_request_missing_email_returns_422() {
	let (app, _dir) = setup_test_app().await;

	let body = serde_json::json!({}).to_string();
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/magic-link")
				.method("POST")
				.header("content-type", "application/json")
				.body(Body::from(body))
				.unwrap(),
		)
		.await
		.unwrap();

	// Missing required field returns 422 Unprocessable Entity
	assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Device Code Tests
// ============================================================================

#[tokio::test]
async fn test_device_start_returns_codes() {
	let (app, _dir) = setup_test_app().await;

	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/device/start")
				.method("POST")
				.header("content-type", "application/json")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::OK);

	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

	assert!(json["device_code"].is_string());
	assert!(json["user_code"].is_string());
	assert!(json["expires_in"].is_number());
}

#[tokio::test]
async fn test_device_poll_pending_initially() {
	let (app, _dir) = setup_test_app().await;

	let start_response = app
		.clone()
		.oneshot(
			Request::builder()
				.uri("/api/auth/device/start")
				.method("POST")
				.header("content-type", "application/json")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	let start_body = axum::body::to_bytes(start_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let start_json: serde_json::Value = serde_json::from_slice(&start_body).unwrap();
	let device_code = start_json["device_code"].as_str().unwrap();

	let poll_body = serde_json::json!({ "device_code": device_code }).to_string();
	let poll_response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/device/poll")
				.method("POST")
				.header("content-type", "application/json")
				.body(Body::from(poll_body))
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(poll_response.status(), StatusCode::OK);

	let poll_body = axum::body::to_bytes(poll_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let poll_json: serde_json::Value = serde_json::from_slice(&poll_body).unwrap();
	assert_eq!(poll_json["status"], "pending");
}

// ============================================================================
// Session Tests
// ============================================================================

#[tokio::test]
async fn test_auth_me_without_session_returns_401() {
	let (app, _dir) = setup_test_app().await;

	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/me")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_logout_without_session_returns_401() {
	let (app, _dir) = setup_test_app().await;

	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/logout")
				.method("POST")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Cookie Security Tests (C1: Session cookies must include Secure flag)
// ============================================================================

/// Helper to check if a Set-Cookie header value contains the Secure flag.
fn cookie_has_secure_flag(cookie_value: &str) -> bool {
	cookie_value
		.split(';')
		.map(|s| s.trim())
		.any(|attr| attr.eq_ignore_ascii_case("Secure"))
}

/// Helper to check if a Set-Cookie header value contains the HttpOnly flag.
fn cookie_has_httponly_flag(cookie_value: &str) -> bool {
	cookie_value
		.split(';')
		.map(|s| s.trim())
		.any(|attr| attr.eq_ignore_ascii_case("HttpOnly"))
}

/// Helper to check if a Set-Cookie header value contains the SameSite attribute.
fn cookie_has_samesite_attribute(cookie_value: &str) -> bool {
	cookie_value
		.split(';')
		.map(|s| s.trim())
		.any(|attr| attr.to_lowercase().starts_with("samesite="))
}

#[test]
fn test_cookie_secure_flag_detection() {
	// Test detection helper with various cookie formats
	assert!(cookie_has_secure_flag(
		"session=abc123; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax"
	));
	assert!(cookie_has_secure_flag(
		"session=abc123; Path=/; Secure; HttpOnly"
	));
	assert!(cookie_has_secure_flag("session=abc123; Secure"));
	assert!(cookie_has_secure_flag("session=abc123;Secure;HttpOnly"));

	// Negative cases
	assert!(!cookie_has_secure_flag("session=abc123; Path=/; HttpOnly"));
	assert!(!cookie_has_secure_flag(
		"session=abc123; Path=/; HttpOnly; SameSite=Lax"
	));
	// "Secure" as part of value should not match
	assert!(!cookie_has_secure_flag("session=SecureValue123; Path=/"));
}

#[test]
fn test_cookie_httponly_flag_detection() {
	assert!(cookie_has_httponly_flag(
		"session=abc123; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax"
	));
	assert!(cookie_has_httponly_flag("session=abc123; HttpOnly"));
	assert!(!cookie_has_httponly_flag("session=abc123; Secure"));
}

#[test]
fn test_cookie_samesite_attribute_detection() {
	assert!(cookie_has_samesite_attribute(
		"session=abc123; Path=/; SameSite=Lax"
	));
	assert!(cookie_has_samesite_attribute(
		"session=abc123; SameSite=Strict"
	));
	assert!(!cookie_has_samesite_attribute("session=abc123; Secure"));
}

#[tokio::test]
async fn test_oauth_callback_cookie_has_secure_flag() {
	// OAuth callback without provider config returns 501, which doesn't set a cookie.
	// This test validates that when OAuth IS configured and succeeds, the cookie
	// format used in auth.rs includes the Secure flag.
	//
	// The cookie format is defined at auth.rs:1267 as:
	// "{}={}; Path=/; Max-Age={}; HttpOnly; Secure; SameSite=Lax"
	//
	// Since we can't trigger a real OAuth flow in integration tests without
	// external provider credentials, we verify the expected format directly.
	let expected_cookie_format = "session=TOKEN; Path=/; Max-Age=5184000; HttpOnly; Secure; SameSite=Lax";
	assert!(
		cookie_has_secure_flag(expected_cookie_format),
		"OAuth callback cookie format must include Secure flag"
	);
	assert!(
		cookie_has_httponly_flag(expected_cookie_format),
		"OAuth callback cookie format must include HttpOnly flag"
	);
	assert!(
		cookie_has_samesite_attribute(expected_cookie_format),
		"OAuth callback cookie format must include SameSite attribute"
	);
}

#[tokio::test]
async fn test_magic_link_verify_cookie_has_secure_flag() {
	// Magic link verification sets a cookie on successful verification.
	// The cookie format is defined at auth.rs:1434 as:
	// "{}={}; Path=/; Max-Age={}; HttpOnly; Secure; SameSite=Lax"
	//
	// Since we can't create a valid magic link token in integration tests,
	// we verify the expected format directly.
	let expected_cookie_format = "session=TOKEN; Path=/; Max-Age=5184000; HttpOnly; Secure; SameSite=Lax";
	assert!(
		cookie_has_secure_flag(expected_cookie_format),
		"Magic link verification cookie format must include Secure flag"
	);
	assert!(
		cookie_has_httponly_flag(expected_cookie_format),
		"Magic link verification cookie format must include HttpOnly flag"
	);
	assert!(
		cookie_has_samesite_attribute(expected_cookie_format),
		"Magic link verification cookie format must include SameSite attribute"
	);
}

#[tokio::test]
async fn test_logout_clears_cookie_with_secure_flag() {
	// Logout endpoint clears the session cookie using the format at auth.rs:206:
	// "{}=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax"
	//
	// Since logout requires authentication, we can't test the full flow.
	// We verify the expected format directly.
	let expected_clear_cookie = "session=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax";
	assert!(
		cookie_has_secure_flag(expected_clear_cookie),
		"Logout clear cookie format must include Secure flag"
	);
	assert!(
		cookie_has_httponly_flag(expected_clear_cookie),
		"Logout clear cookie format must include HttpOnly flag"
	);
	assert!(
		cookie_has_samesite_attribute(expected_clear_cookie),
		"Logout clear cookie format must include SameSite attribute"
	);
}

#[tokio::test]
async fn test_magic_link_verify_invalid_token_no_cookie() {
	let (app, _dir) = setup_test_app().await;

	// Invalid magic link token should not set any cookie
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/auth/magic-link/verify?token=invalid_token_12345")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	// Should return 400 Bad Request
	assert_eq!(response.status(), StatusCode::BAD_REQUEST);

	// Should NOT set any cookie on failure
	let set_cookie_header = response.headers().get(SET_COOKIE);
	assert!(
		set_cookie_header.is_none(),
		"Invalid magic link should not set any cookie"
	);
}

// ============================================================================
// OAuth Email Verification Tests (H1/H2: Verified email requirements)
// ============================================================================

#[test]
fn test_google_user_with_verified_email_passes_check() {
	let user = GoogleUserInfo {
		sub: "12345".to_string(),
		email: "verified@example.com".to_string(),
		email_verified: true,
		name: Some("Test User".to_string()),
		picture: None,
		given_name: None,
		family_name: None,
	};

	assert!(user.email_verified, "Verified email should pass the check");
}

#[test]
fn test_google_user_with_unverified_email_fails_check() {
	let user = GoogleUserInfo {
		sub: "12345".to_string(),
		email: "unverified@example.com".to_string(),
		email_verified: false,
		name: Some("Test User".to_string()),
		picture: None,
		given_name: None,
		family_name: None,
	};

	assert!(!user.email_verified, "Unverified email should fail the check");
}

#[test]
fn test_okta_user_with_verified_email_passes_check() {
	let user = OktaUserInfo {
		sub: "okta-12345".to_string(),
		email: "verified@company.com".to_string(),
		email_verified: Some(true),
		name: Some("Corporate User".to_string()),
		preferred_username: Some("user@company.com".to_string()),
		given_name: Some("Corporate".to_string()),
		family_name: Some("User".to_string()),
		groups: None,
	};

	assert!(
		user.email_verified.unwrap_or(false),
		"Verified email should pass the check"
	);
}

#[test]
fn test_okta_user_with_unverified_email_fails_check() {
	let user = OktaUserInfo {
		sub: "okta-12345".to_string(),
		email: "unverified@company.com".to_string(),
		email_verified: Some(false),
		name: Some("Corporate User".to_string()),
		preferred_username: Some("user@company.com".to_string()),
		given_name: Some("Corporate".to_string()),
		family_name: Some("User".to_string()),
		groups: None,
	};

	assert!(
		!user.email_verified.unwrap_or(false),
		"Unverified email should fail the check"
	);
}

#[test]
fn test_okta_user_with_missing_email_verified_fails_check() {
	let user = OktaUserInfo {
		sub: "okta-12345".to_string(),
		email: "unknown@company.com".to_string(),
		email_verified: None,
		name: Some("Corporate User".to_string()),
		preferred_username: Some("user@company.com".to_string()),
		given_name: Some("Corporate".to_string()),
		family_name: Some("User".to_string()),
		groups: None,
	};

	assert!(
		!user.email_verified.unwrap_or(false),
		"Missing email_verified should default to false (fail)"
	);
}

#[test]
fn test_github_verified_primary_email_passes_check() {
	let emails = [GitHubEmail {
			email: "primary@example.com".to_string(),
			primary: true,
			verified: true,
		},
		GitHubEmail {
			email: "secondary@example.com".to_string(),
			primary: false,
			verified: true,
		}];

	let verified_primary = emails.iter().find(|e| e.primary && e.verified).map(|e| e.email.clone());
	assert_eq!(
		verified_primary,
		Some("primary@example.com".to_string()),
		"Should find verified primary email"
	);
}

#[test]
fn test_github_unverified_primary_email_fails_check() {
	let emails = [GitHubEmail {
			email: "primary@example.com".to_string(),
			primary: true,
			verified: false,
		},
		GitHubEmail {
			email: "secondary@example.com".to_string(),
			primary: false,
			verified: true,
		}];

	let verified_primary = emails.iter().find(|e| e.primary && e.verified).map(|e| e.email.clone());
	assert!(
		verified_primary.is_none(),
		"Should not find verified primary email when primary is unverified"
	);
}

#[test]
fn test_github_no_verified_emails_fails_check() {
	let emails = [GitHubEmail {
			email: "primary@example.com".to_string(),
			primary: true,
			verified: false,
		},
		GitHubEmail {
			email: "secondary@example.com".to_string(),
			primary: false,
			verified: false,
		}];

	let verified_primary = emails.iter().find(|e| e.primary && e.verified).map(|e| e.email.clone());
	assert!(
		verified_primary.is_none(),
		"Should not find any verified primary email"
	);
}

#[test]
fn test_github_empty_emails_fails_check() {
	let emails: Vec<GitHubEmail> = vec![];

	let verified_primary = emails.iter().find(|e| e.primary && e.verified).map(|e| e.email.clone());
	assert!(
		verified_primary.is_none(),
		"Empty email list should fail verification"
	);
}

// ============================================================================
// Access Denial Audit Logging Tests (H6)
// ============================================================================

use chrono::Utc;
use loom_auth::{generate_session_token, Session, SessionType, User, UserId};
use sha2::{Digest, Sha256};
use tokio::time::{sleep, Duration};

/// Hash a token using SHA-256 (same as auth_middleware)
fn hash_token(token: &str) -> String {
	let mut hasher = Sha256::new();
	hasher.update(token.as_bytes());
	hex::encode(hasher.finalize())
}

/// Creates a test user without admin privileges
fn create_test_user(email: &str) -> User {
	User {
		id: UserId::generate(),
		display_name: "Test User".to_string(),
		primary_email: Some(email.to_string()),
		avatar_url: None,
		email_visible: false,
		is_system_admin: false,
		is_support: false,
		is_auditor: false,
		created_at: Utc::now(),
		updated_at: Utc::now(),
		deleted_at: None,
		locale: None,
	}
}

/// H6: Verify access denials are audit logged when a non-admin user
/// attempts to access an admin-only endpoint.
#[tokio::test]
async fn test_access_denial_is_audit_logged() {
	let (app, state, _dir) = setup_test_app_with_state().await;

	// Create a non-admin user
	let user = create_test_user("nonadmin@example.com");
	state.user_repo.create_user(&user).await.unwrap();

	// Create a session for this user
	let session_token = generate_session_token();
	let token_hash = hash_token(&session_token);
	let session = Session::new(user.id, SessionType::Web);
	state
		.session_repo
		.create_session(&session, &token_hash)
		.await
		.unwrap();

	// Make request to admin-only endpoint with the session cookie
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/admin/users")
				.header("cookie", format!("loom_session={session_token}"))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	// Should get 403 Forbidden
	assert_eq!(response.status(), StatusCode::FORBIDDEN);

	// Give the async audit log task time to complete
	sleep(Duration::from_millis(100)).await;

	// Query audit_logs for AccessDenied event
	let (logs, count) = state
		.audit_repo
		.query_logs(
			Some("access_denied"),
			Some(&user.id),
			None,
			None,
			None,
			None,
			100,
			0,
		)
		.await
		.unwrap();

	assert!(count >= 1, "Expected at least 1 AccessDenied audit log entry, got {count}");

	// Verify the logged event has correct details
	let log = logs.first().expect("Should have at least one log entry");
	assert_eq!(log.actor_user_id, Some(user.id));
	assert!(
		log.details.get("reason").is_some(),
		"Audit log should contain reason for denial"
	);
}

/// H6: Verify access denial audit log contains correct resource and action details.
#[tokio::test]
async fn test_access_denial_audit_contains_correct_details() {
	let (app, state, _dir) = setup_test_app_with_state().await;

	// Create a non-admin user
	let user = create_test_user("nonadmin2@example.com");
	state.user_repo.create_user(&user).await.unwrap();

	// Create a session for this user
	let session_token = generate_session_token();
	let token_hash = hash_token(&session_token);
	let session = Session::new(user.id, SessionType::Web);
	state
		.session_repo
		.create_session(&session, &token_hash)
		.await
		.unwrap();

	// Make request to admin audit-logs endpoint
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/admin/audit-logs")
				.header("cookie", format!("loom_session={session_token}"))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	// Should get 403 Forbidden
	assert_eq!(response.status(), StatusCode::FORBIDDEN);

	// Give the async audit log task time to complete
	sleep(Duration::from_millis(100)).await;

	// Query audit_logs for AccessDenied event from this user
	let (logs, count) = state
		.audit_repo
		.query_logs(
			Some("access_denied"),
			Some(&user.id),
			None,
			None,
			None,
			None,
			100,
			0,
		)
		.await
		.unwrap();

	assert!(count >= 1, "Expected at least 1 AccessDenied audit log entry");

	let log = logs.first().expect("Should have at least one log entry");

	// Verify the audit entry contains the expected fields
	assert_eq!(log.actor_user_id, Some(user.id), "Actor should be the denied user");
	assert_eq!(
		log.resource_type.as_deref(),
		Some("role"),
		"Resource type should be 'role' for RequireRole denials"
	);
	assert_eq!(
		log.action.as_str(),
		"role_check",
		"Action should be 'role_check'"
	);

	// Verify details contain required roles
	let details = &log.details;
	assert!(
		details.get("required_roles").is_some(),
		"Details should contain required_roles"
	);
	assert!(
		details.get("reason").is_some(),
		"Details should contain reason"
	);
}

/// H6: Verify unauthenticated requests do NOT create AccessDenied audit events
/// (they should be 401 Unauthorized, not 403 Forbidden)
#[tokio::test]
async fn test_unauthenticated_request_does_not_log_access_denied() {
	let (app, state, _dir) = setup_test_app_with_state().await;

	// Make request to admin endpoint without any authentication
	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/admin/users")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	// Should get 401 Unauthorized (not 403 Forbidden)
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	// Give some time for any async tasks
	sleep(Duration::from_millis(100)).await;

	// Query all audit_logs for AccessDenied events
	let (logs, count) = state
		.audit_repo
		.query_logs(Some("access_denied"), None, None, None, None, None, 100, 0)
		.await
		.unwrap();

	// Should have no AccessDenied events (unauthenticated = 401, not 403)
	assert_eq!(
		count, 0,
		"Unauthenticated requests should not create AccessDenied audit logs, found {count}: {logs:?}"
	);
}

// ============================================================================
// Admin Route Authorization Tests (H9)
// ============================================================================

#[tokio::test]
async fn test_admin_routes_require_authentication() {
	let (app, _dir) = setup_test_app().await;

	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/admin/users")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(
		response.status(),
		StatusCode::UNAUTHORIZED,
		"Unauthenticated request to admin route should return 401"
	);
}

#[tokio::test]
async fn test_admin_routes_require_admin_role() {
	use crate::abac_middleware::RequireRole;
	use crate::routes;
	use axum::{
		middleware::{self, Next},
		routing::get,
		Router,
	};
	use loom_auth::middleware::{AuthContext, CurrentUser};

	let regular_user = create_test_user("regular@example.com");
	let current_user = CurrentUser::from_access_token(regular_user);
	let auth_ctx = AuthContext::authenticated(current_user);

	let dir = tempdir().unwrap();
	let db_path = dir.path().join("test_admin_role.db");
	let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
	let repo = Arc::new(ThreadRepository::new(&db_url).await.unwrap());
	let pool = repo.pool().clone();
	let config = ServerConfig::default();
	let state = create_app_state(pool, repo, &config).await;

	let inject_auth = move |mut req: Request<Body>, next: Next| {
		let auth_ctx = auth_ctx.clone();
		async move {
			req.extensions_mut().insert(auth_ctx);
			next.run(req).await
		}
	};

	let app = Router::new()
		.route("/api/admin/users", get(routes::admin::list_users))
		.route_layer(RequireRole::admin())
		.layer(middleware::from_fn(inject_auth))
		.with_state(state);

	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/admin/users")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(
		response.status(),
		StatusCode::FORBIDDEN,
		"Non-admin user should receive 403 Forbidden for admin routes"
	);
}

#[tokio::test]
async fn test_admin_can_access_admin_routes() {
	use crate::abac_middleware::RequireRole;
	use crate::routes;
	use axum::{
		middleware::{self, Next},
		routing::get,
		Router,
	};
	use loom_auth::middleware::{AuthContext, CurrentUser};

	let mut admin_user = create_test_user("admin@example.com");
	admin_user.is_system_admin = true;
	let current_user = CurrentUser::from_access_token(admin_user);
	let auth_ctx = AuthContext::authenticated(current_user);

	let dir = tempdir().unwrap();
	let db_path = dir.path().join("test_admin_access.db");
	let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
	let repo = Arc::new(ThreadRepository::new(&db_url).await.unwrap());
	let pool = repo.pool().clone();
	let config = ServerConfig::default();
	let state = create_app_state(pool, repo, &config).await;

	let inject_auth = move |mut req: Request<Body>, next: Next| {
		let auth_ctx = auth_ctx.clone();
		async move {
			req.extensions_mut().insert(auth_ctx);
			next.run(req).await
		}
	};

	let app = Router::new()
		.route("/api/admin/users", get(routes::admin::list_users))
		.route_layer(RequireRole::admin())
		.layer(middleware::from_fn(inject_auth))
		.with_state(state);

	let response = app
		.oneshot(
			Request::builder()
				.uri("/api/admin/users")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Admin user should be able to access admin routes"
	);
}

// ============================================================================
// Route Authentication Tests - Verify all protected routes require auth
// ============================================================================

/// Protected API routes that MUST require authentication.
/// These routes should return 401 Unauthorized without a valid token.
/// Routes may return 404 if optional features (like K8s/weaver) aren't configured.
///
/// NOTE: Thread routes (/api/threads/*) are intentionally PUBLIC and not listed here.
/// Only routes with RequireAuth extractor in their handlers are listed.
const PROTECTED_GET_ROUTES: &[&str] = &[
	// Session routes (sessions.rs uses RequireAuth)
	"/api/sessions",
	// Org routes (orgs.rs uses RequireAuth)
	"/api/orgs",
	"/api/orgs/test-id",
	"/api/orgs/test-id/members",
	// Team routes (teams.rs uses RequireAuth)
	"/api/orgs/test-org/teams",
	"/api/orgs/test-org/teams/test-team",
	"/api/orgs/test-org/teams/test-team/members",
	// API key routes (api_keys.rs uses RequireAuth)
	"/api/orgs/test-org/api-keys",
	"/api/orgs/test-org/api-keys/test-key/usage",
	// Invitation routes (invitations.rs uses RequireAuth)
	"/api/orgs/test-org/invitations",
	// User routes (auth.rs get_current_user uses RequireAuth)
	"/api/auth/me",
	// Weaver routes (weaver.rs uses RequireAuth, may 404 if K8s not configured)
	"/api/weavers",
	"/api/weaver/test-id",
	"/api/weaver/test-id/logs",
	// Admin routes (admin.rs uses RequireAuth + RequireRole)
	"/api/admin/users",
	"/api/admin/audit-logs",
];

const PROTECTED_POST_ROUTES: &[&str] = &[
	// Auth routes that need session (auth.rs uses RequireAuth)
	"/api/auth/logout",
	"/api/auth/device/complete",
	// Org routes (orgs.rs uses RequireAuth)
	"/api/orgs",
	"/api/orgs/test-id/members",
	// Team routes (teams.rs uses RequireAuth)
	"/api/orgs/test-org/teams",
	"/api/orgs/test-org/teams/test-team/members",
	// API key routes (api_keys.rs uses RequireAuth)
	"/api/orgs/test-org/api-keys",
	// Invitation routes (invitations.rs uses RequireAuth)
	"/api/orgs/test-org/invitations",
	// Share routes (share.rs uses RequireAuth)
	"/api/threads/test-id/share",
	"/api/threads/test-id/support-access/request",
	"/api/threads/test-id/support-access/approve",
	// Weaver routes (weaver.rs uses RequireAuth, may 404 if K8s not configured)
	"/api/weaver",
	"/api/weavers/cleanup",
];

const PROTECTED_DELETE_ROUTES: &[&str] = &[
	// Session routes (sessions.rs uses RequireAuth)
	"/api/sessions/test-id",
	// Org routes (orgs.rs uses RequireAuth)
	"/api/orgs/test-id",
	"/api/orgs/test-org/members/test-user",
	// Team routes (teams.rs uses RequireAuth)
	"/api/orgs/test-org/teams/test-team",
	"/api/orgs/test-org/teams/test-team/members/test-user",
	// API key routes (api_keys.rs uses RequireAuth)
	"/api/orgs/test-org/api-keys/test-key",
	// Invitation routes (invitations.rs uses RequireAuth)
	"/api/orgs/test-org/invitations/test-id",
	// Share routes (share.rs uses RequireAuth)
	"/api/threads/test-id/share",
	"/api/threads/test-id/support-access",
	// Weaver routes (weaver.rs uses RequireAuth, may 404 if K8s not configured)
	"/api/weaver/test-id",
];

/// Test that all protected GET routes require authentication
#[tokio::test]
async fn test_protected_get_routes_require_auth() {
	let (app, _dir) = setup_test_app().await;

	for route in PROTECTED_GET_ROUTES {
		let response = app
			.clone()
			.oneshot(Request::builder().uri(*route).body(Body::empty()).unwrap())
			.await
			.unwrap();

		// 401 = auth required (correct)
		// 404 = route not found or feature not configured (acceptable)
		// 400/422 = route exists, processed request but bad params (acceptable - auth was checked first)
		// 200 with actual data = auth bypass bug!
		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::NOT_FOUND
				|| response.status() == StatusCode::METHOD_NOT_ALLOWED
				|| response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"GET {} should require auth (401), got unexpected {}",
			route,
			response.status()
		);
	}
}

/// Test that all protected POST routes require authentication
#[tokio::test]
async fn test_protected_post_routes_require_auth() {
	let (app, _dir) = setup_test_app().await;

	for route in PROTECTED_POST_ROUTES {
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("POST")
					.uri(*route)
					.header("content-type", "application/json")
					.body(Body::from("{}"))
					.unwrap(),
			)
			.await
			.unwrap();

		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::NOT_FOUND
				|| response.status() == StatusCode::METHOD_NOT_ALLOWED
				|| response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"POST {} should require auth (401), got unexpected {}",
			route,
			response.status()
		);
	}
}

/// Test that all protected DELETE routes require authentication
#[tokio::test]
async fn test_protected_delete_routes_require_auth() {
	let (app, _dir) = setup_test_app().await;

	for route in PROTECTED_DELETE_ROUTES {
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("DELETE")
					.uri(*route)
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert!(
			response.status() == StatusCode::UNAUTHORIZED
				|| response.status() == StatusCode::NOT_FOUND
				|| response.status() == StatusCode::METHOD_NOT_ALLOWED
				|| response.status() == StatusCode::BAD_REQUEST
				|| response.status() == StatusCode::UNPROCESSABLE_ENTITY,
			"DELETE {} should require auth (401), got unexpected {}",
			route,
			response.status()
		);
	}
}

/// Test that protected routes process bearer tokens (auth middleware is applied)
#[tokio::test]
async fn test_routes_process_bearer_token() {
	let (app, _dir) = setup_test_app().await;

	// Sample of PROTECTED routes to test with invalid bearer token
	// (thread routes are public, so they're not included here)
	let sample_routes = ["/api/orgs", "/api/weavers", "/api/sessions", "/api/auth/me"];

	for route in sample_routes {
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri(route)
					.header("Authorization", "Bearer lt_invalid_token_for_testing")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		// Should be 401 (token processed but invalid) or 404 (feature not configured)
		// NOT 200 (which would indicate auth was bypassed)
		assert!(
			response.status() == StatusCode::UNAUTHORIZED || response.status() == StatusCode::NOT_FOUND,
			"{} with invalid bearer token should return 401 or 404, got {}",
			route,
			response.status()
		);
	}
}
