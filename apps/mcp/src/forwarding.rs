use std::net::IpAddr;

use axum::http::HeaderMap;
use ipnet::IpNet;

use crate::constants::CF_CONNECTING_IP_HEADER;

pub fn caller_address(
    headers: &HeaderMap,
    peer: Option<IpAddr>,
    trusted: &[IpNet],
) -> Option<IpAddr> {
    let peer = peer?;
    if !trusted.iter().any(|network| network.contains(&peer)) {
        return None;
    }
    headers
        .get(CF_CONNECTING_IP_HEADER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn trusted() -> Vec<IpNet> {
        vec!["172.27.0.0/24".parse().expect("a network")]
    }

    fn header(value: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(value) = value {
            headers.insert(
                "cf-connecting-ip",
                HeaderValue::from_str(value).expect("a header value"),
            );
        }
        headers
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().expect("an address")
    }

    #[test]
    fn a_trusted_peer_forwards_the_cloudflare_address() {
        let peer = Some(ip("172.27.0.5"));
        assert_eq!(
            caller_address(&header(Some("203.0.113.7")), peer, &trusted()),
            Some(ip("203.0.113.7"))
        );
        assert_eq!(
            caller_address(&header(Some(" 2001:db8::1 ")), peer, &trusted()),
            Some(ip("2001:db8::1"))
        );
    }

    #[test]
    fn an_untrusted_or_unknown_peer_forwards_nothing() {
        let headers = header(Some("203.0.113.7"));
        assert_eq!(
            caller_address(&headers, Some(ip("198.51.100.7")), &trusted()),
            None
        );
        assert_eq!(caller_address(&headers, None, &trusted()), None);
        assert_eq!(caller_address(&headers, Some(ip("172.27.0.5")), &[]), None);
    }

    #[test]
    fn a_malformed_or_listed_address_is_never_forwarded() {
        let peer = Some(ip("172.27.0.5"));
        for value in [Some("not-an-ip"), Some("203.0.113.1, 203.0.113.2"), None] {
            assert_eq!(
                caller_address(&header(value), peer, &trusted()),
                None,
                "{value:?}"
            );
        }
    }
}
