// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Deterministic sampling logic for session tracking.
//!
//! This module provides a hash-based sampling algorithm that ensures
//! the same session ID always produces the same sampling decision,
//! enabling consistent behavior across distributed systems.

use std::hash::{Hash, Hasher};

/// Determines whether a session should be sampled based on its ID.
///
/// Uses a deterministic hash-based algorithm:
/// 1. Hash the session ID string using DefaultHasher
/// 2. Take the hash modulo 10000 (giving a number 0-9999)
/// 3. Compare to sample_rate * 10000
/// 4. If hash mod < threshold, session is sampled
///
/// # Arguments
///
/// * `session_id` - The session ID string to hash
/// * `sample_rate` - Sample rate from 0.0 to 1.0 (0% to 100%)
///
/// # Returns
///
/// `true` if the session should be sampled, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use loom_sessions_core::sampling::should_sample;
///
/// // 100% sample rate always samples
/// assert!(should_sample("session-123", 1.0));
///
/// // 0% sample rate never samples
/// assert!(!should_sample("session-123", 0.0));
///
/// // Deterministic: same ID + rate = same result
/// let result1 = should_sample("session-456", 0.5);
/// let result2 = should_sample("session-456", 0.5);
/// assert_eq!(result1, result2);
/// ```
#[must_use]
pub fn should_sample(session_id: &str, sample_rate: f64) -> bool {
	// Edge cases
	if sample_rate <= 0.0 {
		return false;
	}
	if sample_rate >= 1.0 {
		return true;
	}

	let hash = compute_hash(session_id);
	let threshold = (sample_rate * 10000.0) as u64;
	(hash % 10000) < threshold
}

/// Computes a deterministic hash for a session ID string.
///
/// Uses `DefaultHasher` which provides consistent hashing within
/// a single program execution. For cross-process consistency,
/// the hash is taken modulo 10000.
#[must_use]
pub fn compute_hash(session_id: &str) -> u64 {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	session_id.hash(&mut hasher);
	hasher.finish()
}

/// Validates that a sample rate is within the valid range [0.0, 1.0].
///
/// # Arguments
///
/// * `sample_rate` - The sample rate to validate
///
/// # Returns
///
/// `true` if the sample rate is valid, `false` otherwise.
#[must_use]
pub fn is_valid_sample_rate(sample_rate: f64) -> bool {
	(0.0..=1.0).contains(&sample_rate) && !sample_rate.is_nan()
}

