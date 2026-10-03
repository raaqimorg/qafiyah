#![expect(
    clippy::print_stdout,
    reason = "the issued key is this command's output"
)]
#![expect(
    clippy::print_stderr,
    reason = "usage and database errors go to stderr"
)]

use std::time::Duration;

use qafiyah_api::accounts::keys::{self, KeyError};
use qafiyah_api::accounts::users;
use qafiyah_api::constants::{MAX_ACTIVE_KEYS_PER_USER, PG_ACQUIRE_TIMEOUT_SECONDS};
use qafiyah_api::db::keys::PgKeys;
use qafiyah_api::db::users::PgUsers;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let Some(email) = args.next() else {
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

    let pool = match qafiyah_api::db::pool(
        &url,
        1,
        Duration::from_secs(PG_ACQUIRE_TIMEOUT_SECONDS),
        qafiyah_api::db::accounts_setup(),
    ) {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("could not connect: {e}");
            std::process::exit(1);
        }
    };

    let user = match users::for_email(&PgUsers::new(pool.clone()), &email).await {
        Ok(user) => user,
        Err(e) => {
            eprintln!("could not upsert the user: {e}");
            std::process::exit(1);
        }
    };

    let key = match keys::create_for(&PgKeys::new(pool), user.id, label.as_deref()).await {
        Ok(key) => key,
        Err(KeyError::TooMany) => {
            eprintln!(
                "{} already has {MAX_ACTIVE_KEYS_PER_USER} active keys; revoke one first",
                user.email
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("could not insert the key: {e}");
            std::process::exit(1);
        }
    };

    println!("user:   {} (id {})", user.email, user.id);
    println!("prefix: {}", key.prefix);
    println!("key:    {}", key.value);
    println!();
    println!("This is the only time the key is shown. Store it now.");
}
