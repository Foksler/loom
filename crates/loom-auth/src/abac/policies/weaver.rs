// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Weaver access policies.

use crate::abac::{Action, ResourceAttrs, SubjectAttrs};

/// Evaluates weaver access policies.
pub fn evaluate(subject: &SubjectAttrs, _action: Action, resource: &ResourceAttrs) -> bool {
	if let Some(owner_id) = &resource.owner_user_id {
		if subject.user_id == *owner_id {
			return true;
		}
	}

	false
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::abac::ResourceAttrs;
	use crate::UserId;
	use uuid::Uuid;

	fn test_user_id() -> UserId {
		UserId::new(Uuid::new_v4())
	}

	#[test]
	fn owner_has_full_access() {
		let user_id = test_user_id();
		let subject = SubjectAttrs::new(user_id);
		let resource = ResourceAttrs::weaver(user_id);

		assert!(evaluate(&subject, Action::Read, &resource));
		assert!(evaluate(&subject, Action::Write, &resource));
		assert!(evaluate(&subject, Action::Delete, &resource));
	}

	#[test]
	fn non_owner_cannot_access() {
		let owner_id = test_user_id();
		let subject = SubjectAttrs::new(test_user_id());
		let resource = ResourceAttrs::weaver(owner_id);

		assert!(!evaluate(&subject, Action::Read, &resource));
		assert!(!evaluate(&subject, Action::Write, &resource));
		assert!(!evaluate(&subject, Action::Delete, &resource));
	}
}
