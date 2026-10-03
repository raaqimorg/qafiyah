use axum::http::StatusCode;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde_json::json;

use qafiyah_api::accounts::usage::UsageRepository;
use qafiyah_api::accounts::{keys, sessions, users};
use qafiyah_api::db::accounts_schema::usage_hourly;
use qafiyah_api::db::usage::PgUsage;

use crate::{FULL, INTERNAL, harness, unique_email};

fn identity(email: &str) -> users::Identity {
    users::Identity {
        provider: "google".into(),
        provider_uid: format!("uid-{email}"),
        email: email.into(),
        display_name: Some("Test User".into()),
        avatar_url: None,
    }
}

#[tokio::test]
async fn a_user_is_upserted_by_email_and_its_identity_relinked() {
    let Some(h) = harness().await else { return };
    h.isolated("upsert", |h, email| async move {
        let created = h
            .call(
                "POST",
                "/account/users",
                Some(INTERNAL),
                Some(json!({
                    "provider": "google",
                    "provider_uid": format!("uid-{email}"),
                    "email": email.to_uppercase(),
                    "display_name": "First",
                    "avatar_url": null
                })),
            )
            .await;
        assert_eq!(created.status, StatusCode::OK, "{}", created.body);
        let profile = created.json();
        assert_eq!(profile["email"], email);
        let id = profile["id"].as_i64().expect("id");
        let again = h
            .call(
                "POST",
                "/account/users",
                Some(INTERNAL),
                Some(json!({
                    "provider": "github",
                    "provider_uid": format!("gh-{email}"),
                    "email": email,
                    "display_name": null,
                    "avatar_url": "https://example.test/a.png"
                })),
            )
            .await
            .json();
        assert_eq!(again["id"], id, "the same email is the same user");
        assert_eq!(
            again["display_name"], "First",
            "a null display name does not erase the stored one"
        );
        assert_eq!(again["avatar_url"], "https://example.test/a.png");
    })
    .await;
}

#[tokio::test]
async fn sessions_are_created_resolved_and_deleted_through_the_internal_api() {
    let Some(h) = harness().await else { return };
    h.isolated("session", |h, email| async move {
        let profile = users::upsert(h.state.users.as_ref(), &identity(&email))
            .await
            .expect("a user");
        let created = h
            .call(
                "POST",
                "/account/sessions",
                Some(INTERNAL),
                Some(json!({ "user_id": profile.id })),
            )
            .await;
        assert_eq!(created.status, StatusCode::OK, "{}", created.body);
        let id = created.json()["id"].as_str().expect("id").to_string();
        let resolved = h
            .call(
                "GET",
                &format!("/account/sessions/{id}"),
                Some(INTERNAL),
                None,
            )
            .await;
        assert_eq!(resolved.status, StatusCode::OK);
        assert_eq!(resolved.json()["id"], profile.id);
        assert_eq!(resolved.header("cache-control"), Some("no-store"));
        assert_eq!(
            h.call("GET", &format!("/account/sessions/{id}"), Some(FULL), None)
                .await
                .status,
            StatusCode::UNAUTHORIZED,
            "the full key never opens the account api"
        );
        assert_eq!(
            h.call(
                "DELETE",
                &format!("/account/sessions/{id}"),
                Some(INTERNAL),
                None
            )
            .await
            .status,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            h.call(
                "GET",
                &format!("/account/sessions/{id}"),
                Some(INTERNAL),
                None
            )
            .await
            .status,
            StatusCode::UNAUTHORIZED
        );
        let second = sessions::create(h.state.sessions.as_ref(), profile.id)
            .await
            .expect("a session");
        assert!(
            sessions::resolve(h.state.sessions.as_ref(), &second)
                .await
                .expect("query")
                .is_some()
        );
        assert_eq!(
            h.call(
                "DELETE",
                "/account/sessions",
                Some(INTERNAL),
                Some(json!({ "user_id": profile.id }))
            )
            .await
            .status,
            StatusCode::NO_CONTENT
        );
        assert!(
            sessions::resolve(h.state.sessions.as_ref(), &second)
                .await
                .expect("query")
                .is_none()
        );
    })
    .await;
}

#[tokio::test]
async fn an_email_finds_or_creates_one_user_whatever_its_case() {
    let Some(h) = harness().await else { return };
    h.isolated("for-email", |h, email| async move {
        let created = users::for_email(
            h.state.users.as_ref(),
            &format!(" {} ", email.to_uppercase()),
        )
        .await
        .expect("a user");
        let found = users::for_email(h.state.users.as_ref(), &email)
            .await
            .expect("the same user");
        assert_eq!(created.id, found.id);
        assert_eq!(created.email, email);
    })
    .await;
}

