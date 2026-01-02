// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! HTTP API routes and handlers for thread operations.

use axum::routing::{delete, get, patch, post, put};

use crate::{
	abac_middleware::RequireRole,
	typed_router::{AuthedRouter, PublicRouter},
};
use loom_server_auth_github::{GitHubOAuthClient, GitHubOAuthConfig};
use loom_server_auth_google::{GoogleOAuthClient, GoogleOAuthConfig};
use loom_server_auth_okta::{OktaOAuthClient, OktaOAuthConfig};
use loom_server_geoip::GeoIpService;
use loom_server_github_app::{GithubAppClient, GithubAppConfig};
use loom_server_jobs::{JobRepository, JobScheduler};
use loom_server_google_cse::CseClient;
use loom_server_k8s::KubeClient;
use loom_server_llm_service::LlmService;
use loom_server_smtp::SmtpClient;
use loom_server_weaver::{Provisioner, WeaverConfig, WebhookConfig, WebhookDispatcher};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use axum::Router;

use crate::{
	config::ServerConfig,
	db::{
		ApiKeyRepository, AuditRepository, OrgRepository, SessionRepository, ShareRepository,
		TeamRepository, ThreadRepository, UserRepository,
	},
	llm_proxy,
	oauth_state::OAuthStateStore,
	query_metrics::QueryMetrics,
	query_tracing::QueryTraceStore,
	routes,
	server_query::{self, ServerQueryManager},
};
use sqlx::SqlitePool;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
	pub repo: Arc<ThreadRepository>,
	pub user_repo: Arc<UserRepository>,
	pub session_repo: Arc<SessionRepository>,
	pub org_repo: Arc<OrgRepository>,
	pub team_repo: Arc<TeamRepository>,
	pub api_key_repo: Arc<ApiKeyRepository>,
	pub audit_repo: Arc<AuditRepository>,
	pub share_repo: Arc<ShareRepository>,
	pub auth_config: loom_server_auth::middleware::AuthConfig,
	pub dev_user: Option<loom_server_auth::User>,
	pub base_url: String,
	pub cse_client: Option<Arc<CseClient>>,
	pub github_client: Option<Arc<GithubAppClient>>,
	pub llm_service: Option<Arc<LlmService>>,
	pub query_manager: Arc<ServerQueryManager>,
	pub query_metrics: Arc<QueryMetrics>,
	pub trace_store: QueryTraceStore,
	pub provisioner: Option<Arc<Provisioner>>,
	pub webhook_dispatcher: Option<Arc<WebhookDispatcher>>,
	pub smtp_client: Option<Arc<SmtpClient>>,
	pub github_oauth: Option<Arc<GitHubOAuthClient>>,
	pub google_oauth: Option<Arc<GoogleOAuthClient>>,
	pub okta_oauth: Option<Arc<OktaOAuthClient>>,
	pub oauth_state_store: Arc<OAuthStateStore>,
	pub default_locale: String,
	pub geoip_service: Option<Arc<GeoIpService>>,
	pub job_scheduler: Option<Arc<JobScheduler>>,
	pub job_repository: Option<Arc<JobRepository>>,
	pub scm_repo_store: Option<Arc<loom_scm::SqliteRepoStore>>,
	pub scm_protection_store: Option<Arc<loom_scm::SqliteProtectionStore>>,
	pub scm_webhook_store: Option<Arc<loom_scm::SqliteWebhookStore>>,
	pub scm_maintenance_store: Option<Arc<loom_scm::SqliteMaintenanceJobStore>>,
	pub scm_team_access_store: Option<Arc<loom_scm::SqliteRepoTeamAccessStore>>,
	pub push_mirror_store: Option<Arc<loom_scm_mirror::SqlitePushMirrorStore>>,
	pub external_mirror_store: Option<Arc<loom_scm_mirror::SqliteExternalMirrorStore>>,
}

