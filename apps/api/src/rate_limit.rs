use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard};

use axum::extract::{ConnectInfo, OriginalUri, Request, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::accounts::keys::Caller;
use crate::client_ip;
use crate::constants::{
    API_KEY_HEADER, BURST_WINDOW_SECONDS, RATE_LIMIT_LIMIT_HEADER, RATE_LIMIT_MAX_TRACKED,
    RATE_LIMIT_REMAINING_HEADER, RATE_LIMIT_RESET_HEADER, RATE_LIMIT_SWEEP_SECONDS,
    SECONDS_PER_HOUR, WINDOW_SECONDS,
};
use crate::error::AppError;
use crate::log::LogHandle;
use crate::state::AppState;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Bucket {
    User(i64),
    UserBurst(i64),
    KeyedIp(IpAddr),
    Ip(IpAddr),
    Unknown,
}

#[derive(Clone, Copy, Debug)]
pub struct Limit {
    pub bucket: Bucket,
    pub ceiling: u32,
    pub window: i64,
}

#[derive(Clone, Copy, Debug)]
pub struct Outcome {
    pub allowed: bool,
    pub sustained: Decision,
    pub refused: Option<Decision>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Decision {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub reset: i64,
}

struct Window {
    expires: i64,
    count: u32,
}

#[derive(Default)]
pub struct Limiter {
    windows: Mutex<HashMap<Bucket, Window>>,
}

fn hour_of(now: i64) -> i64 {
    now.div_euclid(SECONDS_PER_HOUR)
}

fn bucket_address(address: IpAddr) -> IpAddr {
    match address.to_canonical() {
        IpAddr::V4(v4) => IpAddr::V4(v4),
        IpAddr::V6(v6) => {
            let [a, b, c, d, ..] = v6.segments();
            IpAddr::V6(Ipv6Addr::new(a, b, c, d, 0, 0, 0, 0))
        }
    }
}

fn limits_for(caller: Option<Caller>, address: Option<IpAddr>, anon: u32) -> Vec<Limit> {
    let address = address.map(bucket_address);
    let Some(caller) = caller else {
        return vec![Limit {
            bucket: address.map_or(Bucket::Unknown, Bucket::Ip),
            ceiling: anon,
            window: WINDOW_SECONDS,
        }];
    };

    let mut limits = vec![
        Limit {
            bucket: Bucket::User(caller.user_id),
            ceiling: caller.requests,
            window: WINDOW_SECONDS,
        },
        Limit {
            bucket: Bucket::UserBurst(caller.user_id),
            ceiling: caller.burst,
            window: BURST_WINDOW_SECONDS,
        },
    ];
    if let (Some(address), Some(ceiling)) = (address, caller.ip_ceiling) {
        limits.push(Limit {
            bucket: Bucket::KeyedIp(address),
            ceiling,
            window: WINDOW_SECONDS,
        });
    }
    limits
}

impl Limiter {
    fn windows(&self) -> MutexGuard<'_, HashMap<Bucket, Window>> {
        self.windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn bound(windows: &mut HashMap<Bucket, Window>, now: i64) {
        if windows.len() < RATE_LIMIT_MAX_TRACKED {
            return;
        }
        windows.retain(|_, window| now < window.expires);
        if windows.len() >= RATE_LIMIT_MAX_TRACKED {
            windows.clear();
        }
    }

    #[expect(
        clippy::expect_used,
        reason = "the pass above inserts every bucket, and limits always carries the sustained limit"
    )]
    pub fn check_all(&self, limits: &[Limit], now: i64) -> Outcome {
        let mut windows = self.windows();
        Self::bound(&mut windows, now);
        let mut allowed = true;

        for limit in limits {
            let expires = now
                .div_euclid(limit.window)
                .saturating_add(1)
                .saturating_mul(limit.window);
            let window = windows
                .entry(limit.bucket)
                .or_insert(Window { expires, count: 0 });
            if window.expires != expires {
                window.expires = expires;
                window.count = 0;
            }
            if window.count >= limit.ceiling {
                allowed = false;
            }
        }

        let decisions: Vec<Decision> = limits
            .iter()
            .map(|limit| {
                let window = windows
                    .get_mut(&limit.bucket)
                    .expect("every bucket was inserted in the pass above");
                let refused = window.count >= limit.ceiling;
                if allowed {
                    window.count = window.count.saturating_add(1);
                }
                Decision {
                    allowed: !refused,
                    limit: limit.ceiling,
                    remaining: limit.ceiling.saturating_sub(window.count),
                    reset: window.expires,
                }
            })
            .collect();

        let sustained = decisions
            .first()
            .copied()
            .expect("limits always carries the sustained limit as its first entry");
        Outcome {
            allowed,
            sustained,
            refused: decisions.into_iter().find(|decision| !decision.allowed),
        }
    }

    pub fn sweep(&self, now: i64) {
        self.windows().retain(|_, window| now < window.expires);
    }

    pub fn tracked(&self) -> usize {
        self.windows().len()
    }
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(0))
}

