#![expect(
    clippy::print_stdout,
    reason = "the issued key is this command's output"
)]
#![expect(
    clippy::print_stderr,
    reason = "usage and database errors go to stderr"
)]

use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use qafiyah_api::accounts::keys;
use qafiyah_api::accounts::users::normalize_email;
use qafiyah_api::db::accounts_schema::{api_keys, users};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let Some(email) = args.next().map(|raw| normalize_email(&raw)) else {
        eprintln!("usage: issue-key <email> [label]");
        std::process::exit(2);
    };
    let label = args.next();

    let url = match std::env::var("DATABASE_URL_ACCOUNTS") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("DATABASE_URL_ACCOUNTS is required");
            std::process::exit(1);
        }
    };

    let mut conn = match AsyncPgConnection::establish(&url).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("could not connect: {e}");
            std::process::exit(1);
        }
    };

    let user_id: i64 = match diesel::insert_into(users::table)
        .values(users::email.eq(&email))
        .on_conflict(users::email)
        .do_update()
        .set(users::email.eq(excluded(users::email)))
        .returning(users::id)
        .get_result(&mut conn)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("could not upsert the user: {e}");
            std::process::exit(1);
        }
    };

    let key = keys::generate();
    if let Err(e) = diesel::insert_into(api_keys::table)
        .values((
            api_keys::user_id.eq(user_id),
            api_keys::key_hash.eq(key.hash.as_slice()),
            api_keys::prefix.eq(&key.prefix),
            api_keys::label.eq(label.as_deref()),
        ))
        .execute(&mut conn)
        .await
    {
        eprintln!("could not insert the key: {e}");
        std::process::exit(1);
    }

    println!("user:   {email} (id {user_id})");
    println!("prefix: {}", key.prefix);
    println!("key:    {}", key.value);
    println!();
    println!("This is the only time the key is shown. Store it now.");
}
