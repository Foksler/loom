// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

pub use loom_server_db::{BranchProtectionRuleRecord, ProtectionRepository, ProtectionStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectionViolation {
	DirectPushBlocked { branch: String, pattern: String },
	ForcePushBlocked { branch: String, pattern: String },
	DeletionBlocked { branch: String, pattern: String },
}

impl std::fmt::Display for ProtectionViolation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ProtectionViolation::DirectPushBlocked { branch, pattern } => {
				write!(
					f,
					"Direct push to branch '{}' is blocked by protection rule '{}'",
					branch, pattern
				)
			}
			ProtectionViolation::ForcePushBlocked { branch, pattern } => {
				write!(
					f,
					"Force push to branch '{}' is blocked by protection rule '{}'",
					branch, pattern
				)
			}
			ProtectionViolation::DeletionBlocked { branch, pattern } => {
				write!(
					f,
					"Deletion of branch '{}' is blocked by protection rule '{}'",
					branch, pattern
				)
			}
		}
	}
}

#[derive(Debug, Clone)]
pub struct PushCheck {
	pub branch: String,
	pub is_force_push: bool,
	pub is_deletion: bool,
	pub user_is_admin: bool,
}

pub fn matches_pattern(pattern: &str, branch: &str) -> bool {
	if pattern == branch {
		return true;
	}

	if let Some(prefix) = pattern.strip_suffix("/*") {
		return branch.starts_with(&format!("{}/", prefix));
	}

	if let Some(prefix) = pattern.strip_suffix('*') {
		return branch.starts_with(prefix);
	}

	false
}

pub fn check_push_allowed(
	rules: &[BranchProtectionRuleRecord],
	check: &PushCheck,
) -> std::result::Result<(), ProtectionViolation> {
	if check.user_is_admin {
		return Ok(());
	}

	for rule in rules {
		if !matches_pattern(&rule.pattern, &check.branch) {
			continue;
		}

		if check.is_deletion && rule.block_deletion {
			return Err(ProtectionViolation::DeletionBlocked {
				branch: check.branch.clone(),
				pattern: rule.pattern.clone(),
			});
		}

		if check.is_force_push && rule.block_force_push {
			return Err(ProtectionViolation::ForcePushBlocked {
				branch: check.branch.clone(),
				pattern: rule.pattern.clone(),
			});
		}

		if rule.block_direct_push {
			return Err(ProtectionViolation::DirectPushBlocked {
				branch: check.branch.clone(),
				pattern: rule.pattern.clone(),
			});
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::Utc;
	use uuid::Uuid;

	fn make_rule(repo_id: Uuid, pattern: &str) -> BranchProtectionRuleRecord {
		BranchProtectionRuleRecord {
			id: Uuid::new_v4(),
			repo_id,
			pattern: pattern.to_string(),
			block_direct_push: true,
			block_force_push: true,
			block_deletion: true,
			created_at: Utc::now(),
		}
	}

	#[test]
	fn test_matches_pattern_exact() {
		assert!(matches_pattern("cannon", "cannon"));
		assert!(!matches_pattern("cannon", "main"));
		assert!(!matches_pattern("cannon", "cannons"));
	}

	#[test]
	fn test_matches_pattern_wildcard() {
		assert!(matches_pattern("release/*", "release/v1.0"));
		assert!(matches_pattern("release/*", "release/"));
		assert!(!matches_pattern("release/*", "release"));
		assert!(!matches_pattern("release/*", "releases/v1.0"));
	}

	#[test]
	fn test_matches_pattern_prefix_wildcard() {
		assert!(matches_pattern("feat*", "feature"));
		assert!(matches_pattern("feat*", "feat"));
		assert!(matches_pattern("feat*", "feat-new"));
		assert!(!matches_pattern("feat*", "fix"));
	}

	#[test]
	fn test_check_push_allowed_admin_bypass() {
		let rules = vec![make_rule(Uuid::new_v4(), "cannon")];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: true,
			is_deletion: true,
			user_is_admin: true,
		};
		assert!(check_push_allowed(&rules, &check).is_ok());
	}

	#[test]
	fn test_check_push_allowed_direct_push_blocked() {
		let rules = vec![make_rule(Uuid::new_v4(), "cannon")];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: false,
			is_deletion: false,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::DirectPushBlocked { .. })
		));
	}

	#[test]
	fn test_check_push_allowed_force_push_blocked() {
		let mut rule = make_rule(Uuid::new_v4(), "cannon");
		rule.block_direct_push = false;
		let rules = vec![rule];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: true,
			is_deletion: false,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::ForcePushBlocked { .. })
		));
	}

	#[test]
	fn test_check_push_allowed_deletion_blocked() {
		let mut rule = make_rule(Uuid::new_v4(), "cannon");
		rule.block_direct_push = false;
		rule.block_force_push = false;
		let rules = vec![rule];
		let check = PushCheck {
			branch: "cannon".to_string(),
			is_force_push: false,
			is_deletion: true,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::DeletionBlocked { .. })
		));
	}

	#[test]
	fn test_check_push_allowed_unprotected_branch() {
		let rules = vec![make_rule(Uuid::new_v4(), "cannon")];
		let check = PushCheck {
			branch: "feature/new-thing".to_string(),
			is_force_push: true,
			is_deletion: true,
			user_is_admin: false,
		};
		assert!(check_push_allowed(&rules, &check).is_ok());
	}

	#[test]
	fn test_check_push_allowed_wildcard_pattern() {
		let rules = vec![make_rule(Uuid::new_v4(), "release/*")];
		let check = PushCheck {
			branch: "release/v1.0".to_string(),
			is_force_push: false,
			is_deletion: false,
			user_is_admin: false,
		};
		let result = check_push_allowed(&rules, &check);
		assert!(matches!(
			result,
			Err(ProtectionViolation::DirectPushBlocked { .. })
		));
	}
}