fn set(headers: &mut HeaderMap, name: impl header::IntoHeaderName, value: i64) {
    if let Ok(value) = HeaderValue::from_str(&value.to_string()) {
        headers.insert(name, value);
    }
}

fn annotate(headers: &mut HeaderMap, decision: &Decision) {
    set(headers, RATE_LIMIT_LIMIT_HEADER, i64::from(decision.limit));
    set(
        headers,
        RATE_LIMIT_REMAINING_HEADER,
        i64::from(decision.remaining),
    );
    set(headers, RATE_LIMIT_RESET_HEADER, decision.reset);
}

fn retry_after(decision: &Decision, now: i64) -> i64 {
    decision.reset.saturating_sub(now).max(1)
}

pub async fn layer(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let key = request
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|value| value.to_str().ok());
    if state.keys.is_unlimited(key) {
        if let Some(log) = request.extensions().get::<LogHandle>() {
            log.set("keyed", true);
        }
        return next.run(request).await;
    }

    let now = now_seconds();
    let caller = match key {
        Some(key) if crate::accounts::keys::is_well_formed(key) => state
            .key_cache
            .resolve(&state.accounts, key, now)
            .await
            .unwrap_or(None),
        _ => None,
    };

    let limits = limits_for(
        caller,
        client_ip::resolve_from(
            request.headers(),
            request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(peer)| peer.ip()),
        ),
        state.anon_requests,
    );
    let outcome = state.limiter.check_all(&limits, now);

    if let Some(caller) = caller
        && outcome.allowed
    {
        state.usage.record(caller.key_id, hour_of(now));
    }

    if let Some(log) = request.extensions().get::<LogHandle>() {
        log.set(
            "rate_limit_remaining",
            i64::from(outcome.sustained.remaining),
        );
        log.set("keyed", caller.is_some());
        if let Some(caller) = caller {
            log.set("api_key_id", caller.key_id);
        }
    }

    if !outcome.allowed {
        let path = request.extensions().get::<OriginalUri>().map_or_else(
            || request.uri().path().to_string(),
            |uri| uri.path().to_string(),
        );
        let refused = outcome.refused.unwrap_or(outcome.sustained);
        let mut response = AppError::TooManyRequests.render_at(&path);
        annotate(response.headers_mut(), &outcome.sustained);
        set(
            response.headers_mut(),
            header::RETRY_AFTER,
            retry_after(&refused, now),
        );
        return response;
    }

    let mut response = next.run(request).await;
    annotate(response.headers_mut(), &outcome.sustained);
    response
}

