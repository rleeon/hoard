//! Anti-abuse helpers for cloud account provisioning: email canonicalization
//! (Sybil / alias dedup) and a disposable-domain blocklist.
//!
//! Cloud-only. Self-hosted never compiles `--features cloud`, so its
//! no-accounts / no-limits model is untouched by anything here. These are pure
//! functions + a constant; the SQL that uses them lives in
//! `cloud::routes::me` next to the profile upsert.

use std::collections::HashSet;
use std::sync::OnceLock;

/// Max distinct *free* accounts we let share one device fingerprint. Pro
/// accounts don't count toward it and are never blocked — they paid. Generous
/// on purpose: a household sharing one PC stays under it, while bulk
/// free-account farming on a single machine trips it. The fingerprint is
/// client-supplied and the client is open source, so this is a speed bump for
/// casual abuse, not a cryptographic barrier — don't over-invest in it.
pub const MAX_FREE_ACCOUNTS_PER_DEVICE: i64 = 3;

/// Collapse an email to a canonical identity so aliases of the same inbox map
/// to one value:
/// - always lowercased + trimmed;
/// - Gmail / Googlemail: drop everything after `+` in the local part, remove
///   dots from the local part, and normalize the domain to `gmail.com` (Gmail
///   ignores all three).
///
/// Deliberately conservative for non-Gmail providers: we do **not** strip
/// `+tags` there, because not every provider treats them as aliases and a false
/// merge would lock out a legitimate, distinct address. Returns `None` when the
/// input isn't a plausible `local@domain`.
pub fn canonicalize_email(email: &str) -> Option<String> {
    let email = email.trim().to_lowercase();
    let (local, domain) = email.split_once('@')?;
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return None;
    }
    if domain == "gmail.com" || domain == "googlemail.com" {
        let base = local.split('+').next().unwrap_or(local);
        let stripped: String = base.chars().filter(|c| *c != '.').collect();
        if stripped.is_empty() {
            return None;
        }
        Some(format!("{stripped}@gmail.com"))
    } else {
        Some(format!("{local}@{domain}"))
    }
}

/// Is the address on a known disposable / throwaway email provider? Matches the
/// domain against a curated set of the most common ones. Not exhaustive — a
/// coarse filter to raise the cost of bulk free signups, not a guarantee.
pub fn is_disposable_email(email: &str) -> bool {
    let domain = match email.trim().rsplit_once('@') {
        Some((_, d)) => d.trim().to_lowercase(),
        None => return false,
    };
    disposable_domains().contains(domain.as_str())
}

fn disposable_domains() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| DISPOSABLE.iter().copied().collect())
}

/// Curated list of common disposable-email domains. Kept inline (not a config
/// file) so the protection ships by default; extend it as new throwaway
/// providers show up in real signups.
const DISPOSABLE: &[&str] = &[
    "mailinator.com",
    "guerrillamail.com",
    "guerrillamail.info",
    "guerrillamailblock.com",
    "sharklasers.com",
    "grr.la",
    "spam4.me",
    "10minutemail.com",
    "10minutemail.net",
    "temp-mail.org",
    "tempmail.com",
    "tempmailo.com",
    "tempr.email",
    "tempinbox.com",
    "tmpmail.org",
    "tmpmail.net",
    "mytemp.email",
    "throwawaymail.com",
    "yopmail.com",
    "yopmail.fr",
    "getnada.com",
    "nada.email",
    "dispostable.com",
    "trashmail.com",
    "trashmail.de",
    "maildrop.cc",
    "mailnesia.com",
    "mailcatch.com",
    "mintemail.com",
    "mohmal.com",
    "fakeinbox.com",
    "spamgourmet.com",
    "emailondeck.com",
    "moakt.com",
    "inboxkitten.com",
    "burnermail.io",
    "anonbox.net",
    "pokemail.net",
    "vomoto.com",
    "33mail.com",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_dots_and_plus_collapse() {
        let want = Some("rai@gmail.com".to_string());
        assert_eq!(canonicalize_email("rai@gmail.com"), want);
        assert_eq!(canonicalize_email("r.a.i@gmail.com"), want);
        assert_eq!(canonicalize_email("rai+spam@gmail.com"), want);
        assert_eq!(canonicalize_email("R.Ai+Newsletter@Gmail.com"), want);
        // googlemail.com is the same inbox as gmail.com.
        assert_eq!(canonicalize_email("r.ai@googlemail.com"), want);
    }

    #[test]
    fn non_gmail_keeps_dots_and_tags() {
        // Other providers don't universally alias dots/+tags, so we must not
        // merge them or we'd block distinct legitimate addresses.
        assert_eq!(
            canonicalize_email("First.Last@outlook.com"),
            Some("first.last@outlook.com".to_string())
        );
        assert_eq!(
            canonicalize_email("user+tag@fastmail.com"),
            Some("user+tag@fastmail.com".to_string())
        );
    }

    #[test]
    fn malformed_addresses_have_no_canonical() {
        assert_eq!(canonicalize_email(""), None);
        assert_eq!(canonicalize_email("not-an-email"), None);
        assert_eq!(canonicalize_email("@gmail.com"), None);
        assert_eq!(canonicalize_email("user@"), None);
        // A gmail local part made entirely of dots/tag has nothing left.
        assert_eq!(canonicalize_email("...@gmail.com"), None);
    }

    #[test]
    fn disposable_detection() {
        assert!(is_disposable_email("throwaway@mailinator.com"));
        assert!(is_disposable_email("x@YOPMAIL.com")); // case-insensitive
        assert!(is_disposable_email("  a@guerrillamail.com  "));
        assert!(!is_disposable_email("real@gmail.com"));
        assert!(!is_disposable_email("user@company.io"));
        assert!(!is_disposable_email("garbage"));
    }
}
