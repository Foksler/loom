// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Integration tests for crash analytics business logic.
//!
//! These tests verify:
//! - Fingerprinting: same crashes are grouped into the same issue
//! - Issue state transitions: resolve → unresolve → resolve workflow
//! - Regression detection: new crashes for resolved issues create regressions

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
				"name": format!("Integration Test Project {}", slug),
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
// Helper: Capture a crash event and return the response
// ============================================================================

async fn capture_crash(
	app: &TestApp,
	project_id: &str,
	exception_type: &str,
	exception_value: &str,
	function: &str,
	filename: &str,
	lineno: i32,
	release: Option<&str>,
) -> serde_json::Value {
	let mut body = json!({
		"project_id": project_id,
		"exception_type": exception_type,
		"exception_value": exception_value,
		"stacktrace": {
			"frames": [{
				"function": function,
				"filename": filename,
				"lineno": lineno,
				"in_app": true
			}]
		}
	});

	if let Some(rel) = release {
		body["release"] = json!(rel);
	}

	let response = app
		.post("/api/crash/capture", Some(&app.fixtures.org_a.member), body)
		.await;

	assert_eq!(response.status(), StatusCode::OK, "Failed to capture crash");

	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	serde_json::from_slice(&body_bytes).unwrap()
}

// ============================================================================
// Fingerprinting Tests
// ============================================================================

/// Test that identical crashes (same exception type, value, and stack) are grouped
/// into the same issue via fingerprinting.
#[tokio::test]
async fn same_crashes_grouped_by_fingerprint() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "fingerprint-group-test").await;

	// Capture first crash
	let result1 = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Cannot read property 'x' of undefined",
		"handleClick",
		"app.js",
		42,
		None,
	)
	.await;

	let issue_id_1 = result1["issue_id"].as_str().unwrap();
	assert!(
		result1["is_new_issue"].as_bool().unwrap(),
		"First crash should create new issue"
	);

	// Capture identical crash (same type, value, stack)
	let result2 = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Cannot read property 'x' of undefined",
		"handleClick",
		"app.js",
		42,
		None,
	)
	.await;

	let issue_id_2 = result2["issue_id"].as_str().unwrap();
	assert!(
		!result2["is_new_issue"].as_bool().unwrap(),
		"Second crash should not create new issue"
	);

	// Both crashes should be grouped into the same issue
	assert_eq!(
		issue_id_1, issue_id_2,
		"Identical crashes should be grouped into the same issue"
	);
}

/// Test that different crashes (different exception type) create separate issues.
#[tokio::test]
async fn different_exception_types_create_separate_issues() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "fingerprint-diff-type-test").await;

	// Capture first crash with TypeError
	let result1 = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Cannot read property 'x' of undefined",
		"handleClick",
		"app.js",
		42,
		None,
	)
	.await;

	let issue_id_1 = result1["issue_id"].as_str().unwrap();

	// Capture second crash with ReferenceError (different exception type)
	let result2 = capture_crash(
		&app,
		&project_id,
		"ReferenceError",
		"Cannot read property 'x' of undefined",
		"handleClick",
		"app.js",
		42,
		None,
	)
	.await;

	let issue_id_2 = result2["issue_id"].as_str().unwrap();

	// Different exception types should create separate issues
	assert_ne!(
		issue_id_1, issue_id_2,
		"Different exception types should create separate issues"
	);
}

/// Test that crashes with different stack traces create separate issues.
#[tokio::test]
async fn different_stacks_create_separate_issues() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "fingerprint-diff-stack-test").await;

	// Capture first crash
	let result1 = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Cannot read property 'x' of undefined",
		"handleClick",
		"app.js",
		42,
		None,
	)
	.await;

	let issue_id_1 = result1["issue_id"].as_str().unwrap();

	// Capture second crash with different function name (different stack)
	let result2 = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Cannot read property 'x' of undefined",
		"handleSubmit", // Different function
		"app.js",
		42,
		None,
	)
	.await;

	let issue_id_2 = result2["issue_id"].as_str().unwrap();

	// Different stacks should create separate issues
	assert_ne!(
		issue_id_1, issue_id_2,
		"Different stack traces should create separate issues"
	);
}

