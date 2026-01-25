// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! WireGuard tunnel HTTP handlers.
//!
//! Public API routes (require user auth):
//! - `POST /api/wg/devices` - Register a new device
//! - `GET /api/wg/devices` - List user's devices
//! - `DELETE /api/wg/devices/{id}` - Revoke a device
//! - `POST /api/wg/sessions` - Create tunnel session
//! - `GET /api/wg/sessions` - List active sessions
//! - `DELETE /api/wg/sessions/{id}` - Terminate session
//! - `GET /api/wg/derp-map` - Get DERP map
//!
//! Internal API routes (require SVID auth):
//! - `POST /internal/wg/weavers` - Register weaver's WG key
//! - `DELETE /internal/wg/weavers/{id}` - Unregister weaver
//! - `GET /internal/wg/weavers/{id}/peers` - SSE stream of peer updates

pub mod common;
pub mod derp;
pub mod devices;
pub mod internal;
pub mod sessions;

// Re-export handlers
pub use derp::{__path_get_derp_map, get_derp_map};
pub use devices::{
	__path_list_devices, __path_register_device, __path_revoke_device, list_devices,
	register_device, revoke_device,
};
pub use internal::{
	__path_get_weaver, __path_register_weaver, __path_stream_peers, __path_unregister_weaver,
	get_weaver, register_weaver, stream_peers, unregister_weaver,
};
pub use sessions::{
	__path_create_session, __path_list_sessions, __path_terminate_session, create_session,
	list_sessions, terminate_session,
};
