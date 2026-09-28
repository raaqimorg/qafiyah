use std::net::{IpAddr, Ipv4Addr};
use std::sync::OnceLock;

use axum::http::HeaderMap;

use crate::constants::{CF_CONNECTING_IP_HEADER, TRUSTED_PROXY_NETWORKS};

const X_FORWARDED_FOR: &str = "x-forwarded-for";

pub fn resolve(headers: &HeaderMap) -> Option<IpAddr> {
    cloudflare(headers).or_else(|| last_forwarded_hop(headers))
}

pub fn resolve_from(headers: &HeaderMap, peer: Option<IpAddr>) -> Option<IpAddr> {
    match peer {
        Some(peer) if !is_trusted_proxy(peer) => Some(peer),
        _ => resolve(headers),
    }
}

pub fn forwarded(headers: &HeaderMap, peer: Option<IpAddr>) -> Option<IpAddr> {
    match peer {
        Some(peer) if !is_trusted_proxy(peer) => None,
        _ => cloudflare(headers),
    }
}

#[derive(Clone, Copy, Debug)]
struct Cidr {
    base: u32,
    mask: u32,
}

impl Cidr {
    fn parse(input: &str) -> Option<Self> {
        let (address, prefix) = input.split_once('/')?;
        let prefix: u32 = prefix.parse().ok()?;
        let ip: Ipv4Addr = address.parse().ok()?;
        if prefix > 32 {
            return None;
        }
        let mask = if prefix == 0 {
            0
        } else {
            u32::MAX.wrapping_shl(32u32.saturating_sub(prefix))
        };
        Some(Self {
            base: u32::from(ip) & mask,
            mask,
        })
    }

    fn contains(self, ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => (u32::from(ipv4) & self.mask) == self.base,
            IpAddr::V6(_) => false,
        }
    }
}

fn trusted_networks() -> &'static [Cidr] {
    static NETWORKS: OnceLock<Vec<Cidr>> = OnceLock::new();
    NETWORKS.get_or_init(|| {
        TRUSTED_PROXY_NETWORKS
            .iter()
            .filter_map(|network| Cidr::parse(network))
            .collect()
    })
}

pub fn is_trusted_proxy(peer: IpAddr) -> bool {
    trusted_networks()
        .iter()
        .any(|network| network.contains(peer))
}

