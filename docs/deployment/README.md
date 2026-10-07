# Deployment

> **The repo is the source.** Every deploy resets the host copy at `/opt/qafiyah` to `origin/main` (`git reset --hard origin/main`). So edit here, commit, release, and deploy. Nothing needs a manual sync.
>
> **Keep this doc set free of secrets.** Clones, CI logs, and future contributors can read it. So never write credentials, tokens, real IP addresses, or Cloudflare Tunnel IDs here. Say _where_ a secret is (the gitignored `.env` file), never its value. Read host-specific identifiers from the live server, for example with `cloudflared tunnel list`.

These docs are the production deploy guide and the operator runbook for the server. They cover the Docker Compose stack on one VPS behind Cloudflare: the site (`web`, `api`, `search-indexer`), its data stores, the edge gateway, and the observability stack. For local development, use `bun run dev`.

Pick the part that you need:

- **To deploy, release, or roll back:** follow the `deploy` skill (`.claude/skills/deploy/SKILL.md`). It is the ordered runbook for these tasks:
  - a VPS deploy, and a rollback
  - a new database or Elasticsearch dump, and a reindex
  - a major Postgres or Elasticsearch version upgrade
  - the WAF change from DetectionOnly to On

  The skill is manual only (`disable-model-invocation: true`), so ask for it by name or start it yourself.

- **To understand how the system is built:** read `docs/deployment/architecture.md`. It covers the traffic flow through the Cloudflare Tunnel and the edge gateway, the container stack, the isolation of production from dev, the security posture, and what a deploy does internally.
- **To set up or configure an environment:** read `docs/deployment/environments.md`. It covers the VPS prerequisites, the first-boot seeding, the secrets, and the environment variables that gate API keys.
- **To work on one service:** read `docs/deployment/services.md`. It covers the API, the web app (caching, nginx, TLS), the edge gateway and its WAF, and the search indexer.
- **To manage secrets, or to set up a machine that decrypts them:** read `docs/deployment/secrets.md`. It covers SOPS and age, the schema that every key must pass, and what each machine needs.
- **When something is broken, or for an operations task:** read `docs/deployment/troubleshooting.md`. It covers common host commands, known problems, and the recovery after a major version upgrade.

## See also

- `docs/topology.md`: a diagram-first map of the whole system, both code and production.
- `data/db/MAINTAINERS_GUIDE.md`: how to make and ship a database dump.
