// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Gettext catalog loading and translation functions.

use std::collections::HashMap;

use gettext::Catalog;
use once_cell::sync::Lazy;

use crate::locale::DEFAULT_LOCALE;

const EN_MO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/en.mo"));
const ES_MO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/es.mo"));
const AR_MO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ar.mo"));

static CATALOGS: Lazy<HashMap<&'static str, Catalog>> = Lazy::new(|| {
	let mut map = HashMap::new();

	if let Ok(catalog) = Catalog::parse(EN_MO) {
		map.insert("en", catalog);
	} else {
		tracing::error!("Failed to parse English translation catalog");
	}

	if let Ok(catalog) = Catalog::parse(ES_MO) {
		map.insert("es", catalog);
	} else {
		tracing::warn!("Failed to parse Spanish translation catalog");
	}

	if let Ok(catalog) = Catalog::parse(AR_MO) {
		map.insert("ar", catalog);
	} else {
		tracing::warn!("Failed to parse Arabic translation catalog");
	}

	map
});

/// Translate a string for the given locale.
///
/// Falls back to English if the translation is not found, then to the msgid itself.
///
/// # Arguments
///
/// * `locale` - The locale code (e.g., "en", "es", "ar")
/// * `msgid` - The message ID to translate (e.g., "server.email.magic_link.subject")
///
/// # Returns
///
/// The translated string, or the English translation, or the msgid if not found.
///
/// # Example
///
/// ```
/// use loom_i18n::t;
///
/// let subject = t("es", "server.email.magic_link.subject");
/// ```
pub fn t(locale: &str, msgid: &str) -> String {
	if let Some(catalog) = CATALOGS.get(locale) {
		let translated = catalog.gettext(msgid);
		if translated != msgid {
			return translated.to_string();
		}
	}

	if locale != DEFAULT_LOCALE {
		if let Some(catalog) = CATALOGS.get(DEFAULT_LOCALE) {
			let translated = catalog.gettext(msgid);
			if translated != msgid {
				return translated.to_string();
			}
		}
	}

	msgid.to_string()
}

/// Translate a string with variable substitution.
///
/// Variables use `{name}` syntax in the translated string.
///
/// # Arguments
///
/// * `locale` - The locale code (e.g., "en", "es", "ar")
/// * `msgid` - The message ID to translate
/// * `args` - Variable substitutions as (name, value) pairs
///
/// # Returns
///
/// The translated string with variables substituted.
///
/// # Example
///
/// ```
/// use loom_i18n::t_fmt;
///
/// let body = t_fmt("es", "server.email.invitation.subject", &[
///     ("org_name", "Acme Corp"),
/// ]);
/// ```
pub fn t_fmt(locale: &str, msgid: &str, args: &[(&str, &str)]) -> String {
	let mut result = t(locale, msgid);

	for (name, value) in args {
		let placeholder = format!("{{{}}}", name);
		result = result.replace(&placeholder, value);
	}

	result
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_translate_english() {
		let result = t("en", "server.email.magic_link.subject");
		assert_eq!(result, "Sign in to Loom");
	}

	#[test]
	fn test_translate_spanish() {
		let result = t("es", "server.email.magic_link.subject");
		assert_eq!(result, "Iniciar sesión en Loom");
	}

	#[test]
	fn test_translate_arabic() {
		let result = t("ar", "server.email.magic_link.subject");
		assert_eq!(result, "تسجيل الدخول إلى Loom");
	}

	#[test]
	fn test_fallback_to_english() {
		let result = t("es", "server.nonexistent.key");
		let en_result = t("en", "server.nonexistent.key");
		assert_eq!(result, en_result);
	}

	#[test]
	fn test_fallback_to_msgid() {
		let result = t("en", "completely.unknown.key");
		assert_eq!(result, "completely.unknown.key");
	}

	#[test]
	fn test_variable_substitution() {
		let result = t_fmt("en", "server.email.magic_link.expires", &[("minutes", "10")]);
		assert!(result.contains("10"));
		assert!(!result.contains("{minutes}"));
	}

	#[test]
	fn test_multiple_variables() {
		let result = t_fmt("en", "server.email.invitation.body", &[
			("inviter_name", "Alice"),
			("org_name", "Acme"),
		]);
		assert!(result.contains("Alice"));
		assert!(result.contains("Acme"));
	}

	#[test]
	fn test_unknown_locale_falls_back() {
		let result = t("xx", "server.email.magic_link.subject");
		let en_result = t("en", "server.email.magic_link.subject");
		assert_eq!(result, en_result);
	}
}
