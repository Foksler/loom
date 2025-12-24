// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Binary directory listing HTTP handler.

use axum::{http::StatusCode, response::IntoResponse};

/// Handler to list files in the /bin directory
pub async fn list_bin_directory() -> impl IntoResponse {
	let bin_dir = std::env::var("LOOM_SERVER_BIN_DIR").unwrap_or_else(|_| "./bin".to_string());
	let path = std::path::Path::new(&bin_dir);

	let mut entries = Vec::new();

	if path.exists() && path.is_dir() {
		if let Ok(read_dir) = std::fs::read_dir(path) {
			for entry in read_dir.flatten() {
				let name = entry.file_name().to_string_lossy().to_string();
				let metadata = entry.metadata().ok();
				let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
				let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
				let modified = metadata
					.as_ref()
					.and_then(|m| m.modified().ok())
					.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
					.map(|d| {
						let secs = d.as_secs();
						let dt = chrono::DateTime::from_timestamp(secs as i64, 0).unwrap_or_default();
						dt.format("%Y-%m-%d %H:%M").to_string()
					})
					.unwrap_or_else(|| "-".to_string());

				entries.push((name, is_dir, size, modified));
			}
		}
	}

	entries.sort_by(|a, b| match (a.1, b.1) {
		(true, false) => std::cmp::Ordering::Less,
		(false, true) => std::cmp::Ordering::Greater,
		_ => a.0.cmp(&b.0),
	});

	let mut html = String::from(
		r#"<!DOCTYPE html>
<html>
<head>
<title>Index of /bin/</title>
<style>
body { font-family: monospace; margin: 2em; }
table { border-collapse: collapse; }
th, td { text-align: left; padding: 0.25em 1em; }
th { border-bottom: 1px solid #ccc; }
a { text-decoration: none; }
a:hover { text-decoration: underline; }
.dir { font-weight: bold; }
.size { text-align: right; }
</style>
</head>
<body>
<h1>Index of /bin/</h1>
<table>
<tr><th>Name</th><th>Size</th><th>Modified</th></tr>
"#,
	);

	for (name, is_dir, size, modified) in entries {
		let display_name = if is_dir {
			format!("{}/", name)
		} else {
			name.clone()
		};
		let size_str = if is_dir {
			"-".to_string()
		} else {
			format_size(size)
		};
		let class = if is_dir { " class=\"dir\"" } else { "" };
		html.push_str(&format!(
			r#"<tr><td{class}><a href="/bin/{name}">{display_name}</a></td><td class="size">{size_str}</td><td>{modified}</td></tr>
"#
		));
	}

	html.push_str("</table>\n</body>\n</html>");

	(
		StatusCode::OK,
		[("Content-Type", "text/html; charset=utf-8")],
		html,
	)
}

fn format_size(size: u64) -> String {
	const KB: u64 = 1024;
	const MB: u64 = KB * 1024;
	const GB: u64 = MB * 1024;

	if size >= GB {
		format!("{:.1}G", size as f64 / GB as f64)
	} else if size >= MB {
		format!("{:.1}M", size as f64 / MB as f64)
	} else if size >= KB {
		format!("{:.1}K", size as f64 / KB as f64)
	} else {
		format!("{size}")
	}
}