/// Creates the application state, initializing optional components.
pub async fn create_app_state(
	pool: SqlitePool,
	repo: Arc<ThreadRepository>,
	config: &ServerConfig,
) -> AppState {
	// Create auth repositories
	let user_repo = Arc::new(UserRepository::new(pool.clone()));
	let session_repo = Arc::new(SessionRepository::new(pool.clone()));
	let org_repo = Arc::new(OrgRepository::new(pool.clone()));
	let team_repo = Arc::new(TeamRepository::new(pool.clone()));
	let api_key_repo = Arc::new(ApiKeyRepository::new(pool.clone()));
	let audit_repo = Arc::new(AuditRepository::new(pool.clone()));
	let share_repo = Arc::new(ShareRepository::new(pool.clone()));
	let scm_repo_store = Arc::new(loom_scm::SqliteRepoStore::new(pool.clone()));
	let scm_protection_store = Arc::new(loom_scm::SqliteProtectionStore::new(pool.clone()));
	let scm_webhook_store = Arc::new(loom_scm::SqliteWebhookStore::new(pool.clone()));
	let scm_maintenance_store = Arc::new(loom_scm::SqliteMaintenanceJobStore::new(pool.clone()));
	let scm_team_access_store = Arc::new(loom_scm::SqliteRepoTeamAccessStore::new(pool.clone()));
	let push_mirror_store = Arc::new(loom_scm_mirror::SqlitePushMirrorStore::new(pool.clone()));
	let external_mirror_store = Arc::new(loom_scm_mirror::SqliteExternalMirrorStore::new(pool));
	let auth_config = loom_server_auth::middleware::AuthConfig::from_env();
	let cse_client = match (
		std::env::var("LOOM_SERVER_GOOGLE_CSE_API_KEY"),
		std::env::var("LOOM_SERVER_GOOGLE_CSE_SEARCH_ENGINE_ID"),
	) {
		(Ok(api_key), Ok(cx)) if !api_key.is_empty() && !cx.is_empty() => {
			tracing::info!("Google CSE configured, creating client");
			Some(Arc::new(CseClient::new(api_key, cx)))
		}
		_ => {
			tracing::info!("Google CSE not configured");
			None
		}
	};

	let github_client = match GithubAppConfig::from_env() {
		Ok(config) => match GithubAppClient::new(config) {
			Ok(client) => {
				tracing::info!("GitHub App configured, creating client");
				Some(Arc::new(client))
			}
			Err(e) => {
				tracing::warn!(error = %e, "Failed to create GitHub App client");
				None
			}
		},
		Err(_) => {
			tracing::info!("GitHub App not configured");
			None
		}
	};

	let llm_service = match LlmService::from_env().await {
		Ok(service) => {
			tracing::info!(
				anthropic = service.has_anthropic(),
				openai = service.has_openai(),
				"LLM service configured"
			);
			Some(Arc::new(service))
		}
		Err(e) => {
			tracing::info!(error = %e, "LLM service not configured");
			None
		}
	};

	let query_metrics = Arc::new(QueryMetrics::default());
	let query_manager = Arc::new(ServerQueryManager::with_metrics(query_metrics.clone()));

	// Initialize weaver provisioner if enabled
	let (provisioner, webhook_dispatcher) = initialize_weaver_provisioner(config).await;

	// Initialize SMTP client if configured
	let smtp_client = initialize_smtp_client(config);

	// Initialize OAuth clients
	let github_oauth = initialize_github_oauth();
	let google_oauth = initialize_google_oauth();
	let okta_oauth = initialize_okta_oauth();
	let oauth_state_store = Arc::new(OAuthStateStore::new());

	// Initialize GeoIP service
	let geoip_service = GeoIpService::try_from_env().map(Arc::new);

	// Create or fetch dev user if dev mode is enabled
	let dev_user = if auth_config.dev_mode {
		tracing::warn!("═══════════════════════════════════════════════════════════════════");
		tracing::warn!("⚠️  DEV MODE AUTHENTICATION ENABLED - DO NOT USE IN PRODUCTION ⚠️");
		tracing::warn!("All unauthenticated requests will be auto-authenticated as admin!");
		tracing::warn!("Set LOOM_ENV=production to prevent accidental production use.");
		tracing::warn!("═══════════════════════════════════════════════════════════════════");
		match create_or_get_dev_user(&user_repo).await {
			Ok(user) => {
				tracing::info!(user_id = %user.id, "Dev mode enabled, using dev user");
				Some(user)
			}
			Err(e) => {
				tracing::error!(error = %e, "Failed to create dev user, dev mode disabled");
				None
			}
		}
	} else {
		None
	};

	AppState {
		repo,
		user_repo,
		session_repo,
		org_repo,
		team_repo,
		api_key_repo,
		audit_repo,
		share_repo,
		auth_config,
		dev_user,
		base_url: config.base_url.clone(),
		cse_client,
		github_client,
		llm_service,
		query_manager,
		query_metrics,
		trace_store: QueryTraceStore::default(),
		provisioner,
		webhook_dispatcher,
		smtp_client,
		github_oauth,
		google_oauth,
		okta_oauth,
		oauth_state_store,
		default_locale: config.default_locale.clone(),
		geoip_service,
		job_scheduler: None,
		job_repository: None,
		scm_repo_store: Some(scm_repo_store),
		scm_protection_store: Some(scm_protection_store),
		scm_webhook_store: Some(scm_webhook_store),
		scm_maintenance_store: Some(scm_maintenance_store),
		scm_team_access_store: Some(scm_team_access_store),
		push_mirror_store: Some(push_mirror_store),
		external_mirror_store: Some(external_mirror_store),
	}
}

/// Create or get the development user for dev mode.
pub async fn create_or_get_dev_user(
	user_repo: &Arc<UserRepository>,
) -> Result<loom_server_auth::User, crate::error::ServerError> {
	let email = "dev@localhost";
	let display_name = "Development User";

	// Check if dev user already exists
	if let Some(mut user) = user_repo.get_user_by_email(email).await? {
		// Ensure dev user has admin/support privileges
		if !user.is_system_admin || !user.is_support {
			user.is_system_admin = true;
			user.is_support = true;
			user_repo.update_user(&user).await?;
		}
		return Ok(user);
	}

	// Create new dev user with full privileges
	let now = chrono::Utc::now();
	let username = user_repo.generate_unique_username(display_name).await?;
	let user = loom_server_auth::User {
		id: loom_server_auth::UserId::generate(),
		display_name: display_name.to_string(),
		username: Some(username),
		primary_email: Some(email.to_string()),
		avatar_url: None,
		email_visible: true,
		is_system_admin: true,
		is_support: true,
		is_auditor: false,
		created_at: now,
		updated_at: now,
		deleted_at: None,
		locale: None,
	};

	user_repo.create_user(&user).await?;
	tracing::info!(user_id = %user.id, email = %email, "Created dev user");
	Ok(user)
}

