// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;

pub use k8s_openapi::api::core::v1::{
	Container, ContainerPort, EnvVar, Namespace, Pod, PodSpec, PodStatus, ResourceRequirements,
	SecurityContext,
};

/// Options for log streaming.
#[derive(Debug, Clone, Default)]
pub struct LogOptions {
	pub tail: u32,
	pub timestamps: bool,
}

/// A pinned stream of log lines from a container.
pub type LogStream = Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>;
