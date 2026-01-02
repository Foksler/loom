// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Configuration sources: environment variables and TOML files.

use std::path::PathBuf;

use loom_common_config::load_secret_env;
use tracing::{debug, trace};

use crate::error::ConfigError;
use crate::layer::ServerConfigLayer;
use crate::sections::*;

/// Source precedence levels (higher = overrides lower).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
	Defaults = 10,
	ConfigFile = 20,
	Environment = 50,
}

/// Trait for configuration sources.
pub trait ConfigSource: Send + Sync {
	fn name(&self) -> &'static str;
	fn precedence(&self) -> Precedence;
	fn load(&self) -> Result<ServerConfigLayer, ConfigError>;
}

/// Built-in defaults source.
pub struct DefaultsSource;

impl ConfigSource for DefaultsSource {
	fn name(&self) -> &'static str {
		"defaults"
	}

	fn precedence(&self) -> Precedence {
		Precedence::Defaults
	}

	fn load(&self) -> Result<ServerConfigLayer, ConfigError> {
		debug!("loading defaults");
		Ok(ServerConfigLayer::default())
	}
}

/// TOML file configuration source.
pub struct TomlSource {
	path: PathBuf,
}

impl TomlSource {
	pub fn new(path: impl Into<PathBuf>) -> Self {
		Self { path: path.into() }
	}

	pub fn system() -> Self {
		Self::new("/etc/loom/server.toml")
	}
}

impl ConfigSource for TomlSource {
	fn name(&self) -> &'static str {
		"toml-config"
	}

	fn precedence(&self) -> Precedence {
		Precedence::ConfigFile
	}

	fn load(&self) -> Result<ServerConfigLayer, ConfigError> {
		if !self.path.exists() {
			debug!(path = %self.path.display(), "config file not found, skipping");
			return Ok(ServerConfigLayer::default());
		}

		debug!(path = %self.path.display(), "loading config file");
		let content = std::fs::read_to_string(&self.path).map_err(|e| ConfigError::FileRead {
			path: self.path.clone(),
			source: e,
		})?;

		let layer: ServerConfigLayer =
			toml::from_str(&content).map_err(|e| ConfigError::TomlParse {
				path: self.path.clone(),
				source: e,
			})?;

		trace!("parsed config layer from TOML");
		Ok(layer)
	}
}

/// Environment variable source.
///
/// Convention: LOOM_SERVER_<SECTION>_<FIELD>
pub struct EnvSource;

impl ConfigSource for EnvSource {
	fn name(&self) -> &'static str {
		"environment"
	}

	fn precedence(&self) -> Precedence {
		Precedence::Environment
	}

	fn load(&self) -> Result<ServerConfigLayer, ConfigError> {
		debug!("loading environment variables");
		let mut layer = ServerConfigLayer::default();

		layer.http = Some(load_http_from_env()?);
		layer.database = Some(load_database_from_env()?);
		layer.auth = Some(load_auth_from_env()?);
		layer.llm = Some(load_llm_from_env()?);
		layer.weaver = Some(load_weaver_from_env()?);
		layer.smtp = Some(load_smtp_from_env()?);
		layer.oauth = Some(load_oauth_from_env()?);
		layer.github_app = Some(load_github_app_from_env()?);
		layer.geoip = Some(load_geoip_from_env()?);
		layer.jobs = Some(load_jobs_from_env()?);
		layer.search = Some(load_search_from_env()?);
		layer.paths = Some(load_paths_from_env()?);
		layer.logging = Some(load_logging_from_env()?);

		Ok(layer)
	}
}

fn env_var(name: &str) -> Option<String> {
	std::env::var(name).ok().filter(|s| !s.is_empty())
}

fn env_bool(name: &str) -> Option<bool> {
	env_var(name).map(|v| v.eq_ignore_ascii_case("true") || v == "1")
}

fn env_u16(name: &str) -> Result<Option<u16>, ConfigError> {
	match env_var(name) {
		Some(v) => v.parse().map(Some).map_err(|_| ConfigError::InvalidValue {
			key: name.to_string(),
			message: format!("invalid u16 value '{v}'"),
		}),
		None => Ok(None),
	}
}

