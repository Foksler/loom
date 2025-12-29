// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Prometheus metrics for agent provisioning.

use prometheus::{Counter, Gauge};

/// Prometheus metrics for agent provisioning operations.
pub struct AgentMetrics {
	/// Total number of agents created.
	pub agents_created_total: Counter,
	/// Total number of agents deleted.
	pub agents_deleted_total: Counter,
	/// Total number of agents that failed to start.
	pub agents_failed_total: Counter,
	/// Total number of cleanup runs completed.
	pub agents_cleanup_total: Counter,
	/// Total number of agents deleted by cleanup.
	pub agents_cleanup_deleted_total: Counter,
	/// Current number of active agents.
	pub agents_active: Gauge,
}

impl Default for AgentMetrics {
	fn default() -> Self {
		Self::new()
	}
}

impl AgentMetrics {
	/// Create new agent metrics and register them with the default registry.
	pub fn new() -> Self {
		let agents_created_total = Counter::new(
			"loom_agents_created_total",
			"Total number of agents provisioned",
		)
		.expect("Failed to create agents_created_total counter");

		let agents_deleted_total = Counter::new(
			"loom_agents_deleted_total",
			"Total number of agents deleted (manual + cleanup)",
		)
		.expect("Failed to create agents_deleted_total counter");

		let agents_failed_total = Counter::new(
			"loom_agents_failed_total",
			"Total number of agents that entered failed state",
		)
		.expect("Failed to create agents_failed_total counter");

		let agents_cleanup_total = Counter::new(
			"loom_agents_cleanup_total",
			"Total number of cleanup runs completed",
		)
		.expect("Failed to create agents_cleanup_total counter");

		let agents_cleanup_deleted_total = Counter::new(
			"loom_agents_cleanup_deleted_total",
			"Total number of agents deleted by cleanup",
		)
		.expect("Failed to create agents_cleanup_deleted_total counter");

		let agents_active =
			Gauge::new("loom_agents_active", "Current number of running agents")
				.expect("Failed to create agents_active gauge");

		let registry = prometheus::default_registry();

		registry
			.register(Box::new(agents_created_total.clone()))
			.ok();
		registry
			.register(Box::new(agents_deleted_total.clone()))
			.ok();
		registry
			.register(Box::new(agents_failed_total.clone()))
			.ok();
		registry
			.register(Box::new(agents_cleanup_total.clone()))
			.ok();
		registry
			.register(Box::new(agents_cleanup_deleted_total.clone()))
			.ok();
		registry.register(Box::new(agents_active.clone())).ok();

		Self {
			agents_created_total,
			agents_deleted_total,
			agents_failed_total,
			agents_cleanup_total,
			agents_cleanup_deleted_total,
			agents_active,
		}
	}

	/// Increment the created counter.
	pub fn inc_created(&self) {
		self.agents_created_total.inc();
	}

	/// Increment the deleted counter.
	pub fn inc_deleted(&self) {
		self.agents_deleted_total.inc();
	}

	/// Increment the failed counter.
	pub fn inc_failed(&self) {
		self.agents_failed_total.inc();
	}

	/// Increment the cleanup counter.
	pub fn inc_cleanup(&self) {
		self.agents_cleanup_total.inc();
	}

	/// Increment the cleanup deleted counter by the given amount.
	pub fn inc_cleanup_deleted(&self, count: u64) {
		self.agents_cleanup_deleted_total.inc_by(count as f64);
	}

	/// Set the active agents gauge.
	pub fn set_active(&self, count: u64) {
		self.agents_active.set(count as f64);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_agent_metrics_creation() {
		let metrics = AgentMetrics::new();
		assert_eq!(metrics.agents_created_total.get(), 0.0);
		assert_eq!(metrics.agents_active.get(), 0.0);
	}

	#[test]
	fn test_inc_created() {
		let metrics = AgentMetrics::new();
		metrics.inc_created();
		assert_eq!(metrics.agents_created_total.get(), 1.0);
	}

	#[test]
	fn test_set_active() {
		let metrics = AgentMetrics::new();
		metrics.set_active(5);
		assert_eq!(metrics.agents_active.get(), 5.0);
	}
}
