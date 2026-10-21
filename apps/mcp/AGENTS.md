# MCP Agent Guide

This is the read-only MCP server for qafiyah.com, in Rust with axum and the official MCP SDK (`rmcp`). It serves nine tools over stateless Streamable HTTP at `https://mcp.qafiyah.com/mcp`. Each tool makes one call to the public API on the internal network. The app holds no data and no state.

## Shape

- `config.rs`: one `Config`, read from the environment (see "Settings").
- `forwarding.rs`: `caller_address` reads `CF-Connecting-IP` only when the request's peer is in `TRUSTED_PROXIES`. From any other peer, the app sends the API no address.
- `api.rs`: the one module that calls the API, and owns its wire format.
  - It sends `API_KEY_MCP` as `x-api-key`, the caller's address as `CF-Connecting-IP`, and no other header.
  - It builds each URL from percent-encoded path segments, and follows no redirects.
  - It turns each answer into a `Reply` (a 200 body, or the `Location` of a 301) or a `Failure` (429, 400, 404, or anything else as `Unavailable`). A URL too long to send (over 64 KB, such as a search text of 11,000 letters) is a `BadRequest`, like a 400.
- `slugs.rs`: the API's slug patterns. Poem and poet slugs are four ASCII letters. Term and feeling slugs are lowercase letters and hyphens, starting with a letter. A tool checks its path slug before the call, because a slug like `../../account/sessions` would change the route.
- `results.rs`: shapes the API's JSON into tool results.
  - Every poem and poet in a result gets a `url` next to its `slug` (`https://qafiyah.com/poems/{slug}`, `/poets/{slug}`).
  - `cap_poem` keeps whole verses, from the first, up to `MAX_POEM_CHARS` characters, and adds a note with the count and the link to the full poem. So one result stays under the assistants' output limits.
  - It also holds the text of each tool error.
- `tools.rs`: `Catalog`, the MCP server. Its nine tools are `search`, `list_poems`, `get_poem`, `random_poem`, `list_poets`, `get_poet`, `list_terms`, `list_feelings`, and `feeling_verses`. Each one is marked read-only, and its input schema follows the API's parameters.
- `metrics.rs`: the native histogram `mcp_tool_call_duration_seconds`, by `tool` and `outcome`. Both labels come from fixed sets (the nine tool names, and `ok`, `bad_input`, `refused`, `not_found`, `unavailable`).
- `log.rs`: `tracing` JSON lines. In production, only `warn` and `error`, and only `error` from rmcp. rmcp writes a `warn` for each request that it refuses (an `Origin`, a `Host`, an unknown tool), so any client could write one line for each request. The app writes a `warn` line only when the API fails; a refusal, a bad slug, or a 404 is a metric, never a log line.
- `lib.rs`: `app` mounts the MCP service at `/mcp` and `/healthz`, and answers 404 for every other path. `transport_config` sets the MCP transport.

## Settings

| Variable            | Compose                   | `bun run dev`                 | Production                 |
| ------------------- | ------------------------- | ----------------------------- | -------------------------- |
| `INTERNAL_API_URL`  | `http://api-backend:8787` | `http://localhost:8787`       | required                   |
| `API_KEY_MCP`       | from the secrets          | from the dev secrets          | required                   |
| `TRUSTED_PROXIES`   | the `backend` subnet      | unset (no peer is trusted)    | required                   |
| `MCP_ALLOWED_HOSTS` | `mcp.qafiyah.com`         | unset (rmcp's loopback hosts) | required                   |
| `MCP_PORT`          | `8788`                    | `DEV_MCP_PORT` (8788)         | defaults to `DEV_MCP_PORT` |
| `METRICS_PORT`      | `9464`                    | unset (no metrics listener)   | optional                   |

Under `ENVIRONMENT=production`, the app refuses to start without `API_KEY_MCP`, `TRUSTED_PROXIES`, or `MCP_ALLOWED_HOSTS`. Each one would otherwise fail without an error: every request refused for its `Host`, every caller in one shared bucket, or every call in the API's anonymous limits.

`TRUSTED_PROXIES` is `172.27.0.0/24` in `docker-compose.yml` and `172.26.0.0/24` in `docker-compose.dev.yml`, each environment's `backend` subnet.

## Limits

- The transport is stateless (`legacy_session_mode: false`) and answers JSON (`json_response: true`). `GET` and `DELETE` on `/mcp` answer 405.
- It refuses a `Host` outside `MCP_ALLOWED_HOSTS`, and any request that carries an `Origin` (403). It sends no CORS headers.
- A request body is at most 64 KiB, here and in the web nginx.
- The time limits nest: the API answers 503 after 6 seconds, each API call stops after 8 seconds, the whole request after 9 seconds, and the web nginx waits 10 seconds.
- The API counts MCP calls in their own buckets, by the forwarded address (`MCP_ADDRESS_REQUESTS`, 600 an hour), and against an hourly and a per-second total (`docs/exceptions.md`, "Internal keys bypass rate limiting and the accounts database"). The web nginx adds a request rate and a connection limit for each address.

## Tests

- Inline tests cover each module. `test_support::FakeApi` is a local axum server that records each request and answers a canned response, like `FakeEs` in the API.
- The tests in `lib.rs` drive the assembled app over HTTP: `initialize`, `tools/list`, a client on an older protocol version with no session, the `Host` and `Origin` checks, 405, 413, and the deadline.
- `no_input_schema_uses_a_type_array` keeps every input schema in single types, which clients with a single-type schema dialect need.
- The `mcp` smoke suite (`scripts/smoke/suites/mcp.ts`) runs on the origin, stack, and prod surfaces.
- To check with the official MCP Inspector, run `npx @modelcontextprotocol/inspector --cli http://localhost:8788/mcp --transport http --method tools/list --strict`.