fn env_u32(name: &str) -> Result<Option<u32>, ConfigError> {
	match env_var(name) {
		Some(v) => v.parse().map(Some).map_err(|_| ConfigError::InvalidValue {
			key: name.to_string(),
			message: format!("invalid u32 value '{v}'"),
		}),
		None => Ok(None),
	}
}

fn env_u64(name: &str) -> Result<Option<u64>, ConfigError> {
	match env_var(name) {
		Some(v) => v.parse().map(Some).map_err(|_| ConfigError::InvalidValue {
			key: name.to_string(),
			message: format!("invalid u64 value '{v}'"),
		}),
		None => Ok(None),
	}
}

fn load_http_from_env() -> Result<HttpConfigLayer, ConfigError> {
	Ok(HttpConfigLayer {
		host: env_var("LOOM_SERVER_HOST"),
		port: env_u16("LOOM_SERVER_PORT")?,
		base_url: env_var("LOOM_SERVER_BASE_URL"),
	})
}

fn load_database_from_env() -> Result<DatabaseConfigLayer, ConfigError> {
	Ok(DatabaseConfigLayer {
		url: env_var("LOOM_SERVER_DATABASE_URL"),
	})
}

fn load_auth_from_env() -> Result<AuthConfigLayer, ConfigError> {
	Ok(AuthConfigLayer {
		dev_mode: env_bool("LOOM_SERVER_AUTH_DEV_MODE"),
		environment: env_var("LOOM_SERVER_ENV"),
		session_cleanup_interval_secs: env_u64("LOOM_SERVER_SESSION_CLEANUP_INTERVAL_SECS")?,
		oauth_state_cleanup_interval_secs: env_u64("LOOM_SERVER_OAUTH_STATE_CLEANUP_INTERVAL_SECS")?,
	})
}

fn load_llm_from_env() -> Result<LlmConfigLayer, ConfigError> {
	let provider = env_var("LOOM_SERVER_LLM_PROVIDER").map(|s| match s.to_lowercase().as_str() {
		"anthropic" => LlmProvider::Anthropic,
		"openai" => LlmProvider::OpenAi,
		"vertex" => LlmProvider::Vertex,
		_ => LlmProvider::Anthropic,
	});

	let oauth_enabled = env_bool("LOOM_SERVER_ANTHROPIC_OAUTH_ENABLED").unwrap_or(false);

	let anthropic_auth = if oauth_enabled {
		env_var("LOOM_SERVER_ANTHROPIC_OAUTH_CREDENTIAL_FILE").map(|path| {
			AnthropicAuthConfig::OAuthPool {
				credential_file: PathBuf::from(path),
				cooldown_secs: env_u64("LOOM_SERVER_ANTHROPIC_POOL_COOLDOWN_SECS")
					.ok()
					.flatten()
					.unwrap_or(7200),
			}
		})
	} else {
		load_secret_env("LOOM_SERVER_ANTHROPIC_API_KEY")
			.map_err(|e| ConfigError::Secret(e.to_string()))?
			.map(AnthropicAuthConfig::ApiKey)
	};

	Ok(LlmConfigLayer {
		provider,
		anthropic_auth,
		anthropic_model: env_var("LOOM_SERVER_ANTHROPIC_MODEL"),
		openai_api_key: load_secret_env("LOOM_SERVER_OPENAI_API_KEY")
			.map_err(|e| ConfigError::Secret(e.to_string()))?,
		openai_model: env_var("LOOM_SERVER_OPENAI_MODEL"),
		openai_organization: env_var("LOOM_SERVER_OPENAI_ORGANIZATION"),
		vertex_project: env_var("LOOM_SERVER_VERTEX_PROJECT"),
		vertex_location: env_var("LOOM_SERVER_VERTEX_LOCATION"),
		vertex_model: env_var("LOOM_SERVER_VERTEX_MODEL"),
	})
}

