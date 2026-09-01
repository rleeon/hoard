//! Who is this request actually from?
//!
//! `X-Forwarded-For` is a claim, not a fact. Anyone who can open a socket to
//! the server can write whatever they like in it, and the only thing that makes
//! it trustworthy is a reverse proxy the operator runs, overwriting it on the
//! way in. So the header is read **only** when the peer on the other end of the
//! socket is one of the proxies the operator named
//! ([`crate::config::ServerConfig::trusted_proxies`]); from anyone else the
//! connection's own address is the answer.
//!
//! Reading it unconditionally is what made the panel's login throttle a
//! decoration: both of its counters are keyed on the client address, so a
//! direct caller, the normal shape on a LAN with no proxy in front, rotated
//! the header on every attempt, landed in a fresh bucket each time and was
//! never refused. What was left standing between an attacker and the server was
//! an argon2id verify at 19 MiB a go, which is the CPU lever the throttle
//! exists to keep shut.
//!
//! Not used by the per-IP rate limiter yet: that one keys through
//! `tower_governor`'s `SmartIpKeyExtractor`, which has the same unconditional
//! trust. Moving it here would silently collapse every client of a proxied
//! deployment into one token bucket until its operator sets `trusted_proxies`,
//! and that trade, a sync-breaking 429 storm against a limiter that was never a
//! security boundary ("an in-process safety net against accidental request
//! loops", [`crate::ratelimit`]), is not worth taking in the same release that
//! introduces the setting.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use axum::http::HeaderMap;

/// The networks whose `X-Forwarded-For` we believe, resolved once at boot.
///
/// Empty means "believe nobody", which is also what an operator gets by writing
/// `trusted_proxies = []`: every request is attributed to the address it came
/// from.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    nets: Vec<Net>,
    /// `trusted_proxies = ["any"]`, the pre-1.1.4 behaviour, kept reachable
    /// for a deployment that terminates somewhere we can't enumerate.
    any: bool,
}

#[derive(Debug, Clone, Copy)]
struct Net {
    addr: IpAddr,
    prefix: u8,
}

/// One unparseable entry, so the caller can say which one at boot instead of
/// leaving the operator with a setting that silently did nothing.
#[derive(Debug)]
pub struct BadEntry {
    pub entry: String,
    pub why: &'static str,
}

impl TrustedProxies {
    /// Parse the config list. Accepts a bare address (`10.0.0.5`), a CIDR
    /// (`172.16.0.0/12`, `fd00::/8`) or one of the shorthands:
    ///
    /// * `loopback`: `127.0.0.0/8` and `::1`. The default, and what covers the
    ///   common single-box shape: nginx/Caddy on the same host, proxying to
    ///   `127.0.0.1:12421`.
    /// * `private`: the RFC 1918 ranges, link-local, CGNAT (Tailscale) and
    ///   IPv6 ULA. What a proxy in another container or elsewhere on the LAN
    ///   needs. Note it also trusts every *other* machine on that LAN, so it is
    ///   opt-in rather than the default.
    /// * `any`: everyone. Only for a deployment whose proxy address can't be
    ///   pinned down; it puts the header back in the caller's hands.
    ///
    /// Bad entries are returned rather than defaulted away: a typo here fails
    /// open in the direction of "your proxy isn't trusted", which is safe but
    /// invisible, and an operator deserves to be told.
    pub fn parse(entries: &[String]) -> (Self, Vec<BadEntry>) {
        let mut out = Self::default();
        let mut bad = Vec::new();
        for raw in entries {
            let entry = raw.trim();
            if entry.is_empty() {
                continue;
            }
            match entry.to_ascii_lowercase().as_str() {
                "any" | "all" | "*" => {
                    out.any = true;
                    continue;
                }
                "loopback" | "localhost" => {
                    out.nets.push(net("127.0.0.0", 8));
                    out.nets.push(net("::1", 128));
                    continue;
                }
                "private" => {
                    for (a, p) in [
                        ("10.0.0.0", 8),
                        ("172.16.0.0", 12),
                        ("192.168.0.0", 16),
                        // Tailscale and other CGNAT overlays.
                        ("100.64.0.0", 10),
                        ("169.254.0.0", 16),
                        // IPv6 unique-local and link-local.
                        ("fc00::", 7),
                        ("fe80::", 10),
                    ] {
                        out.nets.push(net(a, p));
                    }
                    continue;
                }
                _ => {}
            }
            match parse_cidr(entry) {
                Ok(n) => out.nets.push(n),
                Err(why) => bad.push(BadEntry {
                    entry: entry.to_string(),
                    why,
                }),
            }
        }
        (out, bad)
    }

