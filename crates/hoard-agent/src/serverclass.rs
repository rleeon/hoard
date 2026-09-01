//! Classifying a Hoard server's URL, with no network. A pure heuristic on the
//! host: is it "self-hosted at home" (LAN/Tailscale/loopback) or an external
//! SaaS? Does it point at the managed Hoard Cloud backend? It lives in the agent
//! so desktop and CLI share exactly the same rule (the desktop re-exports it as
//! `classify_server`/`classify_cloud`).

/// Do we treat `url` as "self-hosted at home" (sizes in MB) rather than
/// "external SaaS" (a percentage of quota)?
///
/// A heuristic: loopback, an RFC1918 private IP, Tailscale's CGNAT block
/// (100.64.0.0/10), an mDNS `.local`, or a single-label host (a LAN box or a
/// MagicDNS name) means local. Everything else is external. At worst the user
/// sees a percentage where they wanted MB; both views show the same figure.
pub fn is_local_server(url: &str) -> bool {
    let host = match host_of(url) {
        Some(h) => h,
        None => return false,
    };

    if host == "localhost" || host == "127.0.0.1" || host == "::1" || host.ends_with(".local") {
        return true;
    }
    // RFC1918 private IPv4 ranges plus Tailscale's CGNAT (100.64.0.0/10), which
    // is a private overlay rather than public SaaS. v4 only; an IPv6 ULA
    // (`fc00::/7`) could be added here later.
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        let octs = ip.octets();
        let private = octs[0] == 10
            || (octs[0] == 172 && (16..=31).contains(&octs[1]))
            || (octs[0] == 192 && octs[1] == 168)
            || (octs[0] == 100 && (64..=127).contains(&octs[1]));
        return private;
    }
    // A single-label host (no dot, and not a literal IP or IPv6): a LAN box, or a
    // Tailscale MagicDNS name like `ubserver` or `nas`. Public SaaS always has an
    // FQDN with a TLD, so a bare host is treated as local. Otherwise a self-hoster
    // would see a fake quota bar against the schema's 100 GiB default instead of
    // the "X used" line for their own disk.
    if !host.contains('.') && !host.contains(':') {
        return true;
    }
    false
}

/// Does `url` point at the managed Hoard Cloud backend? The UI uses this to hide
/// the self-hosted upgrade button (Cloud has no `/v1/admin/upgrade` and updates
/// out of band). Matches `hoard.services`, any `*.hoard.services` subdomain, and
/// Fly.io hosts (`*.fly.dev`).
pub fn is_cloud_host(url: &str) -> bool {
    let host = match host_of(url) {
        Some(h) => h,
        None => return false,
    };
    host == "hoard.services" || host.ends_with(".hoard.services") || host.ends_with(".fly.dev")
}

/// Extracts the lowercase host from an `http(s)://host[:port][/path]` URL.
/// `None` si no reconoce el esquema.
/// Drop a `user@` (or `user:pass@`) prefix from a server URL, and any trailing
/// slash.
///
/// Nothing in Hoard's API uses HTTP Basic auth (the access key travels as a
/// bearer token) but reqwest turns URL credentials into a `basic_auth` call on
/// **every** request it builds, and its `header()` *appends*, so the request
/// goes out with two `Authorization` headers: `Basic` first, then our `Bearer`.
/// The server reads the first one, sees no bearer token, and answers 401.
///
/// The result is a login that can never succeed and blames the one thing that
/// is fine: "token rejected by server (401)", for a token that works from curl.
/// A user pasted `http://insider@ubserver:12421` (ago-2026) and neither the CLI
/// nor the app could tell them why.
///
/// So the credentials are stripped rather than rejected: they are meaningless
/// to this API, and `ssh`-shaped addresses are a habit, not a mistake worth an
/// error message.
pub fn normalize_server_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    let Some(scheme_len) = ["https://", "http://"]
        .iter()
        .find(|p| trimmed.starts_with(**p))
        .map(|p| p.len())
    else {
        return trimmed.to_string();
    };
    let (scheme, rest) = trimmed.split_at(scheme_len);
    let (authority, path) = match rest.find('/') {
        Some(i) => rest.split_at(i),
        None => (rest, ""),
    };
    // The **last** `@`: a percent-encoded password may carry one of its own.
    let authority = authority
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(authority);
    format!("{scheme}{authority}{path}")
}

