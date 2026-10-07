//! Keyed answer verification.
//!
//! Stored answers (quiz correct indices today; API sequences and
//! alarm orders by the same pattern) never ship in plaintext. Level
//! data carries only a *tag*: hex HMAC-SHA256 over
//! `{domain}:{scope}:{payload}` — for a quiz question, the payload is
//! the correct choice index and the scope is the question id. The
//! game recomputes the tag for a candidate answer and compares; it
//! never holds the answer itself.
//!
//! The key is injected at build time via the `LIGHT_SHOW_ANSWER_KEY`
//! environment variable (`option_env!`), so the public source tree
//! contains the mechanism but not the key. The key lives only in the
//! primo answer store (sops/age) and in release-build environments.
//!
//! Honest limits, by design:
//! * Without the key, tags cannot be enumerated offline — a quiz's
//!   four candidate indices cannot be tested against a tag. This is
//!   what stops bulk answer-key extraction from the repo or the
//!   shipped level data.
//! * A shipped binary necessarily contains the key (or can compute
//!   tags), so a determined reverse engineer with the binary can
//!   extract it and enumerate from there. This mechanism is
//!   anti-scrape, not DRM.
//! * Fail-closed: a build with no key injected verifies nothing —
//!   every tag check returns false. Release pipelines must inject
//!   the key or quiz-gated levels cannot be passed.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Test-only key so unit tests can compute and verify tags through
/// the same code path as release builds. Never used outside
/// `cfg(test)`.
#[cfg(test)]
const TEST_KEY: &str = "light-show-test-answer-key";

/// The answer key this build resolves: the injected
/// `LIGHT_SHOW_ANSWER_KEY` when present (release builds, and test
/// runs exercising the shipped tagged data), else — in test builds
/// only — the test key, so constructed fixtures verify through the
/// same code path with no key material present.
pub fn answer_key() -> Option<&'static str> {
    if let Some(key) = option_env!("LIGHT_SHOW_ANSWER_KEY") {
        return Some(key);
    }
    #[cfg(test)]
    {
        Some(TEST_KEY)
    }
    #[cfg(not(test))]
    {
        None
    }
}

fn mac_for(key: &str, domain: &str, scope: &str, payload: &str) -> HmacSha256 {
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC accepts a key of any length");
    mac.update(format!("{domain}:{scope}:{payload}").as_bytes());
    mac
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

/// Compute the tag for a payload under an explicit key. Used by the
/// authoring importer (with the real key) and by tests.
pub fn tag_with_key(key: &str, domain: &str, scope: &str, payload: &str) -> String {
    hex_encode(&mac_for(key, domain, scope, payload).finalize().into_bytes())
}

/// Compute the tag for a payload under the build-time key, if any.
pub fn tag_for(domain: &str, scope: &str, payload: &str) -> Option<String> {
    answer_key().map(|key| tag_with_key(key, domain, scope, payload))
}

/// Verify a candidate payload against a stored tag. Fails closed:
/// no key, malformed tag, or mismatch all return `false`.
pub fn verify_tag(domain: &str, scope: &str, payload: &str, expected_hex: &str) -> bool {
    let Some(key) = answer_key() else {
        return false;
    };
    let Some(expected) = hex_decode(expected_hex) else {
        return false;
    };
    mac_for(key, domain, scope, payload)
        .verify_slice(&expected)
        .is_ok()
}

/// Test helper: the tag for a payload under the key this build
/// resolves (the injected key when present, else the test key).
#[cfg(test)]
pub fn test_tag_for(domain: &str, scope: &str, payload: &str) -> String {
    tag_with_key(
        answer_key().expect("test builds always resolve an answer key"),
        domain,
        scope,
        payload,
    )
}

/// Test helper: the tag for a quiz question's correct index under
/// the resolved key.
#[cfg(test)]
pub fn test_tag(question_id: &str, correct_idx: usize) -> String {
    test_tag_for("quiz", question_id, &correct_idx.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_accepts_only_the_tagged_payload() {
        let tag = test_tag("nec250-001", 2);
        assert!(verify_tag("quiz", "nec250-001", "2", &tag));
        assert!(!verify_tag("quiz", "nec250-001", "0", &tag));
        assert!(!verify_tag("quiz", "nec250-001", "1", &tag));
        assert!(!verify_tag("quiz", "nec250-001", "3", &tag));
        assert!(!verify_tag("quiz", "nec250-002", "2", &tag));
    }

    #[test]
    fn verify_rejects_malformed_tags() {
        assert!(!verify_tag("quiz", "q", "0", "not-hex"));
        assert!(!verify_tag("quiz", "q", "0", ""));
        assert!(!verify_tag("quiz", "q", "0", "00"));
    }

    #[test]
    fn verify_rejects_tampered_tags() {
        let tag = test_tag("nec250-001", 2);
        // Flip the first hex digit: a well-formed tag that matches
        // nothing must not verify.
        let mut bytes = tag.into_bytes();
        bytes[0] = if bytes[0] == b'0' { b'1' } else { b'0' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(!verify_tag("quiz", "nec250-001", "2", &tampered));
    }
}
