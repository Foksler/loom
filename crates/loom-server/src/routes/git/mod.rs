// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git HTTP Smart Protocol endpoints for clone/fetch/push operations.
//!
//! Implements the Git smart HTTP protocol as specified in:
//! https://git-scm.com/docs/http-protocol
//!
//! # Endpoints
//!
//! - `GET /git/{owner}/{repo}.git/info/refs` - Advertise refs for clone/fetch/push
//! - `POST /git/{owner}/{repo}.git/git-upload-pack` - Serve clone/fetch
//! - `POST /git/{owner}/{repo}.git/git-receive-pack` - Receive push
//!
//! # Authentication
//!
//! - Public repos: anonymous clone allowed
//! - Private repos: authentication required for all operations
//! - Push: always requires authentication

pub mod access;
pub mod clips;
pub mod common;
pub mod handlers;
pub mod mirror;
pub mod router;

// Re-export public API
pub use access::{check_read_access, get_user_repo_role};
pub use common::{get_repo_path_by_id, resolve_repo};
pub use handlers::{info_refs, receive_pack, upload_pack};
pub use router::router;

#[cfg(test)]
mod tests {
	use super::common::*;
	use super::mirror::*;
	use loom_server_scm_mirror::Platform;

	#[test]
	fn test_pkt_line() {
		let line = pkt_line("# service=git-upload-pack\n");
		assert_eq!(line, b"001e# service=git-upload-pack\n");
	}

	#[test]
	fn test_pkt_flush() {
		assert_eq!(pkt_flush(), b"0000");
	}

	#[test]
	fn test_git_service_from_str() {
		assert_eq!(
			GitService::from_str("git-upload-pack"),
			Some(GitService::UploadPack)
		);
		assert_eq!(
			GitService::from_str("git-receive-pack"),
			Some(GitService::ReceivePack)
		);
		assert_eq!(GitService::from_str("invalid"), None);
	}

	#[test]
	fn test_git_service_content_types() {
		assert_eq!(
			GitService::UploadPack.content_type(),
			"application/x-git-upload-pack-advertisement"
		);
		assert_eq!(
			GitService::ReceivePack.content_type(),
			"application/x-git-receive-pack-advertisement"
		);
		assert_eq!(
			GitService::UploadPack.result_content_type(),
			"application/x-git-upload-pack-result"
		);
		assert_eq!(
			GitService::ReceivePack.result_content_type(),
			"application/x-git-receive-pack-result"
		);
	}

	#[test]
	fn test_get_repos_base_dir() {
		let path = get_repos_base_dir();
		assert!(path.to_string_lossy().contains("repos"));
	}

	#[test]
	fn test_parse_push_commands_empty() {
		let commands = parse_push_commands(b"");
		assert!(commands.is_empty());
	}

	#[test]
	fn test_parse_push_commands_flush() {
		let commands = parse_push_commands(b"0000");
		assert!(commands.is_empty());
	}

	#[test]
	fn test_parse_push_commands_single() {
		let old = "0000000000000000000000000000000000000000";
		let new = "1234567890abcdef1234567890abcdef12345678";
		let refname = "refs/heads/main";
		let line = format!("{} {} {}\n", old, new, refname);
		let pkt = format!("{:04x}{}", line.len() + 4, line);
		let data = format!("{}0000", pkt);

		let commands = parse_push_commands(data.as_bytes());
		assert_eq!(commands.len(), 1);
		assert_eq!(commands[0].old_sha, old);
		assert_eq!(commands[0].new_sha, new);
		assert_eq!(commands[0].ref_name, refname);
	}

	#[test]
	fn test_extract_branch_name() {
		assert_eq!(extract_branch_name("refs/heads/main"), Some("main"));
		assert_eq!(
			extract_branch_name("refs/heads/feature/test"),
			Some("feature/test")
		);
		assert_eq!(extract_branch_name("refs/tags/v1.0"), None);
		assert_eq!(extract_branch_name("main"), None);
	}

	#[test]
	fn test_zero_sha_constant() {
		assert_eq!(ZERO_SHA.len(), 40);
		assert!(ZERO_SHA.chars().all(|c| c == '0'));
	}

	#[test]
	fn test_parse_mirror_path_github() {
		let result = parse_mirror_path("mirrors/github/torvalds", "linux.git");
		assert!(result.is_some());
		let info = result.unwrap();
		assert_eq!(info.platform, Platform::GitHub);
		assert_eq!(info.external_owner, "torvalds");
		assert_eq!(info.external_repo, "linux");
	}

	#[test]
	fn test_parse_mirror_path_gitlab() {
		let result = parse_mirror_path("mirrors/gitlab/gitlab-org", "gitlab");
		assert!(result.is_some());
		let info = result.unwrap();
		assert_eq!(info.platform, Platform::GitLab);
		assert_eq!(info.external_owner, "gitlab-org");
		assert_eq!(info.external_repo, "gitlab");
	}

