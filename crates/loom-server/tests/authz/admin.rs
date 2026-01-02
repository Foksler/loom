// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Authorization tests for admin routes.
//!
//! Key invariant: ALL admin routes require `is_system_admin = true`.

use axum::http::{Method, StatusCode};
use serde_json::json;

use super::support::{run_authz_cases, AuthzCase, TestApp};

#[tokio::test]
async fn admin_can_list_all_users() {
	let app = TestApp::new().await;

	let cases = vec![AuthzCase {
		name: "admin_can_list_all_users",
		method: Method::GET,
		path: "/api/admin/users".to_string(),
		user: Some(app.fixtures.admin.clone()),
		body: None,
		expected_status: StatusCode::OK,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn non_admin_cannot_list_users() {
	let app = TestApp::new().await;

	let cases = vec![AuthzCase {
		name: "non_admin_cannot_list_users",
		method: Method::GET,
		path: "/api/admin/users".to_string(),
		user: Some(app.fixtures.org_a.owner.clone()),
		body: None,
		expected_status: StatusCode::FORBIDDEN,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn admin_can_update_roles() {
	let app = TestApp::new().await;
	let target_id = app.fixtures.org_a.member.user.id.to_string();

	let cases = vec![AuthzCase {
		name: "admin_can_update_roles",
		method: Method::PATCH,
		path: format!("/api/admin/users/{}/roles", target_id),
		user: Some(app.fixtures.admin.clone()),
		body: Some(json!({ "is_auditor": true })),
		expected_status: StatusCode::OK,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn non_admin_cannot_update_roles() {
	let app = TestApp::new().await;
	let target_id = app.fixtures.org_a.member.user.id.to_string();

	let cases = vec![AuthzCase {
		name: "non_admin_cannot_update_roles",
		method: Method::PATCH,
		path: format!("/api/admin/users/{}/roles", target_id),
		user: Some(app.fixtures.org_a.owner.clone()),
		body: Some(json!({ "is_auditor": true })),
		expected_status: StatusCode::FORBIDDEN,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn admin_can_impersonate() {
	let app = TestApp::new().await;
	let target_id = app.fixtures.org_a.member.user.id.to_string();

	let cases = vec![AuthzCase {
		name: "admin_can_impersonate",
		method: Method::POST,
		path: format!("/api/admin/users/{}/impersonate", target_id),
		user: Some(app.fixtures.admin.clone()),
		body: Some(json!({ "reason": "Testing impersonation" })),
		expected_status: StatusCode::OK,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn non_admin_cannot_impersonate() {
	let app = TestApp::new().await;
	let target_id = app.fixtures.org_a.member.user.id.to_string();

	let cases = vec![AuthzCase {
		name: "non_admin_cannot_impersonate",
		method: Method::POST,
		path: format!("/api/admin/users/{}/impersonate", target_id),
		user: Some(app.fixtures.org_a.owner.clone()),
		body: Some(json!({ "reason": "Testing impersonation" })),
		expected_status: StatusCode::FORBIDDEN,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn admin_can_stop_impersonation() {
	let app = TestApp::new().await;

	let cases = vec![AuthzCase {
		name: "admin_can_stop_impersonation",
		method: Method::POST,
		path: "/api/admin/impersonate/stop".to_string(),
		user: Some(app.fixtures.admin.clone()),
		body: Some(json!({})),
		expected_status: StatusCode::NOT_FOUND,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn non_admin_cannot_stop_impersonation() {
	let app = TestApp::new().await;

	let cases = vec![AuthzCase {
		name: "non_admin_cannot_stop_impersonation",
		method: Method::POST,
		path: "/api/admin/impersonate/stop".to_string(),
		user: Some(app.fixtures.org_a.owner.clone()),
		body: Some(json!({})),
		expected_status: StatusCode::FORBIDDEN,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn admin_can_list_audit_logs() {
	let app = TestApp::new().await;

	let cases = vec![AuthzCase {
		name: "admin_can_list_audit_logs",
		method: Method::GET,
		path: "/api/admin/audit-logs".to_string(),
		user: Some(app.fixtures.admin.clone()),
		body: None,
		expected_status: StatusCode::OK,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn non_admin_cannot_list_audit_logs() {
	let app = TestApp::new().await;

	let cases = vec![AuthzCase {
		name: "non_admin_cannot_list_audit_logs",
		method: Method::GET,
		path: "/api/admin/audit-logs".to_string(),
		user: Some(app.fixtures.org_a.owner.clone()),
		body: None,
		expected_status: StatusCode::FORBIDDEN,
	}];

	run_authz_cases(&app, &cases).await;
}

#[tokio::test]
async fn admin_cannot_remove_last_system_admin() {
	let app = TestApp::new().await;
	// The admin fixture is the only system admin in the test setup
	let admin_id = app.fixtures.admin.user.id.to_string();

	let cases = vec![AuthzCase {
		name: "admin_cannot_remove_last_system_admin",
		method: Method::PATCH,
		path: format!("/api/admin/users/{}/roles", admin_id),
		user: Some(app.fixtures.admin.clone()),
		body: Some(json!({ "is_system_admin": false })),
		// Should fail with 400 because this is the last admin AND because you can't remove your own admin
		expected_status: StatusCode::BAD_REQUEST,
	}];

	run_authz_cases(&app, &cases).await;
}