#[tokio::test]
async fn keys_are_capped_per_user_listed_with_their_plan_and_revocable_once() {
    let Some(h) = harness().await else { return };
    h.isolated("keys", |h, email| async move {
        let profile = users::upsert(h.state.users.as_ref(), &identity(&email))
            .await
            .expect("a user");
        let first = h
            .call(
                "POST",
                "/account/keys",
                Some(INTERNAL),
                Some(json!({ "user_id": profile.id, "label": "one" })),
            )
            .await;
        assert_eq!(first.status, StatusCode::OK, "{}", first.body);
        let raw = first.json()["value"].as_str().expect("value").to_string();
        assert!(raw.starts_with("qaf_") && raw.len() == 36);
        assert_eq!(
            first.json()["prefix"],
            raw.get(..12).expect("a 36-char key")
        );
        assert_eq!(
            h.call(
                "POST",
                "/account/keys",
                Some(INTERNAL),
                Some(json!({ "user_id": profile.id, "label": null }))
            )
            .await
            .status,
            StatusCode::OK
        );
        let third = h
            .call(
                "POST",
                "/account/keys",
                Some(INTERNAL),
                Some(json!({ "user_id": profile.id, "label": "three" })),
            )
            .await;
        assert_eq!(third.status, StatusCode::CONFLICT);
        assert_eq!(third.json()["code"], "TOO_MANY_KEYS");
        let view = h
            .call(
                "GET",
                &format!("/account/users/{}/keys", profile.id),
                Some(INTERNAL),
                None,
            )
            .await
            .json();
        assert_eq!(view["plan"], "free");
        assert_eq!(view["requests"], 500);
        assert_eq!(view["burst"], 10);
        assert_eq!(view["keys"].as_array().expect("keys").len(), 2);
        let caller = keys::lookup(h.state.api_keys.as_ref(), &raw)
            .await
            .expect("lookup")
            .expect("a live key");
        assert_eq!(caller.user_id, profile.id);
        assert_eq!(
            (caller.requests, caller.burst, caller.ip_ceiling),
            (500, 10, Some(1500))
        );
        let key_id = view["keys"][0]["id"].as_i64().expect("id");
        assert_eq!(
            h.call(
                "POST",
                "/account/keys/revoke",
                Some(INTERNAL),
                Some(json!({ "user_id": profile.id, "key_id": key_id }))
            )
            .await
            .status,
            StatusCode::NO_CONTENT
        );
        let again = h
            .call(
                "POST",
                "/account/keys/revoke",
                Some(INTERNAL),
                Some(json!({ "user_id": profile.id, "key_id": key_id })),
            )
            .await;
        assert_eq!(again.status, StatusCode::NOT_FOUND);
        assert_eq!(again.json()["detail"], "API key not found");
        assert_eq!(
            h.call("GET", "/account/users/-1/keys", Some(INTERNAL), None)
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    })
    .await;
}

#[tokio::test]
async fn a_real_key_is_limited_by_its_plan_and_its_usage_is_flushed() {
    let Some(h) = harness().await else { return };
    h.isolated("usage", |h, email| async move {
        let profile = users::upsert(h.state.users.as_ref(), &identity(&email))
            .await
            .expect("a user");
        let raw = keys::create_for(h.state.api_keys.as_ref(), profile.id, Some("limits"))
            .await
            .expect("a key")
            .value;
        let sent = h.call("GET", "/v1/meters", Some(&raw), None).await;
        assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
        assert_eq!(sent.header("x-ratelimit-limit"), Some("500"));
        assert_eq!(sent.header("x-ratelimit-remaining"), Some("499"));
        let recorder = qafiyah_api::accounts::usage::UsageRecorder::default();
        let key_id = h.state.api_keys.active_for(profile.id).await.expect("keys")[0].id;
        recorder.record(key_id, 500_000);
        assert_eq!(
            recorder
                .flush(&PgUsage::new(h.accounts.clone()))
                .await
                .expect("flush"),
            1
        );
        recorder.record(key_id, 500_000);
        recorder.record(key_id, 500_000);
        assert_eq!(
            recorder
                .flush(&PgUsage::new(h.accounts.clone()))
                .await
                .expect("flush"),
            1
        );
        let mut conn = h.accounts.get().await.expect("an accounts connection");
        let used: i32 = usage_hourly::table
            .filter(usage_hourly::api_key_id.eq(key_id))
            .select(usage_hourly::requests)
            .first(&mut conn)
            .await
            .expect("a row");
        assert_eq!(used, 3, "a later flush adds to the same hour");
    })
    .await;
}

#[tokio::test]
async fn usage_that_cannot_be_stored_as_counted_is_refused_and_writes_nothing() {
    let Some(h) = harness().await else { return };
    h.isolated("usage-range", |h, email| async move {
        let profile = users::upsert(h.state.users.as_ref(), &identity(&email))
            .await
            .expect("a user");
        keys::create_for(h.state.api_keys.as_ref(), profile.id, None)
            .await
            .expect("a key");
        let key_id = h.state.api_keys.active_for(profile.id).await.expect("keys")[0].id;
        let usage = PgUsage::new(h.accounts.clone());
        assert!(
            usage.add(&[(key_id, i64::MAX, 1)]).await.is_err(),
            "an hour past the calendar"
        );
        assert!(
            usage.add(&[(key_id, 500_000, u32::MAX)]).await.is_err(),
            "a count past the column's range"
        );
        let mut conn = h.accounts.get().await.expect("an accounts connection");
        let rows: i64 = usage_hourly::table
            .filter(usage_hourly::api_key_id.eq(key_id))
            .count()
            .get_result(&mut conn)
            .await
            .expect("a count");
        assert_eq!(rows, 0);
    })
    .await;
}

#[tokio::test]
async fn the_same_provider_identity_with_a_new_email_keeps_its_account_and_keys() {
    let Some(h) = harness().await else { return };
    let first = unique_email("email-change");
    let second = unique_email("email-change-next");
    let third = unique_email("email-change-taken");
    let uid = format!("uid-{first}");

    let mut identity = users::Identity {
        provider: "google".into(),
        provider_uid: uid,
        email: first.clone(),
        display_name: Some("Test User".into()),
        avatar_url: None,
    };

    let original = users::upsert(h.state.users.as_ref(), &identity)
        .await
        .expect("a user");
    let created = keys::create_for(h.state.api_keys.as_ref(), original.id, Some("kept"))
        .await
        .expect("a key");

    identity.email = second.clone();
    let relinked = users::upsert(h.state.users.as_ref(), &identity)
        .await
        .expect("relinked");
    assert_eq!(relinked.id, original.id, "the identity keeps its account");
    assert_eq!(relinked.email, second, "the email follows the identity");

    let listed = h
        .state
        .api_keys
        .active_for(original.id)
        .await
        .expect("keys");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].prefix, created.prefix, "the key stays attached");

    users::upsert(
        h.state.users.as_ref(),
        &users::Identity {
            provider: "github".into(),
            provider_uid: "gh-other".into(),
            email: third.clone(),
            display_name: None,
            avatar_url: None,
        },
    )
    .await
    .expect("a second account");

    identity.email = third.clone();
    assert!(
        matches!(
            users::upsert(h.state.users.as_ref(), &identity).await,
            Err(users::UpsertError::EmailTaken)
        ),
        "an email owned by another account is refused"
    );

    for email in [&first, &second, &third] {
        h.delete_user(email).await;
    }
}

