// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Device code flow for CLI/VS Code authentication.

use axum::{
	extract::State,
	http::{HeaderMap, StatusCode},
	response::IntoResponse,
	Json,
};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_auth::{generate_access_token, SessionType};
use loom_server_auth_devicecode::{DeviceCode, DEVICE_CODE_EXPIRY_MINUTES};
use uuid::Uuid;

use crate::{api::AppState, auth_middleware::RequireAuth, i18n::resolve_user_locale, i18n::t};

use super::common::{
	resolve_locale_from_headers, AuthErrorResponse, DeviceCodeCompleteRequest,
	DeviceCodeCompleteResponse, DeviceCodePollRequest, DeviceCodePollResponse,
	DeviceCodeStartResponse,
};

#[utoipa::path(
    post,
    path = "/auth/device/start",
    responses(
        (status = 200, description = "Device code flow started", body = DeviceCodeStartResponse),
        (status = 500, description = "Internal error", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Starts the device code flow for CLI/VS Code authentication.
///
/// Returns a device code for polling and a user code for the user to enter
/// at the verification URL. The CLI polls with the device code while the
/// user authenticates in a browser with the user code.
///
/// # Security
/// Device codes expire after [`DEVICE_CODE_EXPIRY_MINUTES`].
/// Never log device_code or user_code as they grant authentication.
#[tracing::instrument(skip(state))]
pub async fn device_start(State(state): State<AppState>) -> impl IntoResponse {
	let locale = state.default_locale.as_str();
	let device_code = DeviceCode::new();
	tracing::info!("Device code flow started");

	if let Err(e) = state
		.session_repo
		.create_device_code(&device_code.device_code, &device_code.user_code)
		.await
	{
		tracing::error!(error = %e, "Failed to store device code");
		return (
			StatusCode::INTERNAL_SERVER_ERROR,
			Json(AuthErrorResponse {
				error: "internal_error".to_string(),
				message: t(locale, "server.api.error.internal").to_string(),
			}),
		)
			.into_response();
	}

	let verification_url = format!("{}/device", state.base_url);

	state
		.audit_service
		.log(AuditLogBuilder::new(AuditEventType::DeviceCodeStarted).build());

	Json(DeviceCodeStartResponse {
		device_code: device_code.device_code,
		user_code: device_code.user_code,
		verification_url,
		expires_in: DEVICE_CODE_EXPIRY_MINUTES * 60,
	})
	.into_response()
}

#[utoipa::path(
    post,
    path = "/auth/device/poll",
    request_body = DeviceCodePollRequest,
    responses(
        (status = 200, description = "Device code status", body = DeviceCodePollResponse),
        (status = 400, description = "Invalid device code", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Polls the status of a device code authentication flow.
///
/// CLI calls this endpoint repeatedly until the user completes authentication
/// in the browser or the device code expires. Returns `pending`, `completed`
/// (with access token), or `expired`.
///
/// # Security
/// The device_code in the request body is a secret. Never log it.
/// Access tokens are only returned once on completion.
#[tracing::instrument(skip(state, payload))]
pub async fn device_poll(
	State(state): State<AppState>,
	Json(payload): Json<DeviceCodePollRequest>,
) -> impl IntoResponse {
	let locale = state.default_locale.as_str();
	if Uuid::parse_str(&payload.device_code).is_err() {
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "invalid_device_code".to_string(),
				message: t(locale, "server.api.auth.invalid_device_code").to_string(),
			}),
		)
			.into_response();
	}

	let device_code_result = match state
		.session_repo
		.get_device_code(&payload.device_code)
		.await
	{
		Ok(result) => result,
		Err(e) => {
			tracing::error!(error = %e, "Failed to get device code");
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AuthErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response();
		}
	};

	let Some((_user_code, user_id, is_completed)) = device_code_result else {
		return Json(DeviceCodePollResponse::Expired).into_response();
	};

	if is_completed {
		if let Some(user_id) = user_id {
			let (plaintext_token, token_hash) = generate_access_token();

			if let Err(e) = state
				.session_repo
				.create_access_token(&user_id, &token_hash, "device-code-auth", SessionType::Cli)
				.await
			{
				tracing::error!(error = %e, user_id = %user_id, "Failed to create access token");
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(AuthErrorResponse {
						error: "internal_error".to_string(),
						message: t(locale, "server.api.error.internal").to_string(),
					}),
				)
					.into_response();
			}

			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::DeviceCodeCompleted)
					.actor(AuditUserId::new(user_id.into_inner()))
					.details(serde_json::json!({
						"session_type": "cli",
					}))
					.build(),
			);

			tracing::info!(user_id = %user_id, "Device code authentication completed");
			return Json(DeviceCodePollResponse::Completed {
				access_token: plaintext_token,
			})
			.into_response();
		}
	}

	Json(DeviceCodePollResponse::Pending).into_response()
}