fn load_weaver_from_env() -> Result<WeaverConfigLayer, ConfigError> {
	let webhooks_json = env_var("LOOM_SERVER_WEAVER_WEBHOOKS");
	let webhooks = webhooks_json.and_then(|json| serde_json::from_str(&json).ok());

	let image_pull_secrets = env_var("LOOM_SERVER_WEAVER_IMAGE_PULL_SECRETS").map(|s| {
		s.split(',')
			.map(|s| s.trim().to_string())
			.filter(|s| !s.is_empty())
			.collect()
	});

	Ok(WeaverConfigLayer {
		enabled: env_bool("LOOM_SERVER_WEAVER_ENABLED"),
		namespace: env_var("LOOM_SERVER_WEAVER_K8S_NAMESPACE"),
		cleanup_interval_secs: env_u64("LOOM_SERVER_WEAVER_CLEANUP_INTERVAL_SECS")?,
		default_ttl_hours: env_u32("LOOM_SERVER_WEAVER_DEFAULT_TTL_HOURS")?,
		max_ttl_hours: env_u32("LOOM_SERVER_WEAVER_MAX_TTL_HOURS")?,
		max_concurrent: env_u32("LOOM_SERVER_WEAVER_MAX_CONCURRENT")?,
		ready_timeout_secs: env_u64("LOOM_SERVER_WEAVER_READY_TIMEOUT_SECS")?,
		webhooks,
		image_pull_secrets,
	})
}

fn load_smtp_from_env() -> Result<SmtpConfigLayer, ConfigError> {
	let tls_mode = env_var("LOOM_SERVER_SMTP_TLS").map(|v| match v.to_lowercase().as_str() {
		"starttls" => TlsMode::StartTls,
		"false" | "none" => TlsMode::None,
		_ => TlsMode::Tls,
	});

	Ok(SmtpConfigLayer {
		host: env_var("LOOM_SERVER_SMTP_HOST"),
		port: env_u16("LOOM_SERVER_SMTP_PORT")?,
		username: env_var("LOOM_SERVER_SMTP_USERNAME"),
		password: load_secret_env("LOOM_SERVER_SMTP_PASSWORD")
			.map_err(|e| ConfigError::Secret(e.to_string()))?,
		from_address: env_var("LOOM_SERVER_SMTP_FROM_ADDRESS")
			.or_else(|| env_var("LOOM_SERVER_SMTP_FROM")),
		from_name: env_var("LOOM_SERVER_SMTP_FROM_NAME"),
		use_tls: env_bool("LOOM_SERVER_SMTP_USE_TLS"),
		tls_mode,
	})
}

fn load_oauth_from_env() -> Result<OAuthConfigLayer, ConfigError> {
	let github = GitHubOAuthConfigLayer {
		client_id: env_var("LOOM_SERVER_GITHUB_CLIENT_ID"),
		client_secret: load_secret_env("LOOM_SERVER_GITHUB_CLIENT_SECRET")
			.map_err(|e| ConfigError::Secret(e.to_string()))?,
		redirect_uri: env_var("LOOM_SERVER_GITHUB_REDIRECT_URI"),
		scopes: None,
	};

	let google = GoogleOAuthConfigLayer {
		client_id: env_var("LOOM_SERVER_GOOGLE_CLIENT_ID"),
		client_secret: load_secret_env("LOOM_SERVER_GOOGLE_CLIENT_SECRET")
			.map_err(|e| ConfigError::Secret(e.to_string()))?,
		redirect_uri: env_var("LOOM_SERVER_GOOGLE_REDIRECT_URI"),
		scopes: None,
	};

	let okta = OktaOAuthConfigLayer {
		domain: env_var("LOOM_SERVER_OKTA_DOMAIN"),
		client_id: env_var("LOOM_SERVER_OKTA_CLIENT_ID"),
		client_secret: load_secret_env("LOOM_SERVER_OKTA_CLIENT_SECRET")
			.map_err(|e| ConfigError::Secret(e.to_string()))?,
		redirect_uri: env_var("LOOM_SERVER_OKTA_REDIRECT_URI"),
		scopes: None,
	};

	Ok(OAuthConfigLayer {
		github,
		google,
		okta,
	})
}

