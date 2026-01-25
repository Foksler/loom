// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git HTTP Smart Protocol router.

use axum::{
	body::Bytes,
	routing::{get, post},
};

use super::clips::{clips_info_refs, clips_receive_pack, clips_upload_pack};
use super::handlers::{git_wildcard_handler, info_refs, receive_pack, upload_pack};

pub fn router() -> crate::OptionalAuthRouter {
	crate::OptionalAuthRouter::new()
		// Repository git routes
		.route("/git/{owner}/{repo}/info/refs", get(info_refs))
		.route("/git/{owner}/{repo}/git-upload-pack", post(upload_pack))
		.route("/git/{owner}/{repo}/git-receive-pack", post(receive_pack))
		// Clips git routes
		.route("/git/clips/{owner}/{name}/info/refs", get(clips_info_refs))
		.route(
			"/git/clips/{owner}/{name}/git-upload-pack",
			post(clips_upload_pack),
		)
		.route(
			"/git/clips/{owner}/{name}/git-receive-pack",
			post(clips_receive_pack),
		)
		.route(
			"/git/mirrors/{*path}",
			get(|path, query, auth, state, headers, body| {
				git_wildcard_handler(
					path,
					axum::http::Method::GET,
					Some(query),
					auth,
					state,
					headers,
					body,
				)
			})
			.post(|path, auth, state, headers, body: Bytes| async move {
				git_wildcard_handler(
					path,
					axum::http::Method::POST,
					None,
					auth,
					state,
					headers,
					body,
				)
				.await
			}),
		)
}