#[tokio::test]
async fn creating_a_key_for_an_unknown_user_is_a_not_found() {
    let Some(h) = harness().await else { return };
    let sent = h
        .call(
            "POST",
            "/account/keys",
            Some(INTERNAL),
            Some(json!({ "user_id": -1, "label": null })),
        )
        .await;
    assert_eq!(sent.status, StatusCode::NOT_FOUND, "{}", sent.body);
    assert_eq!(sent.json()["detail"], "Account not found");
}

#[tokio::test]
async fn the_issue_key_command_normalizes_the_email_issues_working_keys_and_stops_at_the_cap() {
    let Some(h) = harness().await else { return };
    let url = std::env::var("QAFIYAH_TEST_DATABASE_URL_ACCOUNTS").expect("the accounts URL");
    h.isolated("issue-key", |h, email| async move {
        let run = |label: &str| {
            std::process::Command::new(env!("CARGO_BIN_EXE_issue-key"))
                .arg(format!("  {}  ", email.to_uppercase()))
                .arg(label)
                .env("DATABASE_URL_ACCOUNTS", &url)
                .output()
                .expect("the command runs")
        };
        let first = run("one");
        assert!(first.status.success(), "{first:?}");
        let printed = String::from_utf8(first.stdout).expect("UTF-8 output");
        assert!(
            printed.contains(&format!("user:   {email} (id ")),
            "{printed}"
        );
        let raw = printed
            .lines()
            .find_map(|line| line.strip_prefix("key:    "))
            .expect("the printed key")
            .to_string();
        assert!(keys::is_well_formed(&raw));
        let caller = keys::lookup(h.state.api_keys.as_ref(), &raw)
            .await
            .expect("a lookup");
        assert!(caller.is_some(), "the issued key authenticates");

        assert!(run("two").status.success());
        let third = run("three");
        assert_eq!(third.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&third.stderr).contains("revoke one first"),
            "{third:?}"
        );
    })
    .await;
}