    /// Is this peer allowed to tell us who the client is?
    pub fn contains(&self, ip: IpAddr) -> bool {
        if self.any {
            return true;
        }
        // A dual-stack listener hands us `::ffff:10.0.0.1` for an IPv4 peer, so
        // the v4 rules have to see through the mapping or a Docker bridge
        // address never matches `172.16.0.0/12`.
        let ip = ip.to_canonical();
        self.nets.iter().any(|n| n.contains(ip))
    }
}

impl std::fmt::Display for TrustedProxies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.any {
            return f.write_str("any");
        }
        if self.nets.is_empty() {
            return f.write_str("none");
        }
        let list: Vec<String> = self
            .nets
            .iter()
            .map(|n| format!("{}/{}", n.addr, n.prefix))
            .collect();
        f.write_str(&list.join(", "))
    }
}

impl Net {
    fn contains(&self, ip: IpAddr) -> bool {
        match (self.addr, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                prefix_eq(&net.octets(), &ip.octets(), self.prefix)
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                prefix_eq(&net.octets(), &ip.octets(), self.prefix)
            }
            // A v4 rule never matches a v6 address or the other way round.
            // `to_canonical` above has already unwrapped the mapped ones.
            _ => false,
        }
    }
}

/// Do `a` and `b` agree on their first `bits` bits?
fn prefix_eq(a: &[u8], b: &[u8], bits: u8) -> bool {
    let bits = usize::from(bits).min(a.len() * 8);
    let whole = bits / 8;
    if a[..whole] != b[..whole] {
        return false;
    }
    let rest = bits % 8;
    if rest == 0 {
        return true;
    }
    let mask = 0xffu8 << (8 - rest);
    a[whole] & mask == b[whole] & mask
}

/// Panics only on the literals above, which are covered by the tests.
fn net(addr: &str, prefix: u8) -> Net {
    Net {
        addr: addr.parse().expect("built-in shorthand network"),
        prefix,
    }
}

fn parse_cidr(entry: &str) -> Result<Net, &'static str> {
    let (addr_part, prefix_part) = match entry.split_once('/') {
        Some((a, p)) => (a, Some(p)),
        None => (entry, None),
    };
    let addr: IpAddr = addr_part.trim().parse().map_err(|_| "not an IP address")?;
    let max = if addr.is_ipv4() { 32 } else { 128 };
    let prefix = match prefix_part {
        // A bare address is that one host.
        None => max,
        Some(p) => {
            let n: u8 = p
                .trim()
                .parse()
                .map_err(|_| "prefix length is not a number")?;
            if n > max {
                return Err("prefix length is too long for the address family");
            }
            n
        }
    };
    Ok(Net { addr, prefix })
}

/// The address to attribute this request to.
///
/// From a trusted proxy, the leftmost entry of `X-Forwarded-For` (or
/// `X-Real-Ip`), which is the client the proxy saw. From anyone else, the peer,
/// because
/// anything they claim about themselves is theirs to make up. An unparseable
/// claim also falls back to the peer: it would otherwise become a throttle-map
/// key of arbitrary attacker-chosen text.
pub fn client_ip(headers: &HeaderMap, peer: SocketAddr, trusted: &TrustedProxies) -> IpAddr {
    let peer_ip = peer.ip();
    if !trusted.contains(peer_ip) {
        return peer_ip;
    }
    for h in ["x-forwarded-for", "x-real-ip"] {
        let Some(v) = headers.get(h).and_then(|v| v.to_str().ok()) else {
            continue;
        };
        let Some(first) = v.split(',').next() else {
            continue;
        };
        // `[2001:db8::1]:443` is legal in a `Forwarded`-style header and shows
        // up in the wild in `X-Forwarded-For` too.
        let first = first.trim().trim_start_matches('[');
        let first = first.split(']').next().unwrap_or(first);
        if let Ok(ip) = first.parse::<IpAddr>() {
            return ip;
        }
    }
    peer_ip
}

