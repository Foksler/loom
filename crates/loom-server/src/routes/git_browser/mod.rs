// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git browser API endpoints for the web UI.
//!
//! These endpoints support browsing repository contents via the web UI:
//! - Get repository by owner/name
//! - List branches
//! - Browse tree (directories)
//! - View blob (files)
//! - List commits
//! - View single commit with diff
//! - Blame view
//! - Compare refs

pub mod handlers;
pub mod helpers;
pub mod router;
pub mod types;

// Re-export types
pub use types::*;

// Re-export handlers
pub use handlers::{
	compare_refs, get_blame, get_blob, get_commit, get_raw, get_repo_by_owner_name, get_tree,
	list_branches, list_commits,
};

// Re-export router
pub use router::router;

#[cfg(test)]
mod tests {
	use super::helpers::{build_clone_url, get_content_type_for_path, parse_blame_output};
	use uuid::Uuid;

	// ==================== Blame Output Parsing Tests ====================

	#[test]
	fn test_parse_blame_output_single_line() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 1
author John Doe
author-mail <john@example.com>
author-time 1609459200
author-tz +0000
committer John Doe
committer-mail <john@example.com>
committer-time 1609459200
committer-tz +0000
summary Initial commit
filename test.txt
	Hello, World!"#;

		let result = parse_blame_output(input).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].line_number, 1);
		assert_eq!(
			result[0].commit_sha,
			"abc123def456789012345678901234567890abcd"
		);
		assert_eq!(result[0].author_name, "John Doe");
		assert_eq!(result[0].author_email, "john@example.com");
		assert_eq!(result[0].content, "Hello, World!");
	}

	#[test]
	fn test_parse_blame_output_multiple_lines_same_commit() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 2
author Alice
author-mail <alice@example.com>
author-time 1609459200
author-tz +0000
summary Initial commit
filename test.txt
	Line 1
abc123def456789012345678901234567890abcd 2 2
	Line 2"#;

		let result = parse_blame_output(input).unwrap();
		assert_eq!(result.len(), 2);

		assert_eq!(result[0].line_number, 1);
		assert_eq!(result[0].content, "Line 1");
		assert_eq!(result[0].author_name, "Alice");

		assert_eq!(result[1].line_number, 2);
		assert_eq!(result[1].content, "Line 2");
		assert_eq!(result[1].author_name, "Alice");
	}

	#[test]
	fn test_parse_blame_output_different_commits() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 1
author Alice
author-mail <alice@example.com>
author-time 1609459200
author-tz +0000
summary First commit
filename test.txt
	First line
def456789012345678901234567890abcdef0123 2 2 1
author Bob
author-mail <bob@example.com>
author-time 1609545600
author-tz +0000
summary Second commit
filename test.txt
	Second line"#;

		let result = parse_blame_output(input).unwrap();
		assert_eq!(result.len(), 2);

		assert_eq!(result[0].author_name, "Alice");
		assert_eq!(
			result[0].commit_sha,
			"abc123def456789012345678901234567890abcd"
		);

		assert_eq!(result[1].author_name, "Bob");
		assert_eq!(
			result[1].commit_sha,
			"def456789012345678901234567890abcdef0123"
		);
	}

	#[test]
	fn test_parse_blame_output_empty_lines() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 1
author Test
author-mail <test@example.com>
author-time 1609459200
author-tz +0000
filename test.txt
	"#;

		let result = parse_blame_output(input).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].content, "");
	}

	#[test]
	fn test_parse_blame_output_special_characters() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 1
author Test
author-mail <test@example.com>
author-time 1609459200
author-tz +0000
filename test.txt
	fn main() { println!("Hello, <World>!"); }"#;

		let result = parse_blame_output(input).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(
			result[0].content,
			"fn main() { println!(\"Hello, <World>!\"); }"
		);
	}

	#[test]
	fn test_parse_blame_output_empty() {
		let result = parse_blame_output("").unwrap();
		assert!(result.is_empty());
	}

	#[test]
	fn test_parse_blame_author_email_strips_brackets() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 1
author Test
author-mail <user@domain.com>
author-time 1609459200
author-tz +0000
filename test.txt
	content"#;

		let result = parse_blame_output(input).unwrap();
		assert_eq!(result[0].author_email, "user@domain.com");
	}

	#[test]
	fn test_parse_blame_author_time_format() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 1