fn host_of(url: &str) -> Option<String> {
    let url = normalize_server_url(url);
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    Some(
        rest.split('/')
            .next()
            .unwrap_or(rest)
            .split(':')
            .next()
            .unwrap_or(rest)
            .trim_end_matches('.')
            .to_lowercase(),
    )
}

#[cfg(test)]
mod tests {
    use super::{is_cloud_host, is_local_server, normalize_server_url};

    /// A `user@` in the address made every request carry two `Authorization`
    /// headers (reqwest's, from the URL credentials, plus ours) and the server
    /// read the wrong one: a permanent 401 with a perfectly good token.
    #[test]
    fn credentials_never_survive_into_the_url() {
        assert_eq!(
            normalize_server_url("http://insider@ubserver:12421"),
            "http://ubserver:12421"
        );
        assert_eq!(
            normalize_server_url("https://me:secret@hoard.example.com/"),
            "https://hoard.example.com"
        );
        assert_eq!(
            normalize_server_url("http://ubserver:12421/"),
            "http://ubserver:12421"
        );
        // The `@` belongs to the path here, not to any credentials.
        assert_eq!(
            normalize_server_url("http://ubserver:12421/a@b"),
            "http://ubserver:12421/a@b"
        );
        // Not a URL we recognise: hand it back untouched rather than mangle it.
        assert_eq!(normalize_server_url("ubserver:12421"), "ubserver:12421");
    }

    /// The classification runs on the real host, not on `insider@ubserver`.
    #[test]
    fn a_username_doesnt_confuse_the_classifier() {
        assert!(is_local_server("http://insider@ubserver:12421"));
        assert!(is_cloud_host("https://someone@api.hoard.services"));
    }

    #[test]
    fn localhost_and_loopback_are_local() {
        assert!(is_local_server("http://localhost:8082"));
        assert!(is_local_server("http://127.0.0.1:8082"));
        assert!(is_local_server("http://[::1]:8082"));
        assert!(is_local_server("http://nas.local"));
    }

    #[test]
    fn rfc1918_and_tailscale_cgnat_are_local() {
        assert!(is_local_server("http://10.0.0.5:12421"));
        assert!(is_local_server("http://172.16.4.4"));
        assert!(is_local_server("http://192.168.1.10"));
        // CGNAT de Tailscale 100.64.0.0/10.
        assert!(is_local_server("http://100.100.1.1:12421"));
        assert!(is_local_server("http://100.127.255.254"));
    }

    #[test]
    fn single_label_lan_hostnames_are_local() {
        assert!(is_local_server("http://ubserver:12421"));
        assert!(is_local_server("http://homelab"));
    }

    #[test]
    fn public_hosts_are_external() {
        assert!(!is_local_server("https://saves.example.com"));
        assert!(!is_local_server("https://hoard.services"));
        // 100.63.x falls just outside the CGNAT block; 8.8.8.8 is public.
        assert!(!is_local_server("http://100.63.0.1"));
        assert!(!is_local_server("http://8.8.8.8"));
    }

    #[test]
    fn cloud_hosts_detected() {
        assert!(is_cloud_host("https://hoard.services"));
        assert!(is_cloud_host("https://api.hoard.services"));
        assert!(is_cloud_host("https://hoard-server.fly.dev"));
        assert!(!is_cloud_host("http://localhost:8082"));
        assert!(!is_cloud_host("https://saves.example.com"));
    }
}
