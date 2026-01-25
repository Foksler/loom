// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Authorization tests for clips (code snippets).
//!
//! Note: The clips API currently has basic authentication requirements but limited
//! authorization checks. These tests document the current behavior. Additional
//! authorization (visibility checks, org membership for viewing) may be added later.

use axum::http::{Method, StatusCode};
use serde_json::json;

use super::support::{run_authz_cases, AuthzCase, TestApp};

/// Test that authenticated users can create clips in their organizations
#[tokio::test]
async fn test_clip_create_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let member = &app.fixtures.org_a.member;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let other_org_id = app.fixtures.org_b.org.id.to_string();

	let cases = vec![
		// Org owner can create clip in their org
		AuthzCase {
			name: "org_owner_can_create_clip",
			method: Method::POST,
			path: "/api/clips".to_string(),
			user: Some(owner.clone()),
			body: Some(json!({
				"org_id": org_id,
				"name": "my-clip",
				"visibility": "private",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			})),
			expected_status: StatusCode::CREATED,
		},
		// Org member can create clip in their org
		AuthzCase {
			name: "org_member_can_create_clip",
			method: Method::POST,
			path: "/api/clips".to_string(),
			user: Some(member.clone()),
			body: Some(json!({
				"org_id": org_id,
				"name": "member-clip",
				"visibility": "private",
				"files": [{"path": "lib.rs", "content": "// lib"}]
			})),
			expected_status: StatusCode::CREATED,
		},
		// User cannot create clip in another org
		AuthzCase {
			name: "user_cannot_create_clip_in_other_org",
			method: Method::POST,
			path: "/api/clips".to_string(),
			user: Some(owner.clone()),
			body: Some(json!({
				"org_id": other_org_id,
				"name": "unauthorized-clip",
				"visibility": "private",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			})),
			expected_status: StatusCode::FORBIDDEN,
		},
		// Unauthenticated user cannot create clip
		AuthzCase {
			name: "unauthenticated_cannot_create_clip",
			method: Method::POST,
			path: "/api/clips".to_string(),
			user: None,
			body: Some(json!({
				"org_id": org_id,
				"name": "anon-clip",
				"visibility": "private",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			})),
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

/// Test that owners can view clips in their org
#[tokio::test]
async fn test_clip_view_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let org_a_name = &app.fixtures.org_a.org.slug;

	// Create a clip
	let create_response = app
		.post(
			"/api/clips",
			Some(owner),
			json!({
				"org_id": org_id,
				"name": "test-view-clip",
				"visibility": "private",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);

	let cases = vec![
		// Owner can view their clip
		AuthzCase {
			name: "owner_can_view_clip",
			method: Method::GET,
			path: format!("/api/clips/{org_a_name}/test-view-clip"),
			user: Some(owner.clone()),
			body: None,
			expected_status: StatusCode::OK,
		},
		// Unauthenticated cannot view clip (requires auth)
		AuthzCase {
			name: "unauthenticated_cannot_view_clip",
			method: Method::GET,
			path: format!("/api/clips/{org_a_name}/test-view-clip"),
			user: None,
			body: None,
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

/// Test that clip deletion requires proper authorization
#[tokio::test]
async fn test_clip_delete_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let other_user = &app.fixtures.org_b.owner;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// Create clips for delete tests
	let create_for_other = app
		.post(
			"/api/clips",
			Some(owner),
			json!({
				"org_id": org_id,
				"name": "delete-other-test",
				"visibility": "private",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			}),
		)
		.await;
	assert_eq!(create_for_other.status(), StatusCode::CREATED);
	let body = axum::body::to_bytes(create_for_other.into_body(), usize::MAX)
		.await
		.unwrap();
	let other_clip: serde_json::Value = serde_json::from_slice(&body).unwrap();
	let other_clip_id = other_clip["id"].as_str().unwrap();

	let create_for_anon = app
		.post(
			"/api/clips",
			Some(owner),
			json!({
				"org_id": org_id,
				"name": "delete-anon-test",
				"visibility": "private",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			}),
		)
		.await;
	assert_eq!(create_for_anon.status(), StatusCode::CREATED);
	let body = axum::body::to_bytes(create_for_anon.into_body(), usize::MAX)
		.await
		.unwrap();
	let anon_clip: serde_json::Value = serde_json::from_slice(&body).unwrap();
	let anon_clip_id = anon_clip["id"].as_str().unwrap();

	let create_for_owner = app
		.post(
			"/api/clips",
			Some(owner),
			json!({
				"org_id": org_id,
				"name": "delete-owner-test",
				"visibility": "private",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			}),
		)
		.await;
	assert_eq!(create_for_owner.status(), StatusCode::CREATED);
	let body = axum::body::to_bytes(create_for_owner.into_body(), usize::MAX)
		.await
		.unwrap();
	let owner_clip: serde_json::Value = serde_json::from_slice(&body).unwrap();
	let owner_clip_id = owner_clip["id"].as_str().unwrap();

	let cases = vec![
		// Non-member cannot delete clip
		AuthzCase {
			name: "other_user_cannot_delete_clip",
			method: Method::DELETE,
			path: format!("/api/clips/{other_clip_id}"),
			user: Some(other_user.clone()),
			body: None,
			expected_status: StatusCode::FORBIDDEN,
		},
		// Unauthenticated cannot delete clip
		AuthzCase {
			name: "unauthenticated_cannot_delete_clip",
			method: Method::DELETE,
			path: format!("/api/clips/{anon_clip_id}"),
			user: None,
			body: None,
			expected_status: StatusCode::UNAUTHORIZED,
		},
		// Owner can delete their clip
		AuthzCase {
			name: "owner_can_delete_clip",
			method: Method::DELETE,
			path: format!("/api/clips/{owner_clip_id}"),
			user: Some(owner.clone()),
			body: None,
			expected_status: StatusCode::NO_CONTENT,
		},
	];

	run_authz_cases(&app, &cases).await;
}

/// Test that starring/unstarring requires authentication
#[tokio::test]
async fn test_clip_star_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// Create a clip to star
	let create_response = app
		.post(
			"/api/clips",
			Some(owner),
			json!({
				"org_id": org_id,
				"name": "star-test",
				"visibility": "public",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);
	let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let clip: serde_json::Value = serde_json::from_slice(&body).unwrap();
	let clip_id = clip["id"].as_str().unwrap();

	let cases = vec![
		// Authenticated user can star clip
		AuthzCase {
			name: "authenticated_can_star_clip",
			method: Method::POST,
			path: format!("/api/clips/{clip_id}/star"),
			user: Some(owner.clone()),
			body: Some(json!({})),  // Try with empty body
			expected_status: StatusCode::OK,
		},
		// Unauthenticated cannot star clip
		AuthzCase {
			name: "unauthenticated_cannot_star_clip",
			method: Method::POST,
			path: format!("/api/clips/{clip_id}/star"),
			user: None,
			body: Some(json!({})),
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

/// Test public clips listing
#[tokio::test]
async fn test_clip_public_listing() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// Create a public clip
	let create_response = app
		.post(
			"/api/clips",
			Some(owner),
			json!({
				"org_id": org_id,
				"name": "public-list-test",
				"visibility": "public",
				"files": [{"path": "main.rs", "content": "fn main() {}"}]
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);

	let cases = vec![
		// Authenticated user can list public clips
		AuthzCase {
			name: "authenticated_can_list_public_clips",
			method: Method::GET,
			path: "/api/clips/public".to_string(),
			user: Some(owner.clone()),
			body: None,
			expected_status: StatusCode::OK,
		},
		// Unauthenticated user cannot list public clips (requires auth)
		AuthzCase {
			name: "unauthenticated_cannot_list_public_clips",
			method: Method::GET,
			path: "/api/clips/public".to_string(),
			user: None,
			body: None,
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

// TODO: Additional authorization tests needed when authorization is implemented:
// - test_clip_visibility_authorization: Test that private clips are not visible to non-members
// - test_clip_update_authorization: Test that only org members can update clips
// - test_clip_fork_authorization: Test that users cannot fork private clips they can't see
// - test_clip_list_org_clips: Test that only org members can list org clips
// - test_clip_list_user_clips: Test that users can only list their own clips
// - test_clip_search_only_public: Test that search only returns visible clips