fn load_github_app_from_env() -> Result<GitHubAppConfigLayer, ConfigError> {
	let app_id = match env_var("LOOM_SERVER_GITHUB_APP_ID") {
		Some(v) => Some(v.parse().map_err(|_| ConfigError::InvalidValue {
			key: "LOOM_SERVER_GITHUB_APP_ID".to_string(),
			message: format!("invalid u64 value '{v}'"),
		})?),
		None => None,
	};

	Ok(GitHubAppConfigLayer {
		app_id,
		private_key_pem: load_secret_env("LOOM_SERVER_GITHUB_APP_PRIVATE_KEY")
			.map_err(|e| ConfigError::Secret(e.to_string()))?,
		webhook_secret: load_secret_env("LOOM_SERVER_GITHUB_APP_WEBHOOK_SECRET")
			.map_err(|e| ConfigError::Secret(e.to_string()))?,
		app_slug: env_var("LOOM_SERVER_GITHUB_APP_SLUG"),
		base_url: env_var("LOOM_SERVER_GITHUB_APP_BASE_URL"),
	})
}

fn load_geoip_from_env() -> Result<GeoIpConfigLayer, ConfigError> {
	Ok(GeoIpConfigLayer {
		database_path: env_var("LOOM_SERVER_GEOIP_DATABASE_PATH"),
	})
}

fn load_jobs_from_env() -> Result<JobsConfigLayer, ConfigError> {
	let alert_recipients = env_var("LOOM_SERVER_JOB_ALERT_RECIPIENTS").map(|s| {
		s.split(',')
			.map(|s| s.trim().to_string())
			.filter(|s| !s.is_empty())
			.collect()
	});

	Ok(JobsConfigLayer {
		alert_enabled: env_bool("LOOM_SERVER_JOB_ALERT_ENABLED"),
		alert_recipients,
		history_retention_days: env_u32("LOOM_SERVER_JOB_HISTORY_RETENTION_DAYS")?,
	})
}

fn load_search_from_env() -> Result<SearchConfigLayer, ConfigError> {
	let google_cse = if env_var("LOOM_SERVER_GOOGLE_CSE_API_KEY").is_some()
		|| env_var("LOOM_SERVER_GOOGLE_CSE_SEARCH_ENGINE_ID").is_some()
	{
		Some(GoogleCseConfigLayer {
			api_key: load_secret_env("LOOM_SERVER_GOOGLE_CSE_API_KEY")
				.map_err(|e| ConfigError::Secret(e.to_string()))?,
			search_engine_id: env_var("LOOM_SERVER_GOOGLE_CSE_SEARCH_ENGINE_ID"),
		})
	} else {
		None
	};

	let serper = if env_var("LOOM_SERVER_SERPER_API_KEY").is_some() {
		Some(SerperConfigLayer {
			api_key: load_secret_env("LOOM_SERVER_SERPER_API_KEY")
				.map_err(|e| ConfigError::Secret(e.to_string()))?,
		})
	} else {
		None
	};

	Ok(SearchConfigLayer { google_cse, serper })
}

fn load_paths_from_env() -> Result<PathsConfigLayer, ConfigError> {
	Ok(PathsConfigLayer {
		bin_dir: env_var("LOOM_SERVER_BIN_DIR"),
		web_dir: env_var("LOOM_SERVER_WEB_DIR"),
		data_dir: env_var("LOOM_SERVER_DATA_DIR"),
	})
}

fn load_logging_from_env() -> Result<LoggingConfigLayer, ConfigError> {
	Ok(LoggingConfigLayer {
		level: env_var("LOOM_SERVER_LOG_LEVEL"),
		locale: env_var("LOOM_SERVER_DEFAULT_LOCALE"),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_precedence_ordering() {
		assert!(Precedence::Environment > Precedence::ConfigFile);
		assert!(Precedence::ConfigFile > Precedence::Defaults);
	}

	#[test]
	fn test_defaults_source_returns_empty_layer() {
		let source = DefaultsSource;
		let layer = source.load().unwrap();
		assert!(layer.http.is_none());
		assert!(layer.database.is_none());
	}

	#[test]
	fn test_toml_source_missing_file_returns_empty() {
		let source = TomlSource::new("/nonexistent/config.toml");
		let layer = source.load().unwrap();
		assert!(layer.http.is_none());
	}
}
