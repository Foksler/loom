// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Loom thread persistence server binary.

use clap::{Parser, Subcommand};
use loom_jobs::{JobRepository, JobScheduler};
use loom_server::{create_app_state, create_router, ServerConfig, ThreadRepository};
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
	cors::{Any, CorsLayer},
	trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod version;

/// Loom server - HTTP server for Loom thread persistence.
#[derive(Parser, Debug)]
#[command(
	name = "loom-server",
	about = "Loom thread persistence server",
	version
)]
struct Args {
	/// Subcommands for loom-server (e.g., `version`)
	#[command(subcommand)]
	command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
	/// Show version and build information
	Version,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Parse CLI arguments
	let args = Args::parse();

	// Handle subcommands that should not start the server
	if let Some(Command::Version) = args.command {
		println!("{}", version::format_version_info());
		return Ok(());
	}

	// Load .env file if present
	dotenvy::dotenv().ok();

	// Load configuration
	let config = ServerConfig::from_env()?;

	// Setup tracing
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| config.log_level.clone().into()),
		)
		.with(tracing_subscriber::fmt::layer())
		.init();

	tracing::info!(
			host = %config.host,
			port = config.port,
			database = %config.database_url,
			"starting loom-server"
	);

	// Create database repository
	let repo = Arc::new(ThreadRepository::new(&config.database_url).await?);

	// Create application state and router with middleware
	let pool = repo.pool().clone();
	let mut state = create_app_state(pool.clone(), repo, &config).await;

	// Weaver provisioner startup lifecycle - validate namespace
	if let Some(ref provisioner) = state.provisioner {
		if let Err(e) = provisioner.validate_namespace().await {
			tracing::error!(error = %e, "Weaver provisioner namespace validation failed");
			tracing::warn!("Continuing without weaver provisioning support");
		}
	}

	// Create job repository and scheduler
	let job_repo = Arc::new(JobRepository::new(pool.clone()));
	let mut scheduler = JobScheduler::new(Arc::clone(&job_repo));

	// Register weaver cleanup job if provisioner is enabled
	if let Some(ref provisioner) = state.provisioner {
		use loom_server::jobs::WeaverCleanupJob;
		scheduler.register_periodic(
			Arc::new(WeaverCleanupJob::new(Arc::clone(provisioner))),
			Duration::from_secs(config.weaver_cleanup_interval_secs),
		);
	}

	// Register session cleanup job
	{
		use loom_server::jobs::SessionCleanupJob;
		scheduler.register_periodic(
			Arc::new(SessionCleanupJob::new(pool.clone())),
			Duration::from_secs(config.session_cleanup_interval_secs),
		);
	}

	// Register OAuth state cleanup job
	{
		use loom_server::jobs::OAuthStateCleanupJob;
		scheduler.register_periodic(
			Arc::new(OAuthStateCleanupJob::new(Arc::clone(&state.oauth_state_store))),
			Duration::from_secs(config.oauth_state_cleanup_interval_secs),
		);
	}

	// Register job history cleanup job
	{
		use loom_server::jobs::JobHistoryCleanupJob;
		scheduler.register_periodic(
			Arc::new(JobHistoryCleanupJob::new(Arc::clone(&job_repo), config.job_history_retention_days)),
			Duration::from_secs(24 * 60 * 60), // Daily
		);
	}

	// Register token refresh job if LLM service has OAuth pool
	if let Some(ref llm_service) = state.llm_service {
		if llm_service.is_anthropic_oauth_pool() {
			use loom_server::jobs::TokenRefreshJob;
			scheduler.register_periodic(
				Arc::new(TokenRefreshJob::new(Arc::clone(llm_service))),
				Duration::from_secs(300), // 5 minutes
			);
		}
	}

	let scheduler = Arc::new(scheduler);

	// Update state with scheduler and repository
	state.job_scheduler = Some(Arc::clone(&scheduler));
	state.job_repository = Some(Arc::clone(&job_repo));

	// Start job scheduler
	if let Err(e) = scheduler.start().await {
		tracing::error!(error = %e, "Failed to start job scheduler");
	}

	let app = create_router(state)
		.layer(TraceLayer::new_for_http())
		.layer(
			CorsLayer::new()
				.allow_origin(Any)
				.allow_methods(Any)
				.allow_headers(Any),
		);

	// Start server
	let addr = config.socket_addr();
	tracing::info!("listening on {}", addr);

	let listener = tokio::net::TcpListener::bind(&addr).await?;

	// Run server with graceful shutdown
	tokio::select! {
		result = axum::serve(listener, app) => {
			if let Err(e) = result {
				tracing::error!(error = %e, "Server error");
			}
		}
		_ = tokio::signal::ctrl_c() => {
			tracing::info!("Received shutdown signal");
			tracing::info!("Shutting down job scheduler...");
			scheduler.shutdown().await;
		}
	}

	tracing::info!("Server shutdown complete");
	Ok(())
}