#[utoipa::path(
    post,
    path = "/auth/device/complete",
    request_body = DeviceCodeCompleteRequest,
    responses(
        (status = 200, description = "Device code completed", body = DeviceCodeCompleteResponse),
        (status = 400, description = "Invalid or expired user code", body = AuthErrorResponse),
        (status = 401, description = "Not authenticated", body = AuthErrorResponse)
    ),
    tag = "auth"
)]
/// Completes device code flow by linking it to the authenticated user.
///
/// Called by the web UI after the user logs in and enters the user code.
/// Marks the device code as completed with the current user's ID, allowing
/// the polling CLI to receive an access token.
///
/// # Security
/// Requires authentication. User codes should not be logged.
///
/// # Errors
/// - 400 Bad Request: Invalid or expired user code
/// - 401 Unauthorized: Not authenticated
#[tracing::instrument(skip(state, current_user, payload))]
pub async fn device_complete(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Json(payload): Json<DeviceCodeCompleteRequest>,
) -> impl IntoResponse {
	let locale = resolve_user_locale(&current_user, &state.default_locale);
	let user_code = payload.user_code.trim();

	if user_code.is_empty() {
		return (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "invalid_user_code".to_string(),
				message: t(locale, "server.api.auth.invalid_user_code").to_string(),
			}),
		)
			.into_response();
	}

	match state
		.session_repo
		.complete_device_code(user_code, &current_user.user.id)
		.await
	{
		Ok(true) => {
			// SECURITY: Don't log user_code as it's a secret
			tracing::info!(user_id = %current_user.user.id, "Device code completed by user");

			state.audit_service.log(
				AuditLogBuilder::new(AuditEventType::DeviceCodeCompleted)
					.actor(AuditUserId::new(current_user.user.id.into_inner()))
					.details(serde_json::json!({
						"step": "user_authorization",
					}))
					.build(),
			);

			Json(DeviceCodeCompleteResponse {
				message: t(locale, "server.api.auth.device_authorized").to_string(),
			})
			.into_response()
		}
		Ok(false) => (
			StatusCode::BAD_REQUEST,
			Json(AuthErrorResponse {
				error: "invalid_user_code".to_string(),
				message: t(locale, "server.api.auth.invalid_user_code").to_string(),
			}),
		)
			.into_response(),
		Err(e) => {
			tracing::error!(error = %e, "Failed to complete device code");
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(AuthErrorResponse {
					error: "internal_error".to_string(),
					message: t(locale, "server.api.error.internal").to_string(),
				}),
			)
				.into_response()
		}
	}
}