	#[test]
	fn test_parse_mirror_path_invalid_prefix() {
		let result = parse_mirror_path("repos/github/owner", "repo");
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_mirror_path_invalid_platform() {
		let result = parse_mirror_path("mirrors/bitbucket/owner", "repo");
		assert!(result.is_none());
	}

	#[test]
	fn test_parse_mirror_path_missing_parts() {
		let result = parse_mirror_path("mirrors/github", "repo");
		assert!(result.is_none());
	}

	#[test]
	fn test_is_mirror_path() {
		assert!(is_mirror_path("mirrors/github/owner"));
		assert!(is_mirror_path("mirrors/gitlab/owner"));
		assert!(!is_mirror_path("org/owner"));
		assert!(!is_mirror_path("user"));
	}

	#[test]
	fn test_parse_mirror_git_path_info_refs() {
		let result = parse_mirror_git_path("github/torvalds/linux.git/info/refs");
		assert!(result.is_some());
		let (owner, repo) = result.unwrap();
		assert_eq!(owner, "mirrors/github/torvalds");
		assert_eq!(repo, "linux.git");
	}

	#[test]
	fn test_parse_mirror_git_path_upload_pack() {
		let result = parse_mirror_git_path("github/torvalds/linux.git/git-upload-pack");
		assert!(result.is_some());
		let (owner, repo) = result.unwrap();
		assert_eq!(owner, "mirrors/github/torvalds");
		assert_eq!(repo, "linux.git");
	}

	#[test]
	fn test_parse_mirror_git_path_receive_pack() {
		let result = parse_mirror_git_path("gitlab/gitlab-org/gitlab/git-receive-pack");
		assert!(result.is_some());
		let (owner, repo) = result.unwrap();
		assert_eq!(owner, "mirrors/gitlab/gitlab-org");
		assert_eq!(repo, "gitlab");
	}

	#[test]
	fn test_parse_mirror_git_path_with_leading_slash() {
		let result = parse_mirror_git_path("/github/rust-lang/rust.git/info/refs");
		assert!(result.is_some());
		let (owner, repo) = result.unwrap();
		assert_eq!(owner, "mirrors/github/rust-lang");
		assert_eq!(repo, "rust.git");
	}

	#[test]
	fn test_parse_mirror_git_path_invalid() {
		assert!(parse_mirror_git_path("github/torvalds").is_none());
		assert!(parse_mirror_git_path("").is_none());
		assert!(parse_mirror_git_path("github/torvalds/linux.git").is_none());
	}

	#[test]
	fn test_get_repo_path_by_id() {
		let id = uuid::Uuid::parse_str("12345678-1234-1234-1234-123456789012").unwrap();
		let path = get_repo_path_by_id(id);
		assert!(path.to_string_lossy().contains("12"));
		assert!(path
			.to_string_lossy()
			.contains("12345678-1234-1234-1234-123456789012"));
		assert!(path.to_string_lossy().ends_with("git"));
	}

	use proptest::prelude::*;

	proptest! {
		/// **Property: Valid mirror paths parse successfully**
		/// A properly constructed mirror path should always parse correctly.
		#[test]
		fn prop_valid_mirror_path_parses(
			platform in prop_oneof!["github", "gitlab"],
			owner in "[a-zA-Z][a-zA-Z0-9_-]{2,20}",
			repo in "[a-zA-Z][a-zA-Z0-9_-]{2,20}"
		) {
			let owner_path = format!("mirrors/{}/{}", platform, owner);
			let repo_name = format!("{}.git", repo);

			let result = parse_mirror_path(&owner_path, &repo_name);
			prop_assert!(result.is_some(), "Valid path should parse: {}/{}", owner_path, repo_name);

			let info = result.unwrap();
			prop_assert_eq!(info.external_owner, owner);
			prop_assert_eq!(info.external_repo, repo);
		}

		/// **Property: Invalid prefix never parses as mirror**
		/// Paths not starting with "mirrors/" should return None.
		#[test]
		fn prop_non_mirror_prefix_fails(
			prefix in "[a-z]{3,10}",
			platform in "[a-z]{3,10}",
			owner in "[a-zA-Z0-9]{3,10}"
		) {
			prop_assume!(prefix != "mirrors");
			let owner_path = format!("{}/{}/{}", prefix, platform, owner);
			let result = parse_mirror_path(&owner_path, "repo.git");
			prop_assert!(result.is_none());
		}

		/// **Property: is_mirror_path consistent with parse_mirror_path**
		/// If is_mirror_path returns true, parse should have a chance to succeed.
		#[test]
		fn prop_is_mirror_path_consistency(
			platform in prop_oneof!["github", "gitlab"],
			owner in "[a-zA-Z][a-zA-Z0-9]{2,15}"
		) {
			let owner_path = format!("mirrors/{}/{}", platform, owner);
			prop_assert!(is_mirror_path(&owner_path));
		}

		/// **Property: parse_mirror_git_path extracts owner and repo**
		/// Valid git paths should parse to correct owner and repo components.
		#[test]
		fn prop_mirror_git_path_roundtrip(
			platform in prop_oneof!["github", "gitlab"],
			owner in "[a-zA-Z][a-zA-Z0-9]{2,15}",
			repo in "[a-zA-Z][a-zA-Z0-9]{2,15}",
			suffix in prop_oneof!["/info/refs", "/git-upload-pack", "/git-receive-pack"]
		) {
			let path = format!("{}/{}/{}.git{}", platform, owner, repo, suffix);
			let result = parse_mirror_git_path(&path);
			prop_assert!(result.is_some(), "Path should parse: {}", path);

			let (parsed_owner, parsed_repo) = result.unwrap();
			prop_assert!(parsed_owner.contains(&owner));
			prop_assert!(parsed_repo.contains(&repo));
		}
	}
}