fn cloudflare(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get(CF_CONNECTING_IP_HEADER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn last_forwarded_hop(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get(X_FORWARDED_FOR)?
        .to_str()
        .ok()?
        .rsplit(',')
        .find_map(|hop| hop.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, HeaderValue::from_str(value).expect("a valid header"));
        }
        map
    }

    #[test]
    fn cloudflares_header_wins_over_the_forwarded_chain() {
        let map = headers(&[
            ("cf-connecting-ip", "203.0.113.7"),
            ("x-forwarded-for", "198.51.100.1, 192.0.2.1"),
        ]);
        assert_eq!(resolve(&map), "203.0.113.7".parse().ok());
    }

    #[test]
    fn the_last_forwarded_hop_is_used_when_cloudflare_is_absent() {
        let map = headers(&[("x-forwarded-for", "198.51.100.1, 192.0.2.1")]);
        assert_eq!(resolve(&map), "192.0.2.1".parse().ok());
    }

    #[test]
    fn a_malformed_value_falls_through_to_the_next_source() {
        let map = headers(&[
            ("cf-connecting-ip", "not-an-address"),
            ("x-forwarded-for", "192.0.2.1"),
        ]);
        assert_eq!(resolve(&map), "192.0.2.1".parse().ok());
    }

    #[test]
    fn an_unproxied_request_resolves_to_nothing() {
        assert_eq!(resolve(&HeaderMap::new()), None);
    }

    #[test]
    fn an_ipv6_client_resolves() {
        let map = headers(&[("cf-connecting-ip", "2001:db8::1")]);
        assert_eq!(resolve(&map), "2001:db8::1".parse().ok());
    }

    #[test]
    fn resolution_never_panics_on_arbitrary_header_bytes() {
        let mut rng = crate::test_support::Rng::new(7);
        for _ in 0..3_000 {
            let mut map = HeaderMap::new();
            for name in ["cf-connecting-ip", "x-forwarded-for"] {
                if rng.below(2) == 1 {
                    let len = usize::try_from(rng.below(20)).expect("bounded length");
                    let bytes = rng.bytes(len);
                    if let Ok(value) = HeaderValue::from_bytes(&bytes) {
                        map.insert(name, value);
                    }
                }
            }
            let _ = resolve(&map);
        }
    }

    #[test]
    fn a_malformed_last_hop_falls_back_to_an_earlier_hop_pinned_not_endorsed() {
        let map = headers(&[("x-forwarded-for", "198.51.100.1, garbage")]);
        assert_eq!(resolve(&map), "198.51.100.1".parse().ok());
    }

    #[test]
    fn a_hop_with_a_port_is_skipped_and_only_the_first_header_line_is_read() {
        let map = headers(&[("x-forwarded-for", "198.51.100.1, 192.0.2.1:8080")]);
        assert_eq!(resolve(&map), "198.51.100.1".parse().ok());
        let mut two = HeaderMap::new();
        two.append("cf-connecting-ip", HeaderValue::from_static("203.0.113.1"));
        two.append("cf-connecting-ip", HeaderValue::from_static("203.0.113.2"));
        assert_eq!(resolve(&two), "203.0.113.1".parse().ok());
    }

    #[test]
    fn a_trusted_peer_keeps_the_header_address() {
        let map = headers(&[("cf-connecting-ip", "203.0.113.7")]);
        let peer = "172.27.0.9".parse::<IpAddr>().ok();
        assert_eq!(resolve_from(&map, peer), "203.0.113.7".parse().ok());
    }

    #[test]
    fn an_untrusted_peer_is_bucketed_on_its_own_address() {
        let map = headers(&[("cf-connecting-ip", "203.0.113.7")]);
        let peer = "172.18.0.4".parse::<IpAddr>().ok();
        assert_eq!(resolve_from(&map, peer), peer);
    }

    #[test]
    fn a_missing_peer_falls_back_to_the_header() {
        let map = headers(&[("cf-connecting-ip", "203.0.113.7")]);
        assert_eq!(resolve_from(&map, None), "203.0.113.7".parse().ok());
    }

    #[test]
    fn a_forwarded_address_is_read_from_a_trusted_peer_or_a_missing_one() {
        let map = headers(&[("cf-connecting-ip", "2001:db8::7")]);
        let peer = "172.27.0.9".parse::<IpAddr>().ok();
        assert_eq!(forwarded(&map, peer), "2001:db8::7".parse().ok());
        assert_eq!(forwarded(&map, None), "2001:db8::7".parse().ok());
    }

    #[test]
    fn an_untrusted_peer_never_forwards_an_address_not_even_its_own() {
        let map = headers(&[("cf-connecting-ip", "2001:db8::7")]);
        let peer = "127.0.0.1".parse::<IpAddr>().ok();
        assert_eq!(forwarded(&map, peer), None);
        assert_eq!(forwarded(&HeaderMap::new(), peer), None);
    }

    #[test]
    fn a_forwarded_address_is_never_read_from_x_forwarded_for() {
        let map = headers(&[("x-forwarded-for", "2001:db8::7")]);
        let peer = "172.27.0.9".parse::<IpAddr>().ok();
        assert_eq!(forwarded(&map, peer), None);
    }

    #[test]
    fn a_trusted_peer_without_a_forwarding_header_forwards_nothing() {
        let peer = "172.27.0.9".parse::<IpAddr>().ok();
        assert_eq!(forwarded(&HeaderMap::new(), peer), None);
    }

    #[test]
    fn the_dev_backend_subnet_is_also_trusted() {
        let map = headers(&[("cf-connecting-ip", "203.0.113.7")]);
        let peer = "172.26.0.5".parse::<IpAddr>().ok();
        assert_eq!(resolve_from(&map, peer), "203.0.113.7".parse().ok());
    }

    #[test]
    fn an_ipv6_peer_is_never_a_trusted_proxy() {
        let map = headers(&[("cf-connecting-ip", "203.0.113.7")]);
        let peer = "2001:db8::1".parse::<IpAddr>().ok();
        assert_eq!(resolve_from(&map, peer), peer);
    }
}
