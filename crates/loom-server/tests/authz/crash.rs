// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Authorization tests for crash analytics routes.
//!
//! Key invariants:
//! - All crash endpoints require user authentication
//! - Users can only access projects/issues within orgs they are members of
//! - Cross-org isolation: users cannot see projects/issues from other orgs

use axum::http::StatusCode;
use serde_json::json;

use super::support::TestApp;

// ============================================================================
// Helper: Create a crash project and return its ID
// ============================================================================

async fn create_test_project(app: &TestApp, org_id: &str, slug: &str) -> String {
	let response = app
		.post(
			"/api/crash/projects",
			Some(&app.fixtures.org_a.member),
			json!({
				"org_id": org_id,
				"name": format!("Test Project {}", slug),
				"slug": slug,
				"platform": "javascript"
			}),
		)
		.await;

	assert_eq!(
		response.status(),
		StatusCode::CREATED,
		"Failed to create test project"
	);

	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

	result["id"].as_str().unwrap().to_string()
}

// ============================================================================
// Project List Tests
// ============================================================================

#[tokio::test]
async fn list_projects_requires_auth() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// No auth should return 401
	let response = app
		.get(&format!("/api/crash/projects?org_id={}", org_id), None)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::UNAUTHORIZED,
		"List projects without auth should return 401"
	);
}

#[tokio::test]
async fn list_projects_requires_org_membership() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_b.org.id.to_string();

	// User from org_a trying to list projects in org_b should be forbidden
	let response = app
		.get(
			&format!("/api/crash/projects?org_id={}", org_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::FORBIDDEN,
		"Non-member should not be able to list projects in another org"
	);
}

#[tokio::test]
async fn list_projects_succeeds_for_org_member() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();

	let response = app
		.get(
			&format!("/api/crash/projects?org_id={}", org_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Org member should be able to list projects"
	);
}

// ============================================================================
// Project Create Tests
// ============================================================================

#[tokio::test]
async fn create_project_requires_auth() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// No auth should return 401
	let response = app
		.post(
			"/api/crash/projects",
			None,
			json!({
				"org_id": org_id,
				"name": "Test Project",
				"slug": "test-project",
				"platform": "javascript"
			}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::UNAUTHORIZED,
		"Create project without auth should return 401"
	);
}

#[tokio::test]
async fn create_project_requires_org_membership() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_b.org.id.to_string();

	// User from org_a trying to create project in org_b should be forbidden
	let response = app
		.post(
			"/api/crash/projects",
			Some(&app.fixtures.org_a.member),
			json!({
				"org_id": org_id,
				"name": "Test Project",
				"slug": "test-project",
				"platform": "javascript"
			}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::FORBIDDEN,
		"Non-member should not be able to create project in another org"
	);
}

#[tokio::test]
async fn create_project_succeeds_for_org_member() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();

	let response = app
		.post(
			"/api/crash/projects",
			Some(&app.fixtures.org_a.member),
			json!({
				"org_id": org_id,
				"name": "Test Project",
				"slug": "test-project-create",
				"platform": "javascript"
			}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::CREATED,
		"Org member should be able to create project"
	);
}

// ============================================================================
// Capture Tests
// ============================================================================

#[tokio::test]
async fn capture_crash_requires_auth() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// First create a project
	let project_id = create_test_project(&app, &org_id, "capture-auth-test").await;

	// No auth should return 401
	let response = app
		.post(
			"/api/crash/capture",
			None,
			json!({
				"project_id": project_id,
				"exception_type": "TypeError",
				"exception_value": "Cannot read property 'x' of undefined",
				"stacktrace": {
					"frames": [{
						"function": "handleClick",
						"filename": "app.js",
						"lineno": 42,
						"in_app": true
					}]
				}
			}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::UNAUTHORIZED,
		"Capture crash without auth should return 401"
	);
}

#[tokio::test]
async fn capture_crash_requires_org_membership() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// Create project in org_a
	let project_id = create_test_project(&app, &org_id, "capture-membership-test").await;

	// User from org_b trying to capture crash in org_a's project should be forbidden
	let response = app
		.post(
			"/api/crash/capture",
			Some(&app.fixtures.org_b.member),
			json!({
				"project_id": project_id,
				"exception_type": "TypeError",
				"exception_value": "Cannot read property 'x' of undefined",
				"stacktrace": {
					"frames": [{
						"function": "handleClick",
						"filename": "app.js",
						"lineno": 42,
						"in_app": true
					}]
				}
			}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::FORBIDDEN,
		"Non-member should not be able to capture crash in another org's project"
	);
}

#[tokio::test]
async fn capture_crash_succeeds_for_org_member() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();

	// Create project
	let project_id = create_test_project(&app, &org_id, "capture-success-test").await;

	// Org member can capture crash
	let response = app
		.post(
			"/api/crash/capture",
			Some(&app.fixtures.org_a.member),
			json!({
				"project_id": project_id,
				"exception_type": "TypeError",
				"exception_value": "Cannot read property 'x' of undefined",
				"stacktrace": {
					"frames": [{
						"function": "handleClick",
						"filename": "app.js",
						"lineno": 42,
						"in_app": true
					}]
				}
			}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Org member should be able to capture crash"
	);
}