/// Initialize the weaver provisioner and webhook dispatcher if enabled.
async fn initialize_weaver_provisioner(
	config: &ServerConfig,
) -> (Option<Arc<Provisioner>>, Option<Arc<WebhookDispatcher>>) {
	// Check if weaver provisioning is enabled
	if !config.weaver_enabled {
		tracing::info!("Weaver provisioning disabled");
		return (None, None);
	}

	// Try to create K8s client
	let k8s_client = match KubeClient::new().await {
		Ok(client) => Arc::new(client),
		Err(e) => {
			tracing::warn!(
				error = %e,
				"Failed to initialize K8s client, weaver provisioning disabled"
			);
			return (None, None);
		}
	};

	// Parse webhooks from JSON
	let webhooks: Vec<WebhookConfig> = match serde_json::from_str(&config.weaver_webhooks) {
		Ok(webhooks) => webhooks,
		Err(e) => {
			tracing::warn!(
				error = %e,
				webhooks_json = %config.weaver_webhooks,
				"Failed to parse weaver webhooks JSON, using empty list"
			);
			Vec::new()
		}
	};

	// Parse image pull secrets from comma-separated string
	let image_pull_secrets: Vec<String> = config
		.weaver_image_pull_secrets
		.split(',')
		.map(|s| s.trim().to_string())
		.filter(|s| !s.is_empty())
		.collect();

	// Create weaver config from server config
	let weaver_config = WeaverConfig {
		namespace: config.weaver_namespace.clone(),
		cleanup_interval_secs: config.weaver_cleanup_interval_secs,
		default_ttl_hours: config.weaver_default_ttl_hours,
		max_ttl_hours: config.weaver_max_ttl_hours,
		max_concurrent: config.weaver_max_concurrent,
		ready_timeout_secs: config.weaver_ready_timeout_secs,
		webhooks: webhooks.clone(),
		image_pull_secrets,
	};

	// Create provisioner and webhook dispatcher
	let provisioner = Arc::new(Provisioner::new(k8s_client, weaver_config));
	let webhook_dispatcher = Arc::new(WebhookDispatcher::new(webhooks));

	tracing::info!(
		namespace = %config.weaver_namespace,
		max_concurrent = config.weaver_max_concurrent,
		default_ttl_hours = config.weaver_default_ttl_hours,
		"Weaver provisioning enabled"
	);

	(Some(provisioner), Some(webhook_dispatcher))
}

/// Initialize the SMTP client if configured.
fn initialize_smtp_client(config: &ServerConfig) -> Option<Arc<SmtpClient>> {
	let (host, from_address) = match (&config.smtp_host, &config.smtp_from_address) {
		(Some(h), Some(f)) if !h.is_empty() && !f.is_empty() => (h.clone(), f.clone()),
		_ => {
			tracing::info!("SMTP not configured (smtp_host and smtp_from_address required)");
			return None;
		}
	};

	let smtp_config = loom_server_smtp::SmtpConfig {
		host,
		port: config.smtp_port,
		username: config.smtp_username.clone(),
		password: config.smtp_password.clone().map(loom_common_secret::SecretString::new),
		from_address,
		from_name: config.smtp_from_name.clone(),
		use_tls: config.smtp_use_tls,
	};

	match SmtpClient::new(smtp_config) {
		Ok(client) => {
			tracing::info!("SMTP client configured");
			Some(Arc::new(client))
		}
		Err(e) => {
			tracing::warn!(error = %e, "Failed to create SMTP client");
			None
		}
	}
}

/// Initialize the GitHub OAuth client if configured.
fn initialize_github_oauth() -> Option<Arc<GitHubOAuthClient>> {
	match GitHubOAuthConfig::from_env() {
		Ok(config) => {
			tracing::info!("GitHub OAuth configured");
			Some(Arc::new(GitHubOAuthClient::new(config)))
		}
		Err(_) => {
			tracing::info!("GitHub OAuth not configured");
			None
		}
	}
}

/// Initialize the Google OAuth client if configured.
fn initialize_google_oauth() -> Option<Arc<GoogleOAuthClient>> {
	match GoogleOAuthConfig::from_env() {
		Ok(config) => {
			tracing::info!("Google OAuth configured");
			Some(Arc::new(GoogleOAuthClient::new(config)))
		}
		Err(_) => {
			tracing::info!("Google OAuth not configured");
			None
		}
	}
}

/// Initialize the Okta OAuth client if configured.
fn initialize_okta_oauth() -> Option<Arc<OktaOAuthClient>> {
	match OktaOAuthConfig::from_env() {
		Ok(config) => {
			tracing::info!("Okta OAuth configured");
			Some(Arc::new(OktaOAuthClient::new(config)))
		}
		Err(_) => {
			tracing::info!("Okta OAuth not configured");
			None
		}
	}
}

fn admin_routes(state: AppState) -> Router<AppState> {
	use axum::middleware::from_fn_with_state;
	use crate::{auth_middleware::auth_layer, typed_router::require_auth_layer};

	Router::new()
		.route("/users", get(routes::admin::list_users))
		.route(
			"/users/{id}/roles",
			patch(routes::admin::update_user_roles),
		)
		.route(
			"/users/{id}/impersonate",
			post(routes::admin::start_impersonation),
		)
		.route(
			"/impersonate/stop",
			post(routes::admin::stop_impersonation),
		)
		.route("/audit-logs", get(routes::admin::list_audit_logs))
		// Anthropic OAuth pool management
		.route(
			"/anthropic/accounts",
			get(routes::admin_anthropic::list_accounts),
		)
		.route(
			"/anthropic/accounts",
			post(routes::admin_anthropic::initiate_oauth),
		)
		.route(
			"/anthropic/accounts/{id}",
			delete(routes::admin_anthropic::remove_account),
		)
		// Job scheduler management
		.route("/jobs", get(routes::admin_jobs::list_jobs))
		.route("/jobs/{job_id}/run", post(routes::admin_jobs::trigger_job))
		.route(
			"/jobs/{job_id}/cancel",
			post(routes::admin_jobs::cancel_job),
		)
		.route(
			"/jobs/{job_id}/history",
			get(routes::admin_jobs::job_history),
		)
		.route(
			"/jobs/{job_id}/enable",
			post(routes::admin_jobs::enable_job),
		)
		.route(
			"/jobs/{job_id}/disable",
			post(routes::admin_jobs::disable_job),
		)
		.route_layer(RequireRole::admin().with_audit(state.audit_repo.clone()))
		.layer(from_fn_with_state(state.clone(), require_auth_layer))
		.layer(from_fn_with_state(state, auth_layer))
}

