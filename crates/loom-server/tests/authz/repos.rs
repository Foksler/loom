// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use axum::http::{Method, StatusCode};
use serde_json::json;

use super::support::{run_authz_cases, AuthzCase, TestApp};

#[tokio::test]
async fn test_repo_create_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let member = &app.fixtures.org_a.member;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let user_id = owner.user.id.to_string();
	let member_user_id = member.user.id.to_string();

	let cases = vec![
		// User can create repo for themselves
		AuthzCase {
			name: "user_can_create_own_repo",
			method: Method::POST,
			path: "/api/repos".to_string(),
			user: Some(owner.clone()),
			body: Some(json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "my-repo",
				"visibility": "private"
			})),
			expected_status: StatusCode::CREATED,
		},
		// User cannot create repo for another user
		AuthzCase {
			name: "user_cannot_create_repo_for_other_user",
			method: Method::POST,
			path: "/api/repos".to_string(),
			user: Some(member.clone()),
			body: Some(json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "unauthorized-repo",
				"visibility": "private"
			})),
			expected_status: StatusCode::FORBIDDEN,
		},
		// Org owner can create repo for org
		AuthzCase {
			name: "org_owner_can_create_org_repo",
			method: Method::POST,
			path: "/api/repos".to_string(),
			user: Some(owner.clone()),
			body: Some(json!({
				"owner_type": "org",
				"owner_id": org_id,
				"name": "org-repo",
				"visibility": "private"
			})),
			expected_status: StatusCode::CREATED,
		},
		// Org member (not admin) cannot create repo for org
		AuthzCase {
			name: "org_member_cannot_create_org_repo",
			method: Method::POST,
			path: "/api/repos".to_string(),
			user: Some(member.clone()),
			body: Some(json!({
				"owner_type": "org",
				"owner_id": org_id,
				"name": "unauthorized-org-repo",
				"visibility": "private"
			})),
			expected_status: StatusCode::FORBIDDEN,
		},
		// Invalid repo name
		AuthzCase {
			name: "invalid_repo_name_rejected",
			method: Method::POST,
			path: "/api/repos".to_string(),
			user: Some(owner.clone()),
			body: Some(json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "invalid/name",
				"visibility": "private"
			})),
			expected_status: StatusCode::BAD_REQUEST,
		},
		// Unauthenticated request
		AuthzCase {
			name: "unauthenticated_cannot_create_repo",
			method: Method::POST,
			path: "/api/repos".to_string(),
			user: None,
			body: Some(json!({
				"owner_type": "user",
				"owner_id": member_user_id,
				"name": "anon-repo",
				"visibility": "private"
			})),
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn test_repo_get_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let other_user = &app.fixtures.org_b.owner;
	let user_id = owner.user.id.to_string();

	// First create a private repo
	let create_response = app
		.post(
			"/api/repos",
			Some(owner),
			json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "private-repo",
				"visibility": "private"
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);

	let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let created_repo: serde_json::Value = serde_json::from_slice(&body).unwrap();
	let repo_id = created_repo["id"].as_str().unwrap();

	let cases = vec![
		// Owner can get their own private repo
		AuthzCase {
			name: "owner_can_get_own_private_repo",
			method: Method::GET,
			path: format!("/api/repos/{repo_id}"),
			user: Some(owner.clone()),
			body: None,
			expected_status: StatusCode::OK,
		},
		// Other user cannot get private repo
		AuthzCase {
			name: "other_user_cannot_get_private_repo",
			method: Method::GET,
			path: format!("/api/repos/{repo_id}"),
			user: Some(other_user.clone()),
			body: None,
			expected_status: StatusCode::FORBIDDEN,
		},
		// Unauthenticated cannot get private repo
		AuthzCase {
			name: "unauthenticated_cannot_get_private_repo",
			method: Method::GET,
			path: format!("/api/repos/{repo_id}"),
			user: None,
			body: None,
			expected_status: StatusCode::UNAUTHORIZED,
		},
		// Non-existent repo returns 404
		AuthzCase {
			name: "nonexistent_repo_returns_404",
			method: Method::GET,
			path: "/api/repos/00000000-0000-0000-0000-000000000000".to_string(),
			user: Some(owner.clone()),
			body: None,
			expected_status: StatusCode::NOT_FOUND,
		},
	];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn test_repo_update_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let other_user = &app.fixtures.org_b.owner;
	let user_id = owner.user.id.to_string();

	// Create a repo to update
	let create_response = app
		.post(
			"/api/repos",
			Some(owner),
			json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "repo-to-update",
				"visibility": "private"
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);

	let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let created_repo: serde_json::Value = serde_json::from_slice(&body).unwrap();
	let repo_id = created_repo["id"].as_str().unwrap();

	let cases = vec![
		// Owner can update their repo
		AuthzCase {
			name: "owner_can_update_repo",
			method: Method::PATCH,
			path: format!("/api/repos/{repo_id}"),
			user: Some(owner.clone()),
			body: Some(json!({"name": "updated-repo-name"})),
			expected_status: StatusCode::OK,
		},
		// Other user cannot update repo
		AuthzCase {
			name: "other_user_cannot_update_repo",
			method: Method::PATCH,
			path: format!("/api/repos/{repo_id}"),
			user: Some(other_user.clone()),
			body: Some(json!({"name": "hacked-name"})),
			expected_status: StatusCode::FORBIDDEN,
		},
		// Unauthenticated cannot update repo
		AuthzCase {
			name: "unauthenticated_cannot_update_repo",
			method: Method::PATCH,
			path: format!("/api/repos/{repo_id}"),
			user: None,
			body: Some(json!({"name": "anon-update"})),
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn test_repo_delete_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let other_user = &app.fixtures.org_b.owner;
	let user_id = owner.user.id.to_string();

	// Create repos to delete
	let create_response = app
		.post(
			"/api/repos",
			Some(owner),
			json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "repo-to-delete-by-other",
				"visibility": "private"
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);
	let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let repo1: serde_json::Value = serde_json::from_slice(&body).unwrap();
	let repo1_id = repo1["id"].as_str().unwrap();

	let create_response2 = app
		.post(
			"/api/repos",
			Some(owner),
			json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "repo-to-delete-by-owner",
				"visibility": "private"
			}),
		)
		.await;
	assert_eq!(create_response2.status(), StatusCode::CREATED);
	let body2 = axum::body::to_bytes(create_response2.into_body(), usize::MAX)
		.await
		.unwrap();
	let repo2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
	let repo2_id = repo2["id"].as_str().unwrap();

	let cases = vec![
		// Other user cannot delete repo
		AuthzCase {
			name: "other_user_cannot_delete_repo",
			method: Method::DELETE,
			path: format!("/api/repos/{repo1_id}"),
			user: Some(other_user.clone()),
			body: None,
			expected_status: StatusCode::FORBIDDEN,
		},
		// Owner can delete their repo
		AuthzCase {
			name: "owner_can_delete_repo",
			method: Method::DELETE,
			path: format!("/api/repos/{repo2_id}"),
			user: Some(owner.clone()),
			body: None,
			expected_status: StatusCode::NO_CONTENT,
		},
		// Unauthenticated cannot delete repo
		AuthzCase {
			name: "unauthenticated_cannot_delete_repo",
			method: Method::DELETE,
			path: format!("/api/repos/{repo1_id}"),
			user: None,
			body: None,
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn test_repo_list_authorization() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let other_user = &app.fixtures.org_b.owner;
	let user_id = owner.user.id.to_string();
	let org_id = app.fixtures.org_a.org.id.to_string();

	// Create a user repo
	let _ = app
		.post(
			"/api/repos",
			Some(owner),
			json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "list-test-repo",
				"visibility": "private"
			}),
		)
		.await;

	let cases = vec![
		// User can list their own repos
		AuthzCase {
			name: "user_can_list_own_repos",
			method: Method::GET,
			path: format!("/api/users/{user_id}/repos"),
			user: Some(owner.clone()),
			body: None,
			expected_status: StatusCode::OK,
		},
		// Other user can list public repos (returns empty if all private)
		AuthzCase {
			name: "other_user_can_list_user_repos",
			method: Method::GET,
			path: format!("/api/users/{user_id}/repos"),
			user: Some(other_user.clone()),
			body: None,
			expected_status: StatusCode::OK,
		},
		// Member can list org repos
		AuthzCase {
			name: "member_can_list_org_repos",
			method: Method::GET,
			path: format!("/api/orgs/{org_id}/repos"),
			user: Some(app.fixtures.org_a.member.clone()),
			body: None,
			expected_status: StatusCode::OK,
		},
		// Unauthenticated cannot list repos
		AuthzCase {
			name: "unauthenticated_cannot_list_user_repos",
			method: Method::GET,
			path: format!("/api/users/{user_id}/repos"),
			user: None,
			body: None,
			expected_status: StatusCode::UNAUTHORIZED,
		},
	];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn test_repo_duplicate_name_conflict() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let user_id = owner.user.id.to_string();

	// Create first repo
	let create_response = app
		.post(
			"/api/repos",
			Some(owner),
			json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "duplicate-name-test",
				"visibility": "private"
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);

	// Try to create repo with same name - should conflict
	let cases = vec![AuthzCase {
		name: "duplicate_repo_name_returns_conflict",
		method: Method::POST,
		path: "/api/repos".to_string(),
		user: Some(owner.clone()),
		body: Some(json!({
			"owner_type": "user",
			"owner_id": user_id,
			"name": "duplicate-name-test",
			"visibility": "private"
		})),
		expected_status: StatusCode::CONFLICT,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn test_repo_response_contains_expected_fields() {
	let app = TestApp::new().await;
	let owner = &app.fixtures.org_a.owner;
	let user_id = owner.user.id.to_string();

	let create_response = app
		.post(
			"/api/repos",
			Some(owner),
			json!({
				"owner_type": "user",
				"owner_id": user_id,
				"name": "field-test-repo",
				"visibility": "public"
			}),
		)
		.await;
	assert_eq!(create_response.status(), StatusCode::CREATED);

	let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
		.await
		.unwrap();
	let repo: serde_json::Value = serde_json::from_slice(&body).unwrap();

	// Verify all expected fields are present
	assert!(repo.get("id").is_some(), "Response should have 'id' field");
	assert!(
		repo.get("owner_type").is_some(),
		"Response should have 'owner_type' field"
	);
	assert!(
		repo.get("owner_id").is_some(),
		"Response should have 'owner_id' field"
	);
	assert!(
		repo.get("name").is_some(),
		"Response should have 'name' field"
	);
	assert!(
		repo.get("visibility").is_some(),
		"Response should have 'visibility' field"
	);
	assert!(
		repo.get("default_branch").is_some(),
		"Response should have 'default_branch' field"
	);
	assert!(
		repo.get("clone_url").is_some(),
		"Response should have 'clone_url' field"
	);
	assert!(
		repo.get("created_at").is_some(),
		"Response should have 'created_at' field"
	);
	assert!(
		repo.get("updated_at").is_some(),
		"Response should have 'updated_at' field"
	);

	// Verify default branch is 'cannon'
	assert_eq!(
		repo["default_branch"].as_str().unwrap(),
		"cannon",
		"Default branch should be 'cannon'"
	);

	// Verify name matches
	assert_eq!(
		repo["name"].as_str().unwrap(),
		"field-test-repo",
		"Name should match"
	);

	// Verify visibility matches
	assert_eq!(
		repo["visibility"].as_str().unwrap(),
		"public",
		"Visibility should match"
	);
}
