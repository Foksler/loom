// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! K8s client abstraction for Loom agent provisioning.
//!
//! This crate provides:
//! - A trait-based K8s client abstraction for testability
//! - Production implementation using the kube crate
//! - Common types for pod management and log streaming

mod client;
mod error;
mod kube_client;
mod types;

pub use client::K8sClient;
pub use error::{K8sError, K8sResult};
pub use kube_client::KubeClient;
pub use types::{
	AttachedProcess, Container, ContainerPort, EnvVar, LocalObjectReference, LogOptions, LogStream,
	Namespace, Pod, PodSpec, PodStatus, ResourceRequirements, SecurityContext,
};