author Test
author-mail <test@example.com>
author-time 1609459200
author-tz +0000
filename test.txt
	content"#;

		let result = parse_blame_output(input).unwrap();
		assert!(result[0].author_date.starts_with("2021-01-01"));
	}

	// ==================== Compare Refs Tests ====================

	#[test]
	fn test_compare_refs_parse_valid() {
		let refs = "main...feature-branch";
		let result = refs.split_once("...");
		assert!(result.is_some());
		let (base, head) = result.unwrap();
		assert_eq!(base, "main");
		assert_eq!(head, "feature-branch");
	}

	#[test]
	fn test_compare_refs_parse_shas() {
		let refs = "abc123...def456";
		let result = refs.split_once("...");
		assert!(result.is_some());
		let (base, head) = result.unwrap();
		assert_eq!(base, "abc123");
		assert_eq!(head, "def456");
	}

	#[test]
	fn test_compare_refs_parse_with_slashes() {
		let refs = "main...feature/my-feature";
		let result = refs.split_once("...");
		assert!(result.is_some());
		let (base, head) = result.unwrap();
		assert_eq!(base, "main");
		assert_eq!(head, "feature/my-feature");
	}

	#[test]
	fn test_compare_refs_invalid_two_dots() {
		let refs = "main..feature";
		let result = refs.split_once("...");
		assert!(result.is_none());
	}

	#[test]
	fn test_compare_refs_invalid_no_separator() {
		let refs = "main-feature";
		let result = refs.split_once("...");
		assert!(result.is_none());
	}

	// ==================== Content Type Tests ====================

	#[test]
	fn test_get_content_type_rust() {
		assert_eq!(
			get_content_type_for_path("main.rs"),
			"text/x-rust; charset=utf-8"
		);
	}

	#[test]
	fn test_get_content_type_python() {
		assert_eq!(
			get_content_type_for_path("script.py"),
			"text/x-python; charset=utf-8"
		);
	}

	#[test]
	fn test_get_content_type_json() {
		assert_eq!(
			get_content_type_for_path("config.json"),
			"application/json; charset=utf-8"
		);
	}

	#[test]
	fn test_get_content_type_image() {
		assert_eq!(get_content_type_for_path("logo.png"), "image/png");
		assert_eq!(get_content_type_for_path("photo.jpg"), "image/jpeg");
		assert_eq!(get_content_type_for_path("icon.svg"), "image/svg+xml");
	}

	#[test]
	fn test_get_content_type_unknown() {
		assert_eq!(
			get_content_type_for_path("file.unknown"),
			"application/octet-stream"
		);
		assert_eq!(
			get_content_type_for_path("noextension"),
			"application/octet-stream"
		);
	}

	#[test]
	fn test_get_content_type_nested_path() {
		assert_eq!(
			get_content_type_for_path("src/lib/module.rs"),
			"text/x-rust; charset=utf-8"
		);
	}

	// ==================== Clone URL Tests ====================

	#[test]
	fn test_build_clone_url_basic() {
		let url = build_clone_url(
			"https://loom.example.com",
			&Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap(),
			"my-repo",
		);
		assert_eq!(
			url,
			"https://loom.example.com/git/12345678-1234-1234-1234-123456789abc/my-repo.git"
		);
	}

	#[test]
	fn test_build_clone_url_strips_trailing_slash() {
		let url = build_clone_url(
			"https://loom.example.com/",
			&Uuid::parse_str("12345678-1234-1234-1234-123456789abc").unwrap(),
			"my-repo",
		);
		assert_eq!(
			url,
			"https://loom.example.com/git/12345678-1234-1234-1234-123456789abc/my-repo.git"
		);
	}

	// ==================== Blame Property Tests ====================

	#[test]
	fn test_blame_output_line_count_property() {
		let inputs = vec![
			"",
			"abc123def456789012345678901234567890abcd 1 1 1\nauthor Test\nauthor-mail <t@t.com>\nauthor-time 1609459200\nfilename f\n\tline",
			"abc123def456789012345678901234567890abcd 1 1 2\nauthor A\nauthor-mail <a@a.com>\nauthor-time 1609459200\nfilename f\n\tl1\nabc123def456789012345678901234567890abcd 2 2\n\tl2",
		];

		for input in inputs {
			let tab_lines = input.lines().filter(|l| l.starts_with('\t')).count();
			let result = parse_blame_output(input).unwrap();
			assert!(
				result.len() <= tab_lines,
				"Parsed lines ({}) should not exceed tab-prefixed lines ({}) for input: {}",
				result.len(),
				tab_lines,
				input
			);
		}
	}

	#[test]
	fn test_blame_line_numbers_positive() {
		let input = r#"abc123def456789012345678901234567890abcd 1 5 1
author Test
author-mail <t@t.com>
author-time 1609459200
filename f
	content"#;

		let result = parse_blame_output(input).unwrap();
		for line in &result {
			assert!(
				line.line_number > 0,
				"Line number should be positive: {}",
				line.line_number
			);
		}
	}

	#[test]
	fn test_blame_commit_sha_format() {
		let input = r#"abc123def456789012345678901234567890abcd 1 1 1
author Test
author-mail <t@t.com>
author-time 1609459200
filename f
	content"#;

		let result = parse_blame_output(input).unwrap();
		for line in &result {
			assert_eq!(
				line.commit_sha.len(),
				40,
				"SHA should be 40 chars: {}",
				line.commit_sha
			);
			assert!(
				line.commit_sha.chars().all(|c| c.is_ascii_hexdigit()),
				"SHA should be hex: {}",
				line.commit_sha
			);
		}
	}
}