// ============================================================================
// Issue State Transition Tests
// ============================================================================

/// Test the full issue lifecycle: unresolved → resolved → unresolved
#[tokio::test]
async fn issue_state_transitions() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "state-transition-test").await;

	// Create an issue by capturing a crash
	let capture_result = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"State transition test error",
		"testFunction",
		"test.js",
		100,
		None,
	)
	.await;

	let issue_id = capture_result["issue_id"].as_str().unwrap();

	// Verify issue starts as unresolved
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(issue["status"].as_str().unwrap(), "unresolved");

	// Resolve the issue
	let response = app
		.post(
			&format!(
				"/api/crash/projects/{}/issues/{}/resolve",
				project_id, issue_id
			),
			Some(&app.fixtures.org_a.member),
			json!({}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Should be able to resolve issue"
	);

	// Verify issue is now resolved
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(issue["status"].as_str().unwrap(), "resolved");

	// Unresolve the issue
	let response = app
		.post(
			&format!(
				"/api/crash/projects/{}/issues/{}/unresolve",
				project_id, issue_id
			),
			Some(&app.fixtures.org_a.member),
			json!({}),
		)
		.await;
	assert_eq!(
		response.status(),
		StatusCode::OK,
		"Should be able to unresolve issue"
	);

	// Verify issue is unresolved again
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(issue["status"].as_str().unwrap(), "unresolved");
}

/// Test that resolving an issue with resolved_in_release stores the release
#[tokio::test]
async fn resolve_with_release_version() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "resolve-release-test").await;

	// Create an issue
	let capture_result = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Resolve release test error",
		"testFunction",
		"test.js",
		200,
		Some("v1.0.0"),
	)
	.await;

	let issue_id = capture_result["issue_id"].as_str().unwrap();

	// Resolve with a specific release version
	let response = app
		.post(
			&format!(
				"/api/crash/projects/{}/issues/{}/resolve",
				project_id, issue_id
			),
			Some(&app.fixtures.org_a.member),
			json!({
				"resolved_in_release": "v2.0.0"
			}),
		)
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	// Verify the resolved_in_release is stored
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(issue["status"].as_str().unwrap(), "resolved");
	assert_eq!(
		issue["resolved_in_release"].as_str().unwrap(),
		"v2.0.0",
		"Should store resolved_in_release"
	);
}

/// Test that ignoring an issue changes its status to ignored
#[tokio::test]
async fn ignore_issue_changes_status() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "ignore-test").await;

	// Create an issue
	let capture_result = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Ignore test error",
		"testFunction",
		"test.js",
		300,
		None,
	)
	.await;

	let issue_id = capture_result["issue_id"].as_str().unwrap();

	// Ignore the issue
	let response = app
		.post(
			&format!(
				"/api/crash/projects/{}/issues/{}/ignore",
				project_id, issue_id
			),
			Some(&app.fixtures.org_a.member),
			json!({}),
		)
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	// Verify issue is ignored
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(issue["status"].as_str().unwrap(), "ignored");
}

// ============================================================================
// Regression Detection Tests
// ============================================================================

