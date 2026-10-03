set lock_timeout = '5s';
set statement_timeout = '15s';

update users set email = lower(email) where email <> lower(email);

-- squawk-ignore require-concurrent-index-creation -- users holds one row per API account, and a failed concurrent build would leave an invalid index behind
create unique index users_email on users (email);
