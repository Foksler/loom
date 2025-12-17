//! Build information and version utilities.

shadow_rs::shadow!(build);

/// Build information for the CLI.
pub struct BuildInfo {
    pub version: &'static str,
    pub git_sha: &'static str,
    pub build_timestamp: &'static str,
    pub platform: &'static str,
}

/// Get the current build information.
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: build::PKG_VERSION,
        git_sha: if build::SHORT_COMMIT.is_empty() { "unknown" } else { build::SHORT_COMMIT },
        build_timestamp: build::BUILD_TIME,
        platform: platform_string(),
    }
}

/// Get the platform string (os-arch format like "linux-x86_64").
pub fn platform_string() -> &'static str {
    // Convert Rust target triple to our simplified platform string
    // e.g., "x86_64-unknown-linux-gnu" -> "linux-x86_64"
    match build::BUILD_TARGET {
        "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl" => "linux-x86_64",
        "aarch64-unknown-linux-gnu" | "aarch64-unknown-linux-musl" => "linux-aarch64",
        "x86_64-apple-darwin" => "macos-x86_64",
        "aarch64-apple-darwin" => "macos-aarch64",
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "windows-x86_64",
        other => other, // fallback to full triple if unknown
    }
}

/// Format version info for display.
pub fn format_version_info() -> String {
    use chrono::{DateTime, Utc};
    
    let info = build_info();
    
    let mut output = format!(
        "Loom version: {}\n\
         Git SHA:      {}\n\
         Built at:     {}\n\
         Platform:     {}",
        info.version,
        info.git_sha,
        info.build_timestamp,
        info.platform,
    );
    
    // Try to parse build time and calculate age
    if let Ok(built_at) = DateTime::parse_from_rfc3339(info.build_timestamp)
        .or_else(|_| DateTime::parse_from_str(info.build_timestamp, "%Y-%m-%d %H:%M:%S"))
    {
        let built_at_utc: DateTime<Utc> = built_at.into();
        let now = Utc::now();
        let age = now.signed_duration_since(built_at_utc);
        
        if let Ok(std_duration) = age.to_std() {
            output.push_str(&format!(
                "\nBuild age:    {} ({} seconds)",
                humantime::format_duration(std_duration),
                std_duration.as_secs()
            ));
        }
    }
    
    output
}
