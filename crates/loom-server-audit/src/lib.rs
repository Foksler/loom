// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

pub mod config;
pub mod enrichment;
pub mod error;
pub mod event;
pub mod filter;
pub mod pipeline;
pub mod sink;

pub use config::{AuditConfig, QueueConfig, QueueOverflowPolicy};
pub use enrichment::{
	AuditEnricher, EnrichedAuditEvent, GeoIpInfo, NoopEnricher, OrgContext, SessionContext,
};
pub use event::{
	AuditEventType, AuditLogBuilder, AuditLogEntry, AuditSeverity, UserId,
	DEFAULT_AUDIT_RETENTION_DAYS,
};
pub use error::{AuditError, AuditResult, AuditSinkError};
pub use filter::AuditFilterConfig;
pub use pipeline::AuditService;
pub use sink::AuditSink;

#[cfg(feature = "sink-sqlite")]
pub use sink::sqlite::SqliteAuditSink;

#[cfg(feature = "sink-tracing")]
pub use sink::tracing::TracingAuditSink;

#[cfg(feature = "sink-http")]
pub use sink::http::{HttpAuditSink, HttpSinkConfig};

#[cfg(feature = "sink-json-stream")]
pub use sink::json_stream::{JsonStreamConfig, JsonStreamSink, StreamProtocol};