// ============================================================================
// Issues List Tests
// ============================================================================

#[tokio::test]
async fn list_issues_requires_auth() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "issues-auth-test").await;

	// No auth should return 401
	let response = app
		.get(&format!("/api/crash/projects/{}/issues", project_id), None)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::UNAUTHORIZED,
		"List issues without auth should return 401"
	);
}

#[tokio::test]
async fn list_issues_requires_org_membership() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "issues-membership-test").await;

	// User from org_b trying to list issues in org_a's project should be forbidden
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues", project_id),
			Some(&app.fixtures.org_b.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::FORBIDDEN,
		"Non-member should not be able to list issues in another org's project"
	);
}

#[tokio::test]
async fn list_issues_succeeds_for_org_member() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "issues-success-test").await;

	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues", project_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Org member should be able to list issues"
	);
}

// ============================================================================
// Helper: Create a crash event and return the issue ID
// ============================================================================

async fn create_test_issue(app: &TestApp, project_id: &str) -> String {
	let response = app
		.post(
			"/api/crash/capture",
			Some(&app.fixtures.org_a.member),
			json!({
				"project_id": project_id,
				"exception_type": "TypeError",
				"exception_value": "Cannot read property 'x' of undefined",
				"stacktrace": {
					"frames": [{
						"function": "handleClick",
						"filename": "app.js",
						"lineno": 42,
						"in_app": true
					}]
				}
			}),
		)
		.await;

	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Failed to capture crash for test issue"
	);

	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let result: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

	result["issue_id"].as_str().unwrap().to_string()
}

// ============================================================================
// Get Issue Detail Tests
// ============================================================================

#[tokio::test]
async fn get_issue_requires_auth() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "get-issue-auth-test").await;
	let issue_id = create_test_issue(&app, &project_id).await;

	// No auth should return 401
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			None,
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::UNAUTHORIZED,
		"Get issue without auth should return 401"
	);
}

#[tokio::test]
async fn get_issue_requires_org_membership() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "get-issue-membership-test").await;
	let issue_id = create_test_issue(&app, &project_id).await;

	// User from org_b trying to get issue in org_a's project should be forbidden
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_b.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::FORBIDDEN,
		"Non-member should not be able to get issue in another org's project"
	);
}

#[tokio::test]
async fn get_issue_succeeds_for_org_member() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "get-issue-success-test").await;
	let issue_id = create_test_issue(&app, &project_id).await;

	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Org member should be able to get issue detail"
	);
}

#[tokio::test]
async fn get_issue_returns_404_for_nonexistent_issue() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "get-issue-404-test").await;

	let response = app
		.get(
			&format!(
				"/api/crash/projects/{}/issues/{}",
				project_id,
				uuid::Uuid::new_v4()
			),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::NOT_FOUND,
		"Getting nonexistent issue should return 404"
	);
}

// ============================================================================
// List Issue Events Tests
// ============================================================================

#[tokio::test]
async fn list_issue_events_requires_auth() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "list-events-auth-test").await;
	let issue_id = create_test_issue(&app, &project_id).await;

	// No auth should return 401
	let response = app
		.get(
			&format!(
				"/api/crash/projects/{}/issues/{}/events",
				project_id, issue_id
			),
			None,
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::UNAUTHORIZED,
		"List issue events without auth should return 401"
	);
}

#[tokio::test]
async fn list_issue_events_requires_org_membership() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "list-events-membership-test").await;
	let issue_id = create_test_issue(&app, &project_id).await;

	// User from org_b trying to list events in org_a's project should be forbidden
	let response = app
		.get(
			&format!(
				"/api/crash/projects/{}/issues/{}/events",
				project_id, issue_id
			),
			Some(&app.fixtures.org_b.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::FORBIDDEN,
		"Non-member should not be able to list events in another org's project"
	);
}

#[tokio::test]
async fn list_issue_events_succeeds_for_org_member() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "list-events-success-test").await;
	let issue_id = create_test_issue(&app, &project_id).await;

	let response = app
		.get(
			&format!(
				"/api/crash/projects/{}/issues/{}/events",
				project_id, issue_id
			),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Org member should be able to list issue events"
	);

	// Verify we got at least one event (from the test crash we created)
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let events: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
	assert!(
		!events.is_empty(),
		"Should have at least one event for the issue"
	);
}

#[tokio::test]
async fn list_issue_events_returns_404_for_nonexistent_issue() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "list-events-404-test").await;

	let response = app
		.get(
			&format!(
				"/api/crash/projects/{}/issues/{}/events",
				project_id,
				uuid::Uuid::new_v4()
			),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::NOT_FOUND,
		"Listing events for nonexistent issue should return 404"
	);
}
