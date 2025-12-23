// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Loom thread persistence server binary.

use clap::{Parser, Subcommand};
use loom_server::{create_app_state, create_router, ServerConfig, ThreadRepository};
use std::sync::Arc;
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
	let state = create_app_state(repo);
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
	axum::serve(listener, app).await?;

	Ok(())
}
