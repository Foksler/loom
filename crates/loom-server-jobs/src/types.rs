// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
	Running,
	Succeeded,
	Failed,
	Cancelled,
}

impl JobStatus {
	pub fn as_str(&self) -> &'static str {
		match self {
			JobStatus::Running => "running",
			JobStatus::Succeeded => "succeeded",
			JobStatus::Failed => "failed",
			JobStatus::Cancelled => "cancelled",
		}
	}
}

impl std::str::FromStr for JobStatus {
	type Err = String;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"running" => Ok(JobStatus::Running),
			"succeeded" => Ok(JobStatus::Succeeded),
			"failed" => Ok(JobStatus::Failed),
			"cancelled" => Ok(JobStatus::Cancelled),
			_ => Err(format!("unknown job status: {s}")),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
	Schedule,
	Manual,
	Retry,
}

impl TriggerSource {
	pub fn as_str(&self) -> &'static str {
		match self {
			TriggerSource::Schedule => "schedule",
			TriggerSource::Manual => "manual",
			TriggerSource::Retry => "retry",
		}
	}
}

impl std::str::FromStr for TriggerSource {
	type Err = String;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"schedule" => Ok(TriggerSource::Schedule),
			"manual" => Ok(TriggerSource::Manual),
			"retry" => Ok(TriggerSource::Retry),
			_ => Err(format!("unknown trigger source: {s}")),
		}
	}
}

#[derive(Debug, Clone)]
pub enum JobType {
	Periodic { interval: Duration },
	OneShot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
	pub id: String,
	pub name: String,
	pub description: String,
	pub job_type: String,
	pub interval_secs: Option<i64>,
	pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRun {
	pub id: String,
	pub job_id: String,
	pub status: JobStatus,
	pub started_at: DateTime<Utc>,
	pub completed_at: Option<DateTime<Utc>>,
	pub duration_ms: Option<i64>,
	pub error_message: Option<String>,
	pub retry_count: u32,
	pub triggered_by: TriggerSource,
	pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOutput {
	pub message: String,
	pub metadata: Option<serde_json::Value>,
}