/// Test that capturing a crash for a resolved issue triggers regression detection.
/// The issue should transition to "regressed" status.
#[tokio::test]
async fn regression_detection_on_new_crash() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "regression-test").await;

	// Create an issue by capturing a crash with release v1.0.0
	let capture_result = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Regression test error",
		"regressionFunction",
		"regression.js",
		500,
		Some("v1.0.0"),
	)
	.await;

	let issue_id = capture_result["issue_id"].as_str().unwrap();
	assert!(capture_result["is_new_issue"].as_bool().unwrap());

	// Resolve the issue
	let response = app
		.post(
			&format!(
				"/api/crash/projects/{}/issues/{}/resolve",
				project_id, issue_id
			),
			Some(&app.fixtures.org_a.member),
			json!({}),
		)
		.await;
	assert_eq!(response.status(), StatusCode::OK);

	// Verify issue is resolved
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(issue["status"].as_str().unwrap(), "resolved");

	// Capture the same crash again with a new release (this should trigger regression)
	let capture_result2 = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Regression test error",
		"regressionFunction",
		"regression.js",
		500,
		Some("v2.0.0"),
	)
	.await;

	// The crash should be grouped into the existing issue
	assert_eq!(
		capture_result2["issue_id"].as_str().unwrap(),
		issue_id,
		"Same crash should be grouped into existing issue"
	);
	assert!(!capture_result2["is_new_issue"].as_bool().unwrap());
	assert!(
		capture_result2["is_regression"].as_bool().unwrap(),
		"Should detect regression when crash happens after resolve"
	);

	// Verify issue is now regressed
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(
		issue["status"].as_str().unwrap(),
		"regressed",
		"Issue should be marked as regressed"
	);
	assert_eq!(
		issue["regressed_in_release"].as_str().unwrap(),
		"v2.0.0",
		"Should record the release that caused regression"
	);
	assert_eq!(
		issue["times_regressed"].as_i64().unwrap(),
		1,
		"times_regressed should be incremented"
	);
}

/// Test that crashes on an already unresolved issue do not trigger regression
#[tokio::test]
async fn no_regression_on_unresolved_issue() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "no-regression-test").await;

	// Create an issue
	let capture_result = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"No regression test error",
		"noRegressionFunction",
		"noregression.js",
		600,
		Some("v1.0.0"),
	)
	.await;

	let issue_id = capture_result["issue_id"].as_str().unwrap();
	assert!(capture_result["is_new_issue"].as_bool().unwrap());

	// Capture the same crash again (without resolving first)
	let capture_result2 = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"No regression test error",
		"noRegressionFunction",
		"noregression.js",
		600,
		Some("v2.0.0"),
	)
	.await;

	// Should be grouped to same issue but NOT a regression
	assert_eq!(capture_result2["issue_id"].as_str().unwrap(), issue_id);
	assert!(!capture_result2["is_new_issue"].as_bool().unwrap());
	assert!(
		!capture_result2["is_regression"].as_bool().unwrap(),
		"Should NOT be a regression on unresolved issue"
	);

	// Verify issue status is still unresolved (not regressed)
	let response = app
		.get(
			&format!("/api/crash/projects/{}/issues/{}", project_id, issue_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let issue: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(
		issue["status"].as_str().unwrap(),
		"unresolved",
		"Issue should remain unresolved"
	);
}

// ============================================================================
// Release Auto-Creation Tests
// ============================================================================

/// Test that capturing a crash with a release version auto-creates the release
#[tokio::test]
async fn auto_create_release_on_crash() {
	let app = TestApp::new().await;
	let org_id = app.fixtures.org_a.org.id.to_string();
	let project_id = create_test_project(&app, &org_id, "auto-release-test").await;

	// Verify no releases exist yet
	let response = app
		.get(
			&format!("/api/crash/projects/{}/releases", project_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let releases: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
	assert!(releases.is_empty(), "Should have no releases initially");

	// Capture a crash with a release version
	let _capture_result = capture_crash(
		&app,
		&project_id,
		"TypeError",
		"Auto release test error",
		"autoReleaseFunction",
		"autorelease.js",
		700,
		Some("v3.0.0"),
	)
	.await;

	// Verify the release was auto-created
	let response = app
		.get(
			&format!("/api/crash/projects/{}/releases", project_id),
			Some(&app.fixtures.org_a.member),
		)
		.await;
	let (_, body) = response.into_parts();
	let body_bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
	let releases: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
	assert_eq!(releases.len(), 1, "Should have one release auto-created");
	assert_eq!(
		releases[0]["version"].as_str().unwrap(),
		"v3.0.0",
		"Release version should match"
	);
	assert_eq!(
		releases[0]["crash_count"].as_i64().unwrap(),
		1,
		"Release crash count should be 1"
	);
}