/// The key a per-origin counter should use for this address.
///
/// IPv4 is one address, one bucket. IPv6 is bucketed by **/64**, because that
/// is the smallest thing an ISP hands out: counting per address there would let
/// one household walk through 2^64 fresh buckets, which is the same bypass as
/// the spoofed header with extra steps.
pub fn throttle_bucket(ip: IpAddr) -> String {
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            let net = Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0);
            format!("{net}/64")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn peer(s: &str) -> SocketAddr {
        SocketAddr::new(ip(s), 40000)
    }

    fn trusted(entries: &[&str]) -> TrustedProxies {
        let owned: Vec<String> = entries.iter().map(|s| s.to_string()).collect();
        let (t, bad) = TrustedProxies::parse(&owned);
        assert!(bad.is_empty(), "{bad:?}");
        t
    }

    fn xff(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", value.parse().unwrap());
        h
    }

    /// The bug this module exists for: a direct caller rotating the header must
    /// keep landing in the same bucket.
    #[test]
    fn a_direct_caller_cannot_choose_its_own_address() {
        let t = trusted(&["loopback"]);
        assert_eq!(
            client_ip(&xff("1.2.3.4"), peer("192.168.1.50"), &t),
            ip("192.168.1.50")
        );
        assert_eq!(
            client_ip(&xff("9.9.9.9"), peer("192.168.1.50"), &t),
            ip("192.168.1.50")
        );
    }

    #[test]
    fn a_trusted_proxy_is_believed_and_read_from_the_left() {
        let t = trusted(&["loopback"]);
        assert_eq!(
            client_ip(&xff("1.2.3.4, 10.0.0.1"), peer("127.0.0.1"), &t),
            ip("1.2.3.4")
        );
    }

    /// Garbage in the header falls back to the peer instead of becoming a
    /// counter key of the attacker's choosing.
    #[test]
    fn an_unparseable_claim_falls_back_to_the_peer() {
        let t = trusted(&["loopback"]);
        assert_eq!(
            client_ip(&xff("not-an-ip"), peer("127.0.0.1"), &t),
            ip("127.0.0.1")
        );
        assert_eq!(client_ip(&xff(""), peer("127.0.0.1"), &t), ip("127.0.0.1"));
    }

    #[test]
    fn a_bracketed_v6_claim_parses() {
        let t = trusted(&["loopback"]);
        assert_eq!(
            client_ip(&xff("[2001:db8::5]:443"), peer("127.0.0.1"), &t),
            ip("2001:db8::5")
        );
    }

    /// A dual-stack listener reports an IPv4 peer as `::ffff:a.b.c.d`; the v4
    /// rules have to see through that or a Docker bridge never matches.
    #[test]
    fn a_mapped_v4_peer_matches_a_v4_rule() {
        let t = trusted(&["172.16.0.0/12"]);
        assert!(t.contains(ip("::ffff:172.18.0.3")));
        assert!(t.contains(ip("172.18.0.3")));
        assert!(!t.contains(ip("172.32.0.1")));
    }

    #[test]
    fn shorthands_cover_what_they_claim() {
        let lo = trusted(&["loopback"]);
        assert!(lo.contains(ip("127.0.0.1")));
        assert!(lo.contains(ip("127.1.2.3")));
        assert!(lo.contains(ip("::1")));
        assert!(!lo.contains(ip("10.1.2.3")));

        let priv_ = trusted(&["private"]);
        for a in ["10.1.2.3", "172.20.0.9", "192.168.0.7", "100.100.1.1"] {
            assert!(priv_.contains(ip(a)), "{a}");
        }
        assert!(priv_.contains(ip("fd12::1")));
        assert!(!priv_.contains(ip("8.8.8.8")));
        assert!(!priv_.contains(ip("2001:db8::1")));
    }

    #[test]
    fn an_empty_list_trusts_nobody_and_any_trusts_everybody() {
        let none = trusted(&[]);
        assert!(!none.contains(ip("127.0.0.1")));
        assert_eq!(none.to_string(), "none");

        let all = trusted(&["any"]);
        assert!(all.contains(ip("8.8.8.8")));
        assert_eq!(
            client_ip(&xff("1.2.3.4"), peer("8.8.8.8"), &all),
            ip("1.2.3.4")
        );
    }

    #[test]
    fn a_typo_is_reported_rather_than_ignored() {
        let entries = vec![
            "loopback".to_string(),
            "10.0.0.0/33".to_string(),
            "nope".into(),
        ];
        let (t, bad) = TrustedProxies::parse(&entries);
        assert!(t.contains(ip("127.0.0.1")));
        assert_eq!(bad.len(), 2);
    }

    /// One household's /64 is one bucket, or the per-origin budget is a
    /// formality on IPv6.
    #[test]
    fn v6_buckets_by_prefix_and_v4_by_address() {
        assert_eq!(
            throttle_bucket(ip("2001:db8:1:2::1")),
            throttle_bucket(ip("2001:db8:1:2:ffff::9"))
        );
        assert_ne!(
            throttle_bucket(ip("2001:db8:1:2::1")),
            throttle_bucket(ip("2001:db8:1:3::1"))
        );
        assert_eq!(throttle_bucket(ip("10.0.0.1")), "10.0.0.1");
        // A mapped v4 address buckets as the v4 it is.
        assert_eq!(throttle_bucket(ip("::ffff:10.0.0.1")), "10.0.0.1");
    }
}
