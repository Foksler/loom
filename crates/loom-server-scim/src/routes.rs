// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::sync::Arc;

use axum::{
	middleware,
	routing::{get, post},
	Router,
};
use loom_common_secret::SecretString;
use loom_server_auth::OrgId;
use loom_server_provisioning::UserProvisioningService;
use sqlx::SqlitePool;

use crate::auth::scim_auth_middleware;
use crate::handlers::users::ScimState;
use crate::handlers::{bulk, groups, schemas, service_provider, users};

pub fn scim_routes(
	pool: SqlitePool,
	token: Option<SecretString>,
	org_id: OrgId,
	provisioning: Arc<UserProvisioningService>,
) -> Router {
	let state = ScimState { pool, org_id, provisioning };

	Router::new()
		.route(
			"/ServiceProviderConfig",
			get(service_provider::get_service_provider_config),
		)
		.route("/Schemas", get(schemas::list_schemas))
		.route("/Schemas/{id}", get(schemas::get_schema))
		.route("/Users", get(users::list_users).post(users::create_user))
		.route(
			"/Users/{id}",
			get(users::get_user)
				.put(users::replace_user)
				.patch(users::patch_user)
				.delete(users::delete_user),
		)
		.route(
			"/Groups",
			get(groups::list_groups).post(groups::create_group),
		)
		.route(
			"/Groups/{id}",
			get(groups::get_group)
				.put(groups::replace_group)
				.patch(groups::patch_group)
				.delete(groups::delete_group),
		)
		.route("/Bulk", post(bulk::bulk_operations))
		.layer(middleware::from_fn_with_state(token, scim_auth_middleware))
		.with_state(state)
}