pub fn sweeper(limiter: Arc<Limiter>) {
    tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval(std::time::Duration::from_secs(RATE_LIMIT_SWEEP_SECONDS));
        loop {
            ticker.tick().await;
            limiter.sweep(now_seconds());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3_600;

    fn ip(last: u8) -> Bucket {
        Bucket::Ip(IpAddr::from([192, 0, 2, last]))
    }

    fn one(bucket: Bucket, ceiling: u32) -> Vec<Limit> {
        vec![Limit {
            bucket,
            ceiling,
            window: HOUR,
        }]
    }

    fn keyed(user: i64, sustained: u32, burst: u32) -> Vec<Limit> {
        vec![
            Limit {
                bucket: Bucket::User(user),
                ceiling: sustained,
                window: HOUR,
            },
            Limit {
                bucket: Bucket::UserBurst(user),
                ceiling: burst,
                window: 1,
            },
        ]
    }

    fn caller(ip_ceiling: Option<u32>) -> Caller {
        Caller {
            key_id: 7,
            user_id: 1,
            requests: 500,
            burst: 10,
            ip_ceiling,
        }
    }

    #[test]
    fn the_first_request_in_a_window_is_allowed() {
        let limiter = Limiter::default();
        let outcome = limiter.check_all(&one(ip(1), 3), 0);
        assert!(outcome.allowed);
        assert_eq!(outcome.sustained.limit, 3);
        assert_eq!(outcome.sustained.remaining, 2);
    }

    #[test]
    fn a_bucket_is_refused_once_it_passes_its_ceiling() {
        let limiter = Limiter::default();
        for _ in 0..3 {
            assert!(limiter.check_all(&one(ip(1), 3), 0).allowed);
        }
        let outcome = limiter.check_all(&one(ip(1), 3), 0);
        assert!(!outcome.allowed);
        assert_eq!(outcome.sustained.remaining, 0);
    }

    #[test]
    fn a_new_window_resets_the_count() {
        let limiter = Limiter::default();
        for _ in 0..3 {
            limiter.check_all(&one(ip(1), 3), 0);
        }
        assert!(!limiter.check_all(&one(ip(1), 3), HOUR - 1).allowed);
        assert!(limiter.check_all(&one(ip(1), 3), HOUR).allowed);
    }

    #[test]
    fn two_buckets_do_not_share_a_window() {
        let limiter = Limiter::default();
        for _ in 0..3 {
            limiter.check_all(&one(ip(1), 3), 0);
        }
        assert!(!limiter.check_all(&one(ip(1), 3), 0).allowed);
        assert!(limiter.check_all(&one(ip(2), 3), 0).allowed);
    }

    #[test]
    fn unresolvable_callers_share_one_bucket() {
        let limiter = Limiter::default();
        assert!(limiter.check_all(&one(Bucket::Unknown, 1), 0).allowed);
        assert!(!limiter.check_all(&one(Bucket::Unknown, 1), 0).allowed);
    }

    #[test]
    fn reset_points_at_the_end_of_the_current_window() {
        let limiter = Limiter::default();
        assert_eq!(limiter.check_all(&one(ip(1), 3), 0).sustained.reset, HOUR);
        assert_eq!(
            limiter.check_all(&one(ip(1), 3), HOUR + 59).sustained.reset,
            2 * HOUR
        );
    }

    #[test]
    fn a_burst_refusal_does_not_consume_the_sustained_allowance() {
        let limiter = Limiter::default();
        for _ in 0..2 {
            assert!(limiter.check_all(&keyed(1, 100, 2), 0).allowed);
        }
        let refused = limiter.check_all(&keyed(1, 100, 2), 0);
        assert!(!refused.allowed);
        assert_eq!(refused.sustained.remaining, 98);
    }

    #[test]
    fn the_burst_window_reopens_a_second_later() {
        let limiter = Limiter::default();
        for _ in 0..2 {
            limiter.check_all(&keyed(1, 100, 2), 0);
        }
        assert!(!limiter.check_all(&keyed(1, 100, 2), 0).allowed);
        assert!(limiter.check_all(&keyed(1, 100, 2), 1).allowed);
    }

    #[test]
    fn the_refused_decision_names_the_limit_that_refused() {
        let limiter = Limiter::default();
        for _ in 0..2 {
            limiter.check_all(&keyed(1, 100, 2), 0);
        }
        let refused = limiter.check_all(&keyed(1, 100, 2), 0);
        let hit = refused.refused.expect("a refusing limit");
        assert_eq!(hit.limit, 2);
        assert_eq!(hit.reset, 1);
    }

    #[test]
    fn two_keys_belonging_to_one_user_share_a_window() {
        let limiter = Limiter::default();
        for _ in 0..2 {
            assert!(limiter.check_all(&keyed(1, 2, 100), 0).allowed);
        }
        assert!(!limiter.check_all(&keyed(1, 2, 100), 0).allowed);
    }

    #[test]
    fn a_fresh_key_does_not_reset_an_exhausted_users_window() {
        let limiter = Limiter::default();
        for _ in 0..2 {
            limiter.check_all(&keyed(1, 2, 100), 0);
        }
        let after_rotation = limiter.check_all(&keyed(1, 2, 100), 0);
        assert!(!after_rotation.allowed);
        assert_eq!(after_rotation.sustained.remaining, 0);
    }

    #[test]
    fn an_address_refusal_does_not_consume_either_user_allowance() {
        let limiter = Limiter::default();
        let address = IpAddr::from([192, 0, 2, 9]);
        let set = |user| {
            vec![
                Limit {
                    bucket: Bucket::User(user),
                    ceiling: 100,
                    window: HOUR,
                },
                Limit {
                    bucket: Bucket::UserBurst(user),
                    ceiling: 50,
                    window: 1,
                },
                Limit {
                    bucket: Bucket::KeyedIp(address),
                    ceiling: 2,
                    window: HOUR,
                },
            ]
        };
        for _ in 0..2 {
            assert!(limiter.check_all(&set(1), 0).allowed);
        }
        let refused = limiter.check_all(&set(1), 0);
        assert!(!refused.allowed);
        assert_eq!(refused.sustained.remaining, 98);
    }

    #[test]
    fn capped_users_on_one_address_exhaust_a_shared_ceiling_together() {
        let limiter = Limiter::default();
        let address = IpAddr::from([192, 0, 2, 9]);
        let capped = |user| {
            vec![
                Limit {
                    bucket: Bucket::User(user),
                    ceiling: 100,
                    window: HOUR,
                },
                Limit {
                    bucket: Bucket::KeyedIp(address),
                    ceiling: 2,
                    window: HOUR,
                },
            ]
        };
        assert!(limiter.check_all(&capped(1), 0).allowed);
        assert!(limiter.check_all(&capped(2), 0).allowed);
        assert!(!limiter.check_all(&capped(3), 0).allowed);
    }

    #[test]
    fn an_uncapped_user_is_untouched_by_an_exhausted_address() {
        let limiter = Limiter::default();
        let address = IpAddr::from([192, 0, 2, 9]);
        let capped = vec![
            Limit {
                bucket: Bucket::User(1),
                ceiling: 100,
                window: HOUR,
            },
            Limit {
                bucket: Bucket::KeyedIp(address),
                ceiling: 1,
                window: HOUR,
            },
        ];
        let uncapped = vec![Limit {
            bucket: Bucket::User(2),
            ceiling: 100,
            window: HOUR,
        }];
        assert!(limiter.check_all(&capped, 0).allowed);
        assert!(!limiter.check_all(&capped, 0).allowed);
        assert!(limiter.check_all(&uncapped, 0).allowed);
    }

    #[test]
    fn the_sweep_drops_expired_windows_of_every_length() {
        let limiter = Limiter::default();
        limiter.check_all(&keyed(1, 100, 2), 0);
        limiter.check_all(&one(ip(1), 3), 0);
        assert_eq!(limiter.tracked(), 3);
        limiter.sweep(2);
        assert_eq!(limiter.tracked(), 2);
        limiter.sweep(HOUR);
        assert_eq!(limiter.tracked(), 0);
    }

    #[test]
    fn an_unresolvable_address_omits_the_address_limit() {
        assert_eq!(limits_for(Some(caller(Some(1_500))), None, 60).len(), 2);
        let address = IpAddr::from([192, 0, 2, 1]);
        assert_eq!(
            limits_for(Some(caller(Some(1_500))), Some(address), 60).len(),
            3
        );
    }

    #[test]
    fn a_plan_without_an_address_ceiling_is_never_address_bucketed() {
        let address = IpAddr::from([192, 0, 2, 1]);
        let limits = limits_for(Some(caller(None)), Some(address), 60);
        assert_eq!(limits.len(), 2);
        assert_eq!(limits[0].bucket, Bucket::User(1));
        assert_eq!(limits[1].bucket, Bucket::UserBurst(1));
    }

    #[test]
    fn a_plans_address_ceiling_is_used_verbatim() {
        let address = IpAddr::from([192, 0, 2, 1]);
        let limits = limits_for(Some(caller(Some(1_500))), Some(address), 60);
        assert_eq!(limits[0].ceiling, 500);
        assert_eq!(limits[1].ceiling, 10);
        assert_eq!(limits[2].ceiling, 1_500);
        assert_eq!(limits[2].bucket, Bucket::KeyedIp(address));
    }

    #[test]
    fn an_anonymous_caller_gets_one_address_limit() {
        let address = IpAddr::from([192, 0, 2, 1]);
        let limits = limits_for(None, Some(address), 60);
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].ceiling, 60);
        assert_eq!(limits[0].bucket, Bucket::Ip(address));
    }

    fn anonymous_bucket(address: &str) -> Bucket {
        limits_for(None, address.parse().ok(), 60)[0].bucket
    }

    #[test]
    fn ipv6_callers_in_one_slash_64_share_a_bucket() {
        assert_eq!(
            anonymous_bucket("2001:db8:1:2::1"),
            anonymous_bucket("2001:db8:1:2:ffff:ffff:ffff:ffff")
        );
        assert_eq!(
            anonymous_bucket("2001:db8:1:2::1"),
            Bucket::Ip("2001:db8:1:2::".parse().expect("an address"))
        );
    }

    #[test]
    fn ipv6_callers_in_different_slash_64s_do_not_share_a_bucket() {
        assert_ne!(
            anonymous_bucket("2001:db8:1:2::1"),
            anonymous_bucket("2001:db8:1:3::1")
        );
    }

    #[test]
    fn an_ipv4_mapped_address_shares_the_ipv4_bucket() {
        assert_eq!(
            anonymous_bucket("::ffff:192.0.2.7"),
            anonymous_bucket("192.0.2.7")
        );
    }

    #[test]
    fn a_keyed_callers_address_limit_is_also_bucketed_by_slash_64() {
        let limits = limits_for(
            Some(caller(Some(1_500))),
            "2001:db8:1:2::1".parse().ok(),
            60,
        );
        assert_eq!(
            limits[2].bucket,
            Bucket::KeyedIp("2001:db8:1:2::".parse().expect("an address"))
        );
    }

    fn full_map(expires: i64) -> HashMap<Bucket, Window> {
        (0..RATE_LIMIT_MAX_TRACKED)
            .map(|i| {
                (
                    Bucket::User(i64::try_from(i).expect("fits i64")),
                    Window { expires, count: 1 },
                )
            })
            .collect()
    }

    #[test]
    fn a_sequence_of_checks_never_over_consumes_or_partially_consumes() {
        let mut rng = crate::test_support::Rng::new(9);
        for round in 0..200 {
            let limiter = Limiter::default();
            let ceiling_a = u32::try_from(rng.below(6))
                .expect("below six")
                .saturating_add(1);
            let ceiling_b = u32::try_from(rng.below(6))
                .expect("below six")
                .saturating_add(1);
            let limits = [
                Limit {
                    bucket: Bucket::User(round),
                    ceiling: ceiling_a,
                    window: WINDOW_SECONDS,
                },
                Limit {
                    bucket: Bucket::UserBurst(round),
                    ceiling: ceiling_b,
                    window: WINDOW_SECONDS,
                },
            ];
            let mut allowed = 0u32;
            for _ in 0..20 {
                let outcome = limiter.check_all(&limits, 10);
                if let Some(refused) = outcome.refused {
                    assert!(!outcome.allowed);
                    assert_eq!(refused.remaining, 0);
                } else {
                    assert!(outcome.allowed);
                    allowed += 1;
                }
                assert_eq!(
                    outcome.sustained.remaining,
                    ceiling_a - allowed,
                    "round {round}: a refusal consumed the sustained window"
                );
            }
            assert_eq!(allowed, ceiling_a.min(ceiling_b), "round {round}");
            assert_eq!(
                limiter.check_all(&limits[1..], 10).allowed,
                ceiling_b > allowed,
                "round {round}: a refusal consumed the burst window"
            );
        }
    }

    #[test]
    fn a_full_map_of_live_windows_is_flushed_at_the_ceiling_pinned_not_endorsed() {
        let mut windows = full_map(1_001);
        Limiter::bound(&mut windows, 1_000);
        assert!(windows.is_empty());
        let mut below = full_map(1_001);
        below.remove(&Bucket::User(0));
        Limiter::bound(&mut below, 1_000);
        assert_eq!(below.len(), RATE_LIMIT_MAX_TRACKED - 1);
    }

    #[test]
    fn check_all_bounds_the_map_before_tracking_a_new_bucket() {
        let limiter = Limiter {
            windows: Mutex::new(full_map(1_001)),
        };
        limiter.check_all(
            &[Limit {
                bucket: Bucket::User(-1),
                ceiling: 5,
                window: WINDOW_SECONDS,
            }],
            1_000,
        );
        assert_eq!(limiter.tracked(), 1);
    }

    #[test]
    fn expired_windows_are_dropped_before_the_map_is_flushed() {
        let mut windows = full_map(1_001);
        windows.insert(
            Bucket::User(-1),
            Window {
                expires: 9_000,
                count: 1,
            },
        );
        Limiter::bound(&mut windows, 5_000);
        assert_eq!(windows.len(), 1);
        assert!(windows.contains_key(&Bucket::User(-1)));
    }

    #[test]
    fn a_sweep_drops_only_expired_windows() {
        let limiter = Limiter::default();
        limiter.check_all(
            &[Limit {
                bucket: Bucket::User(1),
                ceiling: 5,
                window: 10,
            }],
            100,
        );
        limiter.check_all(
            &[Limit {
                bucket: Bucket::User(2),
                ceiling: 5,
                window: 1_000,
            }],
            100,
        );
        limiter.sweep(150);
        assert_eq!(limiter.tracked(), 1);
    }

    #[test]
    fn retry_after_is_at_least_one_second() {
        let decision = Decision {
            allowed: false,
            limit: 1,
            remaining: 0,
            reset: 100,
        };
        assert_eq!(retry_after(&decision, 100), 1);
        assert_eq!(retry_after(&decision, 150), 1);
        assert_eq!(retry_after(&decision, 40), 60);
    }
}