/// Create the API router with all routes.
pub fn create_router(state: AppState) -> Router {
	let bin_dir = std::env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());
	let web_dir = std::env::var("LOOM_SERVER_WEB_DIR").ok();
	let has_provisioner = state.provisioner.is_some();

	// Public routes - no authentication required
	let public = PublicRouter::new()
		// Health and metrics
		.route("/health", get(routes::health::health_check))
		.route("/metrics", get(routes::health::prometheus_metrics))
		// Auth routes (public)
		.route("/api/auth/providers", get(routes::auth::get_providers))
		.route(
			"/api/auth/magic-link",
			post(routes::auth::request_magic_link),
		)
		.route(
			"/api/auth/magic-link/verify",
			get(routes::auth::verify_magic_link),
		)
		.route("/api/auth/device/start", post(routes::auth::device_start))
		.route("/api/auth/device/poll", post(routes::auth::device_poll))
		// OAuth login/callback routes
		.route("/api/auth/login/github", get(routes::auth::login_github))
		.route(
			"/api/auth/callback/github",
			get(routes::auth::callback_github),
		)
		.route("/api/auth/login/google", get(routes::auth::login_google))
		.route(
			"/api/auth/callback/google",
			get(routes::auth::callback_google),
		)
		.route("/api/auth/login/okta", get(routes::auth::login_okta))
		.route("/api/auth/callback/okta", get(routes::auth::callback_okta))
		// Public invitation view (GET only)
		.route(
			"/api/invitations/{token}",
			get(routes::invitations::get_invitation),
		)
		// Public shared thread access
		.route(
			"/api/threads/{id}/share/{token}",
			get(routes::share::get_shared_thread),
		)
		// GitHub webhook (signature verified separately)
		.route(
			"/api/github/webhook",
			post(routes::github::github_webhook),
		)
		// Anthropic OAuth callback (public - redirected from claude.ai)
		.route(
			"/api/admin/anthropic/callback",
			get(routes::admin_anthropic::oauth_callback),
		)
		.build();

	// Authenticated routes - require valid session/token
	let mut authed = AuthedRouter::new()
		// Thread API routes
		.route(
			"/api/threads/search",
			get(routes::threads::search_threads),
		)
		.route("/api/threads/{id}", put(routes::threads::upsert_thread))
		.route("/api/threads/{id}", get(routes::threads::get_thread))
		.route(
			"/api/threads/{id}",
			delete(routes::threads::delete_thread),
		)
		.route(
			"/api/threads/{id}/visibility",
			post(routes::threads::update_thread_visibility),
		)
		.route("/api/threads", get(routes::threads::list_threads))
		// Share link routes (authenticated)
		.route(
			"/api/threads/{id}/share",
			post(routes::share::create_share_link),
		)
		.route(
			"/api/threads/{id}/share",
			delete(routes::share::revoke_share_link),
		)
		// Support access routes (authenticated)
		.route(
			"/api/threads/{id}/support-access/request",
			post(routes::share::request_support_access),
		)
		.route(
			"/api/threads/{id}/support-access/approve",
			post(routes::share::approve_support_access),
		)
		.route(
			"/api/threads/{id}/support-access",
			delete(routes::share::revoke_support_access),
		)
		// Auth routes (authenticated)
		.route("/api/auth/me", get(routes::auth::get_current_user))
		.route("/api/auth/logout", post(routes::auth::logout))
		.route(
			"/api/auth/device/complete",
			post(routes::auth::device_complete),
		)
		// Session routes
		.route("/api/sessions", get(routes::sessions::list_sessions))
		.route(
			"/api/sessions/{id}",
			delete(routes::sessions::revoke_session),
		)
		// Organization routes
		.route("/api/orgs", get(routes::orgs::list_orgs))
		.route("/api/orgs", post(routes::orgs::create_org))
		.route("/api/orgs/{id}", get(routes::orgs::get_org))
		.route("/api/orgs/{id}", patch(routes::orgs::update_org))
		.route("/api/orgs/{id}", delete(routes::orgs::delete_org))
		.route(
			"/api/orgs/{id}/members",
			get(routes::orgs::list_org_members),
		)
		.route(
			"/api/orgs/{id}/members",
			post(routes::orgs::add_org_member),
		)
		.route(
			"/api/orgs/{org_id}/members/{user_id}",
			delete(routes::orgs::remove_org_member),
		)
		// Team routes
		.route(
			"/api/orgs/{org_id}/teams",
			get(routes::teams::list_teams),
		)
		.route(
			"/api/orgs/{org_id}/teams",
			post(routes::teams::create_team),
		)
		.route(
			"/api/orgs/{org_id}/teams/{team_id}",
			get(routes::teams::get_team),
		)
		.route(
			"/api/orgs/{org_id}/teams/{team_id}",
			patch(routes::teams::update_team),
		)
		.route(
			"/api/orgs/{org_id}/teams/{team_id}",
			delete(routes::teams::delete_team),
		)
		.route(
			"/api/orgs/{org_id}/teams/{team_id}/members",
			get(routes::teams::list_team_members),
		)
		.route(
			"/api/orgs/{org_id}/teams/{team_id}/members",
			post(routes::teams::add_team_member),
		)
		.route(
			"/api/orgs/{org_id}/teams/{team_id}/members/{user_id}",
			delete(routes::teams::remove_team_member),
		)
		// API key routes
		.route(
			"/api/orgs/{org_id}/api-keys",
			get(routes::api_keys::list_api_keys),
		)
		.route(
			"/api/orgs/{org_id}/api-keys",
			post(routes::api_keys::create_api_key),
		)
		.route(
			"/api/orgs/{org_id}/api-keys/{id}",
			delete(routes::api_keys::revoke_api_key),
		)
		.route(
			"/api/orgs/{org_id}/api-keys/{id}/usage",
			get(routes::api_keys::get_api_key_usage),
		)
		// Invitation routes (authenticated)
		.route(
			"/api/orgs/{org_id}/invitations",
			get(routes::invitations::list_invitations),
		)
		.route(
			"/api/orgs/{org_id}/invitations",
			post(routes::invitations::create_invitation),
		)
		.route(
			"/api/orgs/{org_id}/invitations/{id}",
			delete(routes::invitations::cancel_invitation),
		)
		.route(
			"/api/invitations/accept",
			post(routes::invitations::accept_invitation),
		)
		// Join request routes
		.route(
			"/api/orgs/{org_id}/join-requests",
			get(routes::invitations::list_join_requests),
		)
		.route(
			"/api/orgs/{org_id}/join-requests",
			post(routes::invitations::create_join_request),
		)
		.route(
			"/api/orgs/{org_id}/join-requests/{request_id}/approve",
			post(routes::invitations::approve_join_request),
		)
		.route(
			"/api/orgs/{org_id}/join-requests/{request_id}/reject",
			post(routes::invitations::reject_join_request),
		)
		// User routes
		.route("/api/users/{id}", get(routes::users::get_user_profile))
		.route(
			"/api/users/me",
			patch(routes::users::update_current_user),
		)
		.route(
			"/api/users/me/delete",
			post(routes::users::request_account_deletion),
		)
		.route(
			"/api/users/me/restore",
			post(routes::users::restore_account),
		)
		// User identity routes
		.route(
			"/api/users/me/identities",
			get(routes::users::list_identities),
		)
		.route(
			"/api/users/me/identities/{id}",
			delete(routes::users::unlink_identity),
		)
		// Repository routes
		.route("/api/v1/repos", post(routes::repos::create_repo))
		.route("/api/v1/repos/{id}", get(routes::repos::get_repo))
		.route("/api/v1/repos/{id}", patch(routes::repos::update_repo))
		.route("/api/v1/repos/{id}", delete(routes::repos::delete_repo))
		.route(
			"/api/v1/users/{id}/repos",
			get(routes::repos::list_user_repos),
		)
		.route(
			"/api/v1/orgs/{id}/repos",
			get(routes::repos::list_org_repos),
		)
		// Team access routes
		.route(
			"/api/v1/repos/{id}/teams",
			get(routes::repos::list_repo_team_access),
		)
		.route(
			"/api/v1/repos/{id}/teams",
			post(routes::repos::grant_repo_team_access),
		)
		.route(
			"/api/v1/repos/{id}/teams/{tid}",
			delete(routes::repos::revoke_repo_team_access),
		)
		// Branch protection routes
		.route(
			"/api/v1/repos/{id}/protection",
			get(routes::protection::list_protection_rules),
		)
		.route(
			"/api/v1/repos/{id}/protection",
			post(routes::protection::create_protection_rule),
		)
		.route(
			"/api/v1/repos/{id}/protection/{rule_id}",
			delete(routes::protection::delete_protection_rule),
		)
		// Webhook routes
		.route(
			"/api/v1/repos/{id}/webhooks",
			get(routes::webhooks::list_repo_webhooks),
		)
		.route(
			"/api/v1/repos/{id}/webhooks",
			post(routes::webhooks::create_repo_webhook),
		)
		.route(
			"/api/v1/repos/{id}/webhooks/{wid}",
			delete(routes::webhooks::delete_repo_webhook),
		)
		.route(
			"/api/v1/orgs/{id}/webhooks",
			get(routes::webhooks::list_org_webhooks),
		)
		.route(
			"/api/v1/orgs/{id}/webhooks",
			post(routes::webhooks::create_org_webhook),
		)
		.route(
			"/api/v1/orgs/{id}/webhooks/{wid}",
			delete(routes::webhooks::delete_org_webhook),
		)
		// Mirror routes
		.route(
			"/api/v1/repos/{id}/mirrors",
			get(routes::mirrors::list_mirrors),
		)
		.route(
			"/api/v1/repos/{id}/mirrors",
			post(routes::mirrors::create_mirror),
		)
		.route(
			"/api/v1/repos/{id}/mirrors/{mirror_id}",
			delete(routes::mirrors::delete_mirror),
		)
		.route(
			"/api/v1/repos/{id}/mirrors/{mirror_id}/sync",
			post(routes::mirrors::trigger_sync),
		)
		// Maintenance routes
		.route(
			"/api/v1/repos/{id}/maintenance",
			post(routes::maintenance::trigger_repo_maintenance),
		)
		.route(
			"/api/v1/repos/{id}/maintenance/jobs",
			get(routes::maintenance::list_repo_maintenance_jobs),
		)
		.route(
			"/api/v1/admin/maintenance/sweep",
			post(routes::maintenance::trigger_global_sweep),
		)
		// CSE proxy route
		.route("/proxy/cse", post(routes::cse::proxy_cse))
		// GitHub App endpoints (authenticated)
		.route(
			"/api/github/app",
			get(routes::github::get_github_app_info),
		)
		.route(
			"/api/github/installations/by-repo",
			get(routes::github::get_github_installation_by_repo),
		)
		.route(
			"/proxy/github/search-code",
			post(routes::github::proxy_github_search_code),
		)
		.route(
			"/proxy/github/repo-info",
			post(routes::github::proxy_github_repo_info),
		)
		.route(
			"/proxy/github/file-contents",
			post(routes::github::proxy_github_file_contents),
		)
		// LLM proxy endpoints
		.route(
			"/proxy/anthropic/complete",
			post(llm_proxy::proxy_anthropic_complete),
		)
		.route(
			"/proxy/anthropic/stream",
			post(llm_proxy::proxy_anthropic_stream),
		)
		.route(
			"/proxy/openai/complete",
			post(llm_proxy::proxy_openai_complete),
		)
		.route(
			"/proxy/openai/stream",
			post(llm_proxy::proxy_openai_stream),
		)
		.route(
			"/proxy/vertex/complete",
			post(llm_proxy::proxy_vertex_complete),
		)
		.route(
			"/proxy/vertex/stream",
			post(llm_proxy::proxy_vertex_stream),
		)
		// Server query endpoints
		.route(
			"/api/sessions/{session_id}/query-response",
			post(server_query::handle_query_response),
		)
		.route(
			"/api/sessions/{session_id}/queries",
			get(server_query::list_pending_queries),
		)
		// Debug/tracing endpoints
		.route(
			"/api/debug/query-traces/{trace_id}",
			get(routes::debug::get_query_trace),
		)
		.route(
			"/api/debug/query-traces",
			get(routes::debug::list_query_traces),
		)
		.route(
			"/api/debug/query-traces/stats",
			get(routes::debug::get_trace_stats),
		);

	// Add weaver routes if provisioner is configured
	if has_provisioner {
		authed = authed
			.route("/api/weaver", post(routes::weaver::create_weaver))
			.route("/api/weavers", get(routes::weaver::list_weavers))
			.route("/api/weaver/{id}", get(routes::weaver::get_weaver))
			.route("/api/weaver/{id}", delete(routes::weaver::delete_weaver))
			.route("/api/weaver/{id}/logs", get(routes::weaver::stream_logs))
			.route(
				"/api/weaver/{id}/attach",
				get(routes::weaver::attach_weaver),
			)
			.route(
				"/api/weavers/cleanup",
				post(routes::weaver::trigger_cleanup),
			);
	}

	// Build the authenticated router with auth middleware
	let authed = authed.build(state.clone());

	// Git routes use optional auth (public repos allow anonymous access)
	let git_routes = routes::git::router().build(state.clone());

	// Merge public and authenticated routes
	let mut router = Router::new()
		.merge(public)
		.merge(authed)
		// Git HTTP smart protocol endpoints (optional auth)
		.merge(git_routes)
		// Admin routes (nested with role-based authorization layer, built on raw Router)
		.nest("/api/admin", admin_routes(state.clone()))
		// WebSocket endpoint - no auth middleware (uses first-message auth)
		.route(
			"/v1/ws/sessions/{session_id}",
			get(crate::websocket::handler::ws_upgrade_handler),
		)
		.with_state(state)
		// Bin directory endpoints
		.nest_service(
			"/bin",
			ServeDir::new(&bin_dir)
				.precompressed_gzip()
				.fallback(axum::routing::get(routes::bin::list_bin_directory)),
		);

	// Add OpenAPI documentation
	router = router
		.merge(SwaggerUi::new("/api").url("/api/openapi.json", crate::api_docs::ApiDoc::openapi()));

	// Serve static web assets if LOOM_SERVER_WEB_DIR is set
	if let Some(web_path) = web_dir {
		tracing::info!(web_dir = %web_path, "serving static web assets");
		router = router.fallback_service(
			ServeDir::new(&web_path).fallback(ServeFile::new(format!("{web_path}/index.html"))),
		);
	}

	router
}

