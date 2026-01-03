// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use axum::{extract::State, Json};
use loom_scim::ScimError;
use serde::{Deserialize, Serialize};

use crate::error::ScimApiError;
use crate::handlers::users::ScimState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkRequest {
	pub schemas: Vec<String>,
	#[serde(rename = "Operations")]
	pub operations: Vec<BulkOperation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOperation {
	pub method: String,
	pub path: String,
	#[serde(default)]
	pub bulk_id: Option<String>,
	#[serde(default)]
	pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkResponse {
	pub schemas: Vec<String>,
	#[serde(rename = "Operations")]
	pub operations: Vec<BulkOperationResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkOperationResponse {
	pub method: String,
	pub bulk_id: Option<String>,
	pub status: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub location: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub response: Option<serde_json::Value>,
}

pub async fn bulk_operations(
	State(_state): State<ScimState>,
	Json(request): Json<BulkRequest>,
) -> Result<Json<BulkResponse>, ScimApiError> {
	if request.operations.len() > 1000 {
		return Err(ScimApiError::Scim(ScimError::TooMany));
	}

	let mut responses = Vec::new();

	for op in request.operations {
		responses.push(BulkOperationResponse {
			method: op.method.clone(),
			bulk_id: op.bulk_id.clone(),
			status: "501".to_string(),
			location: None,
			response: Some(serde_json::json!({
				"detail": "Bulk operation processing not yet implemented"
			})),
		});
	}

	Ok(Json(BulkResponse {
		schemas: vec!["urn:ietf:params:scim:api:messages:2.0:BulkResponse".to_string()],
		operations: responses,
	}))
}
