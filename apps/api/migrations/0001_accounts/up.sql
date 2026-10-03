create table if not exists plans (
  slug       text primary key,
  requests   integer not null check (requests >= 0),
  burst      integer not null check (burst >= 0),
  ip_ceiling integer          check (ip_ceiling >= 0),
  created_at timestamptz not null default now()
);

insert into plans (slug, requests, burst, ip_ceiling) values
  ('free',         500, 10, 1500),
  ('premium',     5000, 15, null),
  ('enterprise', 10000, 20, null)
on conflict (slug) do nothing;

create table if not exists users (
  id           bigint generated always as identity primary key,
  email        text not null,
  display_name text,
  avatar_url   text,
  plan         text not null default 'free' references plans(slug),
  created_at   timestamptz not null default now()
);
create unique index if not exists users_email_lower on users (lower(email));

create table if not exists identities (
  provider     text   not null,
  provider_uid text   not null,
  user_id      bigint not null references users(id) on delete cascade,
  created_at   timestamptz not null default now(),
  primary key (provider, provider_uid)
);

create table if not exists api_keys (
  id           bigint generated always as identity primary key,
  user_id      bigint not null references users(id) on delete cascade,
  key_hash     bytea  not null unique,
  prefix       text   not null,
  label        text,
  created_at   timestamptz not null default now(),
  last_used_at timestamptz,
  revoked_at   timestamptz
);
create index if not exists api_keys_active on api_keys (user_id) where revoked_at is null;

create table if not exists sessions (
  id         bytea primary key,
  user_id    bigint not null references users(id) on delete cascade,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null
);
create index if not exists sessions_expiry on sessions (expires_at);

create table if not exists usage_hourly (
  api_key_id bigint not null references api_keys(id) on delete cascade,
  hour       timestamptz not null,
  requests   integer not null,
  primary key (api_key_id, hour)
);
