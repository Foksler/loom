// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use regex::Regex;
use std::sync::LazyLock;

static SLUG_REGEX: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$|^[a-z0-9]$").unwrap());

pub fn validate_slug(slug: &str, min_len: usize, max_len: usize) -> bool {
	slug.len() >= min_len && slug.len() <= max_len && SLUG_REGEX.is_match(slug)
}

pub fn sanitize_email(email: &str) -> String {
	email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_validate_slug() {
		assert!(validate_slug("a", 1, 50));
		assert!(validate_slug("abc", 1, 50));
		assert!(validate_slug("abc-def", 1, 50));
		assert!(validate_slug("a1b2c3", 1, 50));

		assert!(!validate_slug("", 1, 50));
		assert!(!validate_slug("-abc", 1, 50));
		assert!(!validate_slug("abc-", 1, 50));
		assert!(!validate_slug("ABC", 1, 50));
		assert!(!validate_slug("ab", 3, 50));
	}

	#[test]
	fn test_sanitize_email() {
		assert_eq!(sanitize_email("  Test@Example.COM  "), "test@example.com");
	}
}