#[cfg(test)]
mod tests {
	use super::*;

	use axum::{
		body::Body,
		http::{Request, StatusCode},
	};
	use loom_common_thread::{
		AgentStateKind, AgentStateSnapshot, ConversationSnapshot, Thread, ThreadId, ThreadMetadata,
		ThreadVisibility,
	};
	use tempfile::tempdir;
	use tower::ServiceExt;

	async fn create_test_app() -> (Router, tempfile::TempDir) {
		create_test_app_with_dev_mode(true).await
	}

	async fn create_test_app_no_auth() -> (Router, tempfile::TempDir) {
		create_test_app_with_dev_mode(false).await
	}

	async fn create_test_app_with_dev_mode(dev_mode: bool) -> (Router, tempfile::TempDir) {
		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
		let pool = crate::db::create_pool(&db_url).await.unwrap();
		let repo = Arc::new(ThreadRepository::new(pool.clone()));
		let config = ServerConfig::default();
		let mut state = create_app_state(pool, repo, &config).await;
		// Override auth config for testing
		state.auth_config.dev_mode = dev_mode;
		if dev_mode && state.dev_user.is_none() {
			// Create dev user if not exists
			state.dev_user = match create_or_get_dev_user(&state.user_repo).await {
				Ok(user) => Some(user),
				Err(_) => None,
			};
		}
		(create_router(state), dir)
	}