#[utoipa::path(
    get,
    path = "/auth/device",
    responses(
        (status = 200, description = "Device code entry page")
    ),
    tag = "auth"
)]
/// Serves the device code entry page.
///
/// Returns HTML for the device code entry page where users enter the code
/// displayed by CLI/VS Code to complete authentication.
#[tracing::instrument(skip(headers, state))]
pub async fn device_page(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
	let locale = resolve_locale_from_headers(&headers, &state.default_locale);
	let dir = if loom_common_i18n::is_rtl(locale) {
		"rtl"
	} else {
		"ltr"
	};

	let title = loom_common_i18n::t(locale, "server.auth.device.title");
	let heading = loom_common_i18n::t(locale, "server.auth.device.heading");
	let subtitle = loom_common_i18n::t(locale, "server.auth.device.subtitle");
	let code_label = loom_common_i18n::t(locale, "server.auth.device.code_label");
	let submit = loom_common_i18n::t(locale, "server.auth.device.submit");
	let authorizing = loom_common_i18n::t(locale, "server.auth.device.authorizing");
	let authorized = loom_common_i18n::t(locale, "server.auth.device.authorized");
	let expiry_help = loom_common_i18n::t(locale, "server.auth.device.expiry_help");
	let success_msg = loom_common_i18n::t(locale, "server.auth.device.success");
	let invalid_format = loom_common_i18n::t(locale, "server.auth.device.invalid_format");
	let error_msg = loom_common_i18n::t(locale, "server.auth.device.error");
	let auth_failed = loom_common_i18n::t(locale, "server.auth.device.auth_failed");

	let html = format!(
		r#"<!DOCTYPE html>
<html lang="{locale}" dir="{dir}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <style>
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: #f5f5f5;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }}
        .container {{
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
            padding: 40px;
            max-width: 400px;
            width: 100%;
        }}
        h1 {{ font-size: 24px; font-weight: 600; color: #1a1a1a; margin-bottom: 8px; text-align: center; }}
        .subtitle {{ color: #666; font-size: 14px; text-align: center; margin-bottom: 32px; }}
        .form-group {{ margin-bottom: 24px; }}
        label {{ display: block; font-size: 14px; font-weight: 500; color: #333; margin-bottom: 8px; }}
        input[type="text"] {{
            width: 100%;
            padding: 12px 16px;
            font-size: 18px;
            font-family: monospace;
            letter-spacing: 2px;
            text-align: center;
            border: 1px solid #ddd;
            border-radius: 6px;
            outline: none;
            transition: border-color 0.2s;
        }}
        input[type="text"]:focus {{ border-color: #0066cc; }}
        input[type="text"]::placeholder {{ color: #bbb; letter-spacing: 2px; }}
        button {{
            width: 100%;
            padding: 14px;
            font-size: 16px;
            font-weight: 500;
            color: white;
            background: #0066cc;
            border: none;
            border-radius: 6px;
            cursor: pointer;
            transition: background 0.2s;
        }}
        button:hover {{ background: #0052a3; }}
        button:disabled {{ background: #ccc; cursor: not-allowed; }}
        .message {{ padding: 12px 16px; border-radius: 6px; font-size: 14px; margin-bottom: 24px; text-align: center; }}
        .message.success {{ background: #e6f4ea; color: #1e7e34; border: 1px solid #b7dfb9; }}
        .message.error {{ background: #fce8e6; color: #c5221f; border: 1px solid #f5c6c4; }}
        .message.hidden {{ display: none; }}
        .help-text {{ font-size: 12px; color: #888; text-align: center; margin-top: 24px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>{heading}</h1>
        <p class="subtitle">{subtitle}</p>
        <div id="message" class="message hidden"></div>
        <form id="deviceForm">
            <div class="form-group">
                <label for="userCode">{code_label}</label>
                <input type="text" id="userCode" name="user_code" placeholder="XXX-XXX-XXX"
                    maxlength="11" autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false" required>
            </div>
            <button type="submit" id="submitBtn">{submit}</button>
        </form>
        <p class="help-text">{expiry_help}</p>
    </div>
    <script>
        const MSGS = {{
            authorizing: "{authorizing}",
            authorized: "{authorized}",
            submit: "{submit}",
            success: "{success_msg}",
            invalidFormat: "{invalid_format}",
            error: "{error_msg}",
            authFailed: "{auth_failed}"
        }};
        const form = document.getElementById('deviceForm');
        const input = document.getElementById('userCode');
        const submitBtn = document.getElementById('submitBtn');
        const messageDiv = document.getElementById('message');
        input.addEventListener('input', function(e) {{
            let value = e.target.value.replace(/[^0-9]/g, '');
            if (value.length > 9) value = value.slice(0, 9);
            let formatted = '';
            for (let i = 0; i < value.length; i++) {{
                if (i > 0 && i % 3 === 0) formatted += '-';
                formatted += value[i];
            }}
            e.target.value = formatted;
        }});
        function showMessage(text, isError) {{
            messageDiv.textContent = text;
            messageDiv.className = 'message ' + (isError ? 'error' : 'success');
        }}
        form.addEventListener('submit', async function(e) {{
            e.preventDefault();
            const userCode = input.value.trim();
            if (!/^\d{{3}}-\d{{3}}-\d{{3}}$/.test(userCode)) {{
                showMessage(MSGS.invalidFormat, true);
                return;
            }}
            submitBtn.disabled = true;
            submitBtn.textContent = MSGS.authorizing;
            messageDiv.className = 'message hidden';
            try {{
                const response = await fetch('/auth/device/complete', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ user_code: userCode }}),
                    credentials: 'include'
                }});
                const data = await response.json();
                if (response.ok) {{
                    showMessage(MSGS.success, false);
                    input.disabled = true;
                    submitBtn.textContent = MSGS.authorized;
                }} else {{
                    showMessage(data.message || MSGS.authFailed, true);
                    submitBtn.disabled = false;
                    submitBtn.textContent = MSGS.submit;
                }}
            }} catch (err) {{
                showMessage(MSGS.error, true);
                submitBtn.disabled = false;
                submitBtn.textContent = MSGS.submit;
            }}
        }});
        input.focus();
    </script>
</body>
</html>"#
	);

	(
		StatusCode::OK,
		[("Content-Type", "text/html; charset=utf-8")],
		html,
	)
}