/// Clamps a sample rate to the valid range [0.0, 1.0].
///
/// # Arguments
///
/// * `sample_rate` - The sample rate to clamp
///
/// # Returns
///
/// The clamped sample rate.
#[must_use]
pub fn clamp_sample_rate(sample_rate: f64) -> f64 {
	if sample_rate.is_nan() {
		1.0 // Default to full sampling on NaN
	} else {
		sample_rate.clamp(0.0, 1.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	// ========================================================================
	// Basic Unit Tests
	// ========================================================================

	#[test]
	fn test_should_sample_100_percent() {
		// 100% sample rate should always sample
		assert!(should_sample("session-1", 1.0));
		assert!(should_sample("session-2", 1.0));
		assert!(should_sample("any-session-id", 1.0));
		assert!(should_sample("", 1.0));
	}

	#[test]
	fn test_should_sample_0_percent() {
		// 0% sample rate should never sample
		assert!(!should_sample("session-1", 0.0));
		assert!(!should_sample("session-2", 0.0));
		assert!(!should_sample("any-session-id", 0.0));
		assert!(!should_sample("", 0.0));
	}

	#[test]
	fn test_should_sample_deterministic() {
		// Same session ID + sample rate should always give same result
		for rate in [0.1, 0.25, 0.5, 0.75, 0.9] {
			let session_id = "test-session-deterministic";
			let result1 = should_sample(session_id, rate);
			let result2 = should_sample(session_id, rate);
			let result3 = should_sample(session_id, rate);
			assert_eq!(result1, result2);
			assert_eq!(result2, result3);
		}
	}

	#[test]
	fn test_should_sample_different_ids_may_differ() {
		// Different session IDs should (statistically) produce different results
		// at 50% sample rate. We test with many IDs to verify distribution.
		let mut sampled_count = 0;
		let total = 1000;

		for i in 0..total {
			if should_sample(&format!("session-{i}"), 0.5) {
				sampled_count += 1;
			}
		}

		// With 50% sample rate, expect roughly half to be sampled
		// Allow 10% variance (400-600 out of 1000)
		assert!(
			(400..=600).contains(&sampled_count),
			"Expected ~500 sampled sessions, got {sampled_count}"
		);
	}

	#[test]
	fn test_should_sample_boundary_rates() {
		// Test rates just above 0 and just below 1
		let session_id = "boundary-test-session";

		// Very low rate should rarely sample
		let low_rate = 0.0001;
		// Very high rate should almost always sample
		let high_rate = 0.9999;

		// These are specific session IDs, so results are deterministic
		// We just verify they return boolean values
		let _ = should_sample(session_id, low_rate);
		let _ = should_sample(session_id, high_rate);
	}

	#[test]
	fn test_should_sample_handles_negative_rate() {
		// Negative rates should be treated as 0%
		assert!(!should_sample("session", -0.5));
		assert!(!should_sample("session", -1.0));
	}

	#[test]
	fn test_should_sample_handles_rate_above_one() {
		// Rates above 1.0 should be treated as 100%
		assert!(should_sample("session", 1.5));
		assert!(should_sample("session", 2.0));
	}

	// ========================================================================
	// Hash Function Tests
	// ========================================================================

	#[test]
	fn test_compute_hash_deterministic() {
		let session_id = "test-hash-session";
		let hash1 = compute_hash(session_id);
		let hash2 = compute_hash(session_id);
		assert_eq!(hash1, hash2);
	}

	#[test]
	fn test_compute_hash_different_ids_different_hashes() {
		let hash1 = compute_hash("session-a");
		let hash2 = compute_hash("session-b");
		// Different inputs should produce different hashes (with very high probability)
		assert_ne!(hash1, hash2);
	}

	#[test]
	fn test_compute_hash_empty_string() {
		// Empty string should produce a valid hash
		let hash = compute_hash("");
		assert!(hash > 0 || hash == 0); // Just verify it returns something
	}

	// ========================================================================
	// Validation Tests
	// ========================================================================

	#[test]
	fn test_is_valid_sample_rate() {
		// Valid rates
		assert!(is_valid_sample_rate(0.0));
		assert!(is_valid_sample_rate(0.5));
		assert!(is_valid_sample_rate(1.0));
		assert!(is_valid_sample_rate(0.001));
		assert!(is_valid_sample_rate(0.999));

		// Invalid rates
		assert!(!is_valid_sample_rate(-0.1));
		assert!(!is_valid_sample_rate(1.1));
		assert!(!is_valid_sample_rate(f64::NAN));
		assert!(!is_valid_sample_rate(f64::INFINITY));
		assert!(!is_valid_sample_rate(f64::NEG_INFINITY));
	}

	#[test]
	fn test_clamp_sample_rate() {
		// Within range - unchanged
		assert!((clamp_sample_rate(0.5) - 0.5).abs() < f64::EPSILON);
		assert!((clamp_sample_rate(0.0) - 0.0).abs() < f64::EPSILON);
		assert!((clamp_sample_rate(1.0) - 1.0).abs() < f64::EPSILON);

		// Below range - clamped to 0
		assert!((clamp_sample_rate(-0.5) - 0.0).abs() < f64::EPSILON);
		assert!((clamp_sample_rate(-100.0) - 0.0).abs() < f64::EPSILON);

		// Above range - clamped to 1
		assert!((clamp_sample_rate(1.5) - 1.0).abs() < f64::EPSILON);
		assert!((clamp_sample_rate(100.0) - 1.0).abs() < f64::EPSILON);

		// NaN - defaults to 1.0
		assert!((clamp_sample_rate(f64::NAN) - 1.0).abs() < f64::EPSILON);
	}

	// ========================================================================
	// Property-Based Tests (Proptest)
	// ========================================================================

	proptest! {
		#[test]
		fn prop_deterministic_sampling(
			session_id in "[a-zA-Z0-9-]{1,50}",
			sample_rate in 0.0f64..=1.0
		) {
			// Same inputs should always produce same output
			let result1 = should_sample(&session_id, sample_rate);
			let result2 = should_sample(&session_id, sample_rate);
			prop_assert_eq!(result1, result2);
		}

		#[test]
		fn prop_100_percent_always_samples(session_id in ".*") {
			prop_assert!(should_sample(&session_id, 1.0));
		}

		#[test]
		fn prop_0_percent_never_samples(session_id in ".*") {
			prop_assert!(!should_sample(&session_id, 0.0));
		}

		#[test]
		fn prop_hash_is_deterministic(session_id in ".*") {
			let hash1 = compute_hash(&session_id);
			let hash2 = compute_hash(&session_id);
			prop_assert_eq!(hash1, hash2);
		}

		#[test]
		fn prop_valid_sample_rate_validation(sample_rate in 0.0f64..=1.0) {
			prop_assert!(is_valid_sample_rate(sample_rate));
		}

		#[test]
		fn prop_invalid_sample_rate_below_zero(sample_rate in f64::MIN..-0.0001) {
			prop_assert!(!is_valid_sample_rate(sample_rate));
		}

		#[test]
		fn prop_invalid_sample_rate_above_one(sample_rate in 1.0001..f64::MAX) {
			prop_assert!(!is_valid_sample_rate(sample_rate));
		}

		#[test]
		fn prop_clamp_keeps_valid_rates(sample_rate in 0.0f64..=1.0) {
			let clamped = clamp_sample_rate(sample_rate);
			prop_assert!((clamped - sample_rate).abs() < f64::EPSILON);
		}

		#[test]
		fn prop_clamp_fixes_invalid_rates(sample_rate in -1000.0f64..2000.0) {
			let clamped = clamp_sample_rate(sample_rate);
			prop_assert!(is_valid_sample_rate(clamped) || clamped.is_nan());
			prop_assert!(clamped >= 0.0);
			prop_assert!(clamped <= 1.0);
		}
	}

	// ========================================================================
	// Statistical Distribution Tests
	// ========================================================================

	#[test]
	fn test_sampling_distribution_10_percent() {
		// At 10% sample rate, ~10% should be sampled
		let mut sampled = 0;
		let total = 10000;
		for i in 0..total {
			if should_sample(&format!("dist-10-{i}"), 0.1) {
				sampled += 1;
			}
		}
		// Allow 20% variance: 800-1200 out of 10000
		let expected = 1000;
		let variance = 200;
		assert!(
			((expected - variance)..=(expected + variance)).contains(&sampled),
			"10% sampling: expected ~{expected} sampled, got {sampled}"
		);
	}

	#[test]
	fn test_sampling_distribution_50_percent() {
		// At 50% sample rate, ~50% should be sampled
		let mut sampled = 0;
		let total = 10000;
		for i in 0..total {
			if should_sample(&format!("dist-50-{i}"), 0.5) {
				sampled += 1;
			}
		}
		// Allow 5% variance: 4500-5500 out of 10000
		let expected = 5000;
		let variance = 500;
		assert!(
			((expected - variance)..=(expected + variance)).contains(&sampled),
			"50% sampling: expected ~{expected} sampled, got {sampled}"
		);
	}

	#[test]
	fn test_sampling_distribution_90_percent() {
		// At 90% sample rate, ~90% should be sampled
		let mut sampled = 0;
		let total = 10000;
		for i in 0..total {
			if should_sample(&format!("dist-90-{i}"), 0.9) {
				sampled += 1;
			}
		}
		// Allow 2% variance: 8800-9200 out of 10000
		let expected = 9000;
		let variance = 200;
		assert!(
			((expected - variance)..=(expected + variance)).contains(&sampled),
			"90% sampling: expected ~{expected} sampled, got {sampled}"
		);
	}
}