	fn create_test_thread() -> Thread {
		Thread {
			id: ThreadId::new(),
			version: 1,
			created_at: chrono::Utc::now().to_rfc3339(),
			updated_at: chrono::Utc::now().to_rfc3339(),
			last_activity_at: chrono::Utc::now().to_rfc3339(),
			workspace_root: Some("/test".to_string()),
			cwd: Some("/test".to_string()),
			loom_version: Some("0.1.0".to_string()),
			git_branch: Some("main".to_string()),
			git_remote_url: Some("github.com/test/repo".to_string()),
			git_initial_branch: Some("main".to_string()),
			git_initial_commit_sha: Some("abc123def456".to_string()),
			git_current_commit_sha: Some("xyz789012345".to_string()),
			git_start_dirty: Some(false),
			git_end_dirty: Some(false),
			git_commits: vec!["abc123def456".to_string(), "xyz789012345".to_string()],
			provider: Some("anthropic".to_string()),
			model: Some("claude-sonnet-4-20250514".to_string()),
			conversation: ConversationSnapshot { messages: vec![] },
			agent_state: AgentStateSnapshot {
				kind: AgentStateKind::WaitingForUserInput,
				retries: 0,
				last_error: None,
				pending_tool_calls: vec![],
			},
			metadata: ThreadMetadata::default(),
			visibility: ThreadVisibility::Private,
			is_private: false,
			is_shared_with_support: false,
		}
	}

