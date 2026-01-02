// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Authorization tests for thread routes.
//!
//! Tests verify access control for thread operations:
//! - Users can list and access their own threads
//! - Authentication is required for all thread operations
//! - Cross-org access is denied (users cannot access threads owned by others)
//!
//! Tests marked with `#[ignore]` represent expected security behavior that requires
//! ownership-based authorization to be implemented. Once authorization is added,
//! remove the `#[ignore]` attribute to enable these tests.

use axum::http::{Method, StatusCode};
use loom_common_thread::{Thread, ThreadVisibility};

use super::support::{run_authz_cases, AuthzCase, TestApp};

// ============================================================================
// GET /api/threads - List threads
// ============================================================================

#[tokio::test]
async fn owner_can_list_threads() {
	let app = TestApp::new().await;
	let cases = [AuthzCase {
		name: "owner_can_list_threads",
		method: Method::GET,
		path: "/api/threads".to_string(),
		user: Some(app.fixtures.org_a.owner.clone()),
		body: None,
		expected_status: StatusCode::OK,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn member_can_list_threads() {
	let app = TestApp::new().await;
	let cases = [AuthzCase {
		name: "member_can_list_threads",
		method: Method::GET,
		path: "/api/threads".to_string(),
		user: Some(app.fixtures.org_a.member.clone()),
		body: None,
		expected_status: StatusCode::OK,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn unauthenticated_cannot_list_threads() {
	let app = TestApp::new().await;
	let cases = [AuthzCase {
		name: "unauthenticated_cannot_list_threads",
		method: Method::GET,
		path: "/api/threads".to_string(),
		user: None,
		body: None,
		expected_status: StatusCode::UNAUTHORIZED,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
#[ignore = "requires ownership-based authorization"]
async fn list_threads_scoped_to_user() {
	let app = TestApp::new().await;

	let response = app
		.get("/api/threads", Some(&app.fixtures.org_a.owner))
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

	let threads = json["threads"].as_array().expect("threads should be array");
	let org_b_thread_id = app.fixtures.org_b.thread.id.as_str();
	let contains_org_b_thread = threads
		.iter()
		.any(|t| t["id"].as_str() == Some(org_b_thread_id));
	assert!(
		!contains_org_b_thread,
		"org_a.owner should not see org_b's threads"
	);
}

// ============================================================================
// GET /api/threads/{id} - Get thread
// ============================================================================

#[tokio::test]
async fn owner_can_get_own_thread() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();
	let cases = [AuthzCase {
		name: "owner_can_get_own_thread",
		method: Method::GET,
		path: format!("/api/threads/{thread_id}"),
		user: Some(app.fixtures.org_a.owner.clone()),
		body: None,
		expected_status: StatusCode::OK,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn member_can_get_org_thread() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();
	let cases = [AuthzCase {
		name: "member_can_get_org_thread",
		method: Method::GET,
		path: format!("/api/threads/{thread_id}"),
		user: Some(app.fixtures.org_a.member.clone()),
		body: None,
		expected_status: StatusCode::OK,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn unauthenticated_cannot_get_thread() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();
	let cases = [AuthzCase {
		name: "unauthenticated_cannot_get_thread",
		method: Method::GET,
		path: format!("/api/threads/{thread_id}"),
		user: None,
		body: None,
		expected_status: StatusCode::UNAUTHORIZED,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
#[ignore = "requires ownership-based authorization"]
async fn other_org_cannot_get_thread() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();
	let cases = [AuthzCase {
		name: "other_org_cannot_get_thread",
		method: Method::GET,
		path: format!("/api/threads/{thread_id}"),
		user: Some(app.fixtures.org_b.member.clone()),
		body: None,
		expected_status: StatusCode::NOT_FOUND,
	}];
	run_authz_cases(&app, &cases).await;
}

// ============================================================================
// PUT /api/threads/{id} - Upsert thread
// ============================================================================

#[tokio::test]
async fn owner_can_upsert_thread() {
	let app = TestApp::new().await;
	let mut thread = app.fixtures.org_a.thread.clone();
	thread.version += 1;

	let response = app
		.put(
			&format!("/api/threads/{}", thread.id.as_str()),
			Some(&app.fixtures.org_a.owner),
			&thread,
		)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn unauthenticated_cannot_upsert_thread() {
	let app = TestApp::new().await;
	let mut thread = app.fixtures.org_a.thread.clone();
	thread.version += 1;

	let response = app
		.put(&format!("/api/threads/{}", thread.id.as_str()), None, &thread)
		.await;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires ownership-based authorization"]
async fn other_org_cannot_upsert_thread() {
	let app = TestApp::new().await;
	let mut thread = app.fixtures.org_a.thread.clone();
	thread.version += 1;

	let response = app
		.put(
			&format!("/api/threads/{}", thread.id.as_str()),
			Some(&app.fixtures.org_b.member),
			&thread,
		)
		.await;

	let status = response.status();
	assert!(
		status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN,
		"Expected 401 or 403, got {status}"
	);
}

// ============================================================================
// DELETE /api/threads/{id} - Delete thread
// ============================================================================

#[tokio::test]
async fn owner_can_delete_thread() {
	let app = TestApp::new().await;

	let thread = Thread::new();
	app.state.repo.upsert(&thread, None).await.unwrap();
	app.state
		.repo
		.set_owner_user_id(thread.id.as_str(), &app.fixtures.org_a.owner.user.id.to_string())
		.await
		.unwrap();

	let response = app
		.delete(
			&format!("/api/threads/{}", thread.id.as_str()),
			Some(&app.fixtures.org_a.owner),
		)
		.await;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn unauthenticated_cannot_delete_thread() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();
	let cases = [AuthzCase {
		name: "unauthenticated_cannot_delete_thread",
		method: Method::DELETE,
		path: format!("/api/threads/{thread_id}"),
		user: None,
		body: None,
		expected_status: StatusCode::UNAUTHORIZED,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
#[ignore = "requires ownership-based authorization"]
async fn other_org_cannot_delete_thread() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();
	let cases = [AuthzCase {
		name: "other_org_cannot_delete_thread",
		method: Method::DELETE,
		path: format!("/api/threads/{thread_id}"),
		user: Some(app.fixtures.org_b.member.clone()),
		body: None,
		expected_status: StatusCode::NOT_FOUND,
	}];
	run_authz_cases(&app, &cases).await;
}

// ============================================================================
// POST /api/threads/{id}/visibility - Update visibility
// ============================================================================

#[tokio::test]
async fn owner_can_update_visibility() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();

	let body = serde_json::json!({
		"visibility": ThreadVisibility::Public
	});

	let response = app
		.post(
			&format!("/api/threads/{thread_id}/visibility"),
			Some(&app.fixtures.org_a.owner),
			body,
		)
		.await;
	assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn unauthenticated_cannot_update_visibility() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();

	let body = serde_json::json!({
		"visibility": ThreadVisibility::Public
	});

	let cases = [AuthzCase {
		name: "unauthenticated_cannot_update_visibility",
		method: Method::POST,
		path: format!("/api/threads/{thread_id}/visibility"),
		user: None,
		body: Some(body),
		expected_status: StatusCode::UNAUTHORIZED,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
#[ignore = "requires ownership-based authorization"]
async fn other_org_cannot_update_visibility() {
	let app = TestApp::new().await;
	let thread_id = app.fixtures.org_a.thread.id.as_str();

	let body = serde_json::json!({
		"visibility": ThreadVisibility::Public
	});

	let cases = [AuthzCase {
		name: "other_org_cannot_update_visibility",
		method: Method::POST,
		path: format!("/api/threads/{thread_id}/visibility"),
		user: Some(app.fixtures.org_b.member.clone()),
		body: Some(body),
		expected_status: StatusCode::NOT_FOUND,
	}];
	run_authz_cases(&app, &cases).await;
}

// ============================================================================
// GET /api/threads/search - Search threads
// ============================================================================

#[tokio::test]
async fn authenticated_can_search_threads() {
	let app = TestApp::new().await;

	let response = app
		.get("/api/threads/search?q=test", Some(&app.fixtures.org_a.owner))
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert!(json["hits"].is_array(), "hits should be array");
}

#[tokio::test]
async fn unauthenticated_cannot_search_threads() {
	let app = TestApp::new().await;
	let cases = [AuthzCase {
		name: "unauthenticated_cannot_search_threads",
		method: Method::GET,
		path: "/api/threads/search?q=test".to_string(),
		user: None,
		body: None,
		expected_status: StatusCode::UNAUTHORIZED,
	}];
	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
#[ignore = "requires ownership-based authorization"]
async fn search_scoped_to_user() {
	let app = TestApp::new().await;

	let response = app
		.get("/api/threads/search?q=test", Some(&app.fixtures.org_a.owner))
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

	let hits = json["hits"].as_array().expect("hits should be array");
	let org_b_thread_id = app.fixtures.org_b.thread.id.as_str();
	let contains_org_b_thread = hits
		.iter()
		.any(|h| h["id"].as_str() == Some(org_b_thread_id));
	assert!(
		!contains_org_b_thread,
		"org_a.owner search results should not contain org_b's threads"
	);
}