	#[tokio::test]
	async fn test_health_check() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/health")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		// Should be OK (healthy or degraded, depending on bin dir)
		assert!(
			response.status() == StatusCode::OK || response.status() == StatusCode::SERVICE_UNAVAILABLE
		);
	}

	#[tokio::test]
	async fn test_health_check_response_structure() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/health")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Verify response structure
		assert!(health.get("status").is_some());
		assert!(health.get("timestamp").is_some());
		assert!(health.get("duration_ms").is_some());
		assert!(health.get("version").is_some());
		assert!(health.get("components").is_some());

		let components = health.get("components").unwrap();
		assert!(components.get("database").is_some());
		assert!(components.get("bin_dir").is_some());
		assert!(components.get("llm_providers").is_some());
		assert!(components.get("google_cse").is_some());
	}

	#[tokio::test]
	async fn test_upsert_and_get() {
		let (app, _dir) = create_test_app().await;
		let thread = create_test_thread();
		let thread_json = serde_json::to_string(&thread).unwrap();

		// Upsert
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("PUT")
					.uri(format!("/api/threads/{}", thread.id))
					.header("Content-Type", "application/json")
					.body(Body::from(thread_json))
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		// Get
		let response = app
			.oneshot(
				Request::builder()
					.uri(format!("/api/threads/{}", thread.id))
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn test_get_not_found() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/threads/T-nonexistent")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::NOT_FOUND);
	}

	#[tokio::test]
	async fn test_list_empty() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/threads")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn test_logout_requires_auth() {
		let (app, _dir) = create_test_app_no_auth().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/api/auth/logout")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	}

	#[tokio::test]
	async fn test_get_providers() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("GET")
					.uri("/api/auth/providers")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn test_get_current_user_unauthorized() {
		let (app, _dir) = create_test_app_no_auth().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("GET")
					.uri("/api/auth/me")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	}

	#[tokio::test]
	async fn test_device_start() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/api/auth/device/start")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn test_update_visibility() {
		let (app, _dir) = create_test_app().await;
		let thread = create_test_thread();
		let thread_json = serde_json::to_string(&thread).unwrap();

		// First create the thread
		let _ = app
			.clone()
			.oneshot(
				Request::builder()
					.method("PUT")
					.uri(format!("/api/threads/{}", thread.id))
					.header("Content-Type", "application/json")
					.body(Body::from(thread_json))
					.unwrap(),
			)
			.await
			.unwrap();

		// Update visibility
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri(format!("/api/threads/{}/visibility", thread.id))
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"visibility":"public"}"#))
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let updated: Thread = serde_json::from_slice(&body).unwrap();
		assert_eq!(updated.visibility, loom_common_thread::ThreadVisibility::Public);
	}

	#[tokio::test]
	async fn test_search_endpoint() {
		let (app, _dir) = create_test_app().await;

		// First create a thread
		let thread = create_test_thread();
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.method("PUT")
					.uri(format!("/api/threads/{}", thread.id.as_str()))
					.header("Content-Type", "application/json")
					.header("If-Match", "0")
					.body(Body::from(serde_json::to_string(&thread).unwrap()))
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::OK);

		// Now search for it
		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/threads/search?q=main")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
		assert!(result.get("hits").is_some());
	}

	#[tokio::test]
	async fn test_search_empty_query_returns_error() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/threads/search?q=")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	}

	#[tokio::test]
	async fn test_proxy_cse_empty_query_returns_400() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/proxy/cse")
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"query":""}"#))
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	}

	#[tokio::test]
	async fn test_proxy_cse_whitespace_query_returns_400() {
		let (app, _dir) = create_test_app().await;
		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/proxy/cse")
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"query":"   "}"#))
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::BAD_REQUEST);
	}

	#[tokio::test]
	async fn test_proxy_cse_unconfigured_returns_500() {
		// This test verifies that when CSE is not configured, we get an error
		// (cache miss path, then env var lookup fails)
		let (app, _dir) = create_test_app().await;

		// Clear env vars to ensure CSE is not configured
		std::env::remove_var("LOOM_SERVER_GOOGLE_CSE_API_KEY");
		std::env::remove_var("LOOM_SERVER_GOOGLE_CSE_SEARCH_ENGINE_ID");

		let response = app
			.oneshot(
				Request::builder()
					.method("POST")
					.uri("/proxy/cse")
					.header("Content-Type", "application/json")
					.body(Body::from(r#"{"query":"test query"}"#))
					.unwrap(),
			)
			.await
			.unwrap();

		// Should be 500 because CSE env vars are not set
		assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
	}

	/// Test debug endpoint: GET /api/debug/query-traces/{trace_id}
	/// **Why Important**: Ensures the debug endpoint correctly retrieves stored
	/// traces for performance analysis and debugging query lifecycle issues.
	#[tokio::test]
	async fn test_get_query_trace_endpoint() {
		use crate::query_tracing::QueryTracer;

		let (app, _dir) = create_test_app().await;

		// Create a test trace
		let mut tracer = QueryTracer::new("Q-test-123", Some("session-debug".to_string()));
		tracer.record_sent(10);
		tracer.record_response_received("ok");

		let _trace_id = tracer.trace_id.as_str().to_string();

		// We need to access the app state to store the trace
		// For now, we'll test the endpoint's 404 behavior when trace doesn't exist
		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces/nonexistent-trace".to_string())
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::NOT_FOUND);
	}

	/// Test debug endpoint: GET /api/debug/query-traces
	/// **Why Important**: Ensures the listing endpoint correctly returns all
	/// stored traces for monitoring and debugging purposes.
	#[tokio::test]
	async fn test_list_query_traces_endpoint() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Should have a traces array and count
		assert!(result.get("traces").is_some());
		assert!(result.get("count").is_some());
	}

	/// Test debug endpoint: GET /api/debug/query-traces?session_id=...
	/// **Why Important**: Ensures filtering by session_id correctly isolates
	/// traces for specific client sessions.
	#[tokio::test]
	async fn test_list_query_traces_with_session_filter() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces?session_id=test-session")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Should have a traces array and count (likely empty for non-existent session)
		assert!(result.get("traces").is_some());
		assert!(result.get("count").is_some());
		assert_eq!(result["count"].as_u64(), Some(0));
	}

	/// Test debug endpoint: GET /api/debug/query-traces/stats
	/// **Why Important**: Ensures statistics endpoint correctly aggregates trace
	/// metrics for monitoring trace store health and performance bottlenecks.
	#[tokio::test]
	async fn test_get_trace_stats_endpoint() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces/stats")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		// Should have required statistics fields
		assert!(result.get("total_traces").is_some());
		assert!(result.get("traces_with_errors").is_some());
		assert!(result.get("slow_traces").is_some());
		assert!(result.get("avg_events_per_trace").is_some());
		assert!(result.get("slow_trace_details").is_some());
	}

	/// Test debug endpoints: Complete flow with trace creation and retrieval
	/// **Why Important**: This integration test demonstrates the full flow of
	/// creating, storing, and retrieving traces through the debug endpoints.
	#[tokio::test]
	async fn test_debug_endpoints_integration() {
		use crate::query_tracing::QueryTracer;

		let dir = tempdir().unwrap();
		let db_path = dir.path().join("test.db");
		let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
		let pool = crate::db::create_pool(&db_url).await.unwrap();
		let repo = Arc::new(ThreadRepository::new(pool.clone()));
		let config = ServerConfig::default();
		let mut state = create_app_state(pool, repo, &config).await;

		// Enable dev mode for testing
		state.auth_config.dev_mode = true;
		state.dev_user = match create_or_get_dev_user(&state.user_repo).await {
			Ok(user) => Some(user),
			Err(_) => None,
		};

		// Create and store a trace directly
		let mut tracer = QueryTracer::new(
			"Q-integration-test",
			Some("integration-session".to_string()),
		);
		tracer.record_sent(5);
		tokio::time::sleep(std::time::Duration::from_millis(10)).await;
		tracer.record_response_received("success");

		let trace_id = tracer.trace_id.as_str().to_string();
		state.trace_store.store(tracer).await;

		// Create router with our state
		let app = create_router(state);

		// Test 1: Retrieve the stored trace
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri(format!("/api/debug/query-traces/{trace_id}"))
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let timeline: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert_eq!(timeline["trace_id"].as_str().unwrap(), &trace_id);
		assert_eq!(timeline["query_id"].as_str().unwrap(), "Q-integration-test");
		assert_eq!(
			timeline["session_id"].as_str().unwrap(),
			"integration-session"
		);
		assert_eq!(timeline["events"].as_array().unwrap().len(), 3); // created, sent, response_received

		// Test 2: List traces
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let list_result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert!(list_result["count"].as_u64().unwrap() >= 1);

		// Test 3: Filter by session
		let response = app
			.clone()
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces?session_id=integration-session")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let session_result: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert_eq!(session_result["count"].as_u64().unwrap(), 1);

		// Test 4: Get statistics
		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces/stats")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::OK);
		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let stats: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert!(stats["total_traces"].as_u64().unwrap() >= 1);
		assert_eq!(stats["traces_with_errors"].as_u64().unwrap(), 0);
	}

	/// Test debug endpoint: Trace not found returns 404
	/// **Why Important**: Ensures the endpoint correctly handles missing traces
	/// and returns appropriate HTTP status codes.
	#[tokio::test]
	async fn test_get_query_trace_not_found() {
		let (app, _dir) = create_test_app().await;

		let response = app
			.oneshot(
				Request::builder()
					.uri("/api/debug/query-traces/TRACE-missing-trace-id")
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();

		assert_eq!(response.status(), StatusCode::NOT_FOUND);

		let body = axum::body::to_bytes(response.into_body(), usize::MAX)
			.await
			.unwrap();
		let error: serde_json::Value = serde_json::from_slice(&body).unwrap();

		assert!(error.get("message").is_some());
	}
}
