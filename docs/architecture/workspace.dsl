workspace "Qafiyah" "The architecture of qafiyah.com, an Arabic poetry catalog, in the C4 model." {

    !identifiers hierarchical
    !impliedRelationships false

    model {
        reader = person "Reader" "Reads, searches, and filters poems on qafiyah.com."
        developer = person "API developer" "Creates API keys on qafiyah.com and calls api.qafiyah.com."
        maintainer = person "Maintainer" "Deploys, reseeds, reindexes, and watches the dashboards."

        cloudflare = softwareSystem "Cloudflare" "DNS, TLS, page cache, rate limits, and the tunnel into the VPS." "External"
        sentry = softwareSystem "Sentry" "Collects errors and browser sessions." "External"
        posthog = softwareSystem "PostHog" "Product analytics, reached through its managed proxy at ix.qafiyah.com." "External"
        github = softwareSystem "GitHub" "Hosts the source, runs CI, and signs developers in." "External"
        google = softwareSystem "Google" "Signs developers in." "External"

        qafiyah = softwareSystem "Qafiyah" "The Arabic poetry catalog at qafiyah.com and its public API at api.qafiyah.com." {
            edge = container "Edge gateway" "Inspects every request and passes it to the website." "nginx, ModSecurity, OWASP CRS"
            web = container "Website" "Renders and caches pages, signs developers in, routes api.qafiyah.com to the API, and proxies the browser's API calls." "Astro SSR, nginx, React" {
                nginx = component "nginx" "Routes by Host, caches pages and searches, and serves static files." "nginx"
                pages = component "Pages" "Server-renders every page, and rewrites to /404 when the API has nothing." "Astro"
                catalog = component "Catalog client" "Calls the API with the internal key, retries network failures, and sets Cache-Control." "TypeScript, openapi-fetch"
                proxy = component "API proxy" "Forwards the browser's /api/v1 calls that are on the allowlist, with the internal key." "Astro endpoint"
                identity = component "Identity" "Google and GitHub sign-in, the session cookie, and account calls." "TypeScript"
                islands = component "Islands" "Search, the poet page's filters, the random poem, and settings, running in the reader's browser." "React"
                timing = component "Request metrics" "Times every request to the end of its body and reports server errors." "Astro middleware, OpenTelemetry, Sentry"
            }
            api = container "API" "Serves the catalog, search, and accounts as JSON under /v1." "Rust, axum" {
                layers = component "HTTP layers" "Request metrics, CORS, request ids, the request log, rate limits, ETags, the 6 s deadline, and error responses." "tower, tower-http"
                routes = component "Routes" "One handler file per resource: parse the request, call the domain, return the contract type." "axum"
                parsing = component "Request parsing" "Typed query and path values; a 400 names the parameter." "serde_html_form"
                domain = component "Domain" "Catalog types and rules, and the repository traits." "Rust"
                contract = component "Contract" "The public JSON types and the OpenAPI document." "serde, utoipa"
                accounts = component "Accounts" "Key hashing and the key cache, sessions, and usage recording." "Rust"
                postgres = component "Postgres adapters" "The repositories over the corpus and accounts databases." "Diesel, deadpool"
                elasticsearch = component "Elasticsearch adapter" "Query bodies with relevance tiers, and typed hits." "reqwest"
                metrics = component "Metrics endpoint" "Serves the request and search histograms on port 9464." "prometheus-client"
            }
            corpus = container "Corpus database" "Poems, poets, and every taxonomy: the source of truth." "PostgreSQL" "Database"
            accountsDb = container "Accounts database" "Users, sessions, API keys, and usage." "PostgreSQL" "Database"
            search = container "Search index" "Poems and poets for full-text search, rebuilt from the corpus." "Elasticsearch" "Database"
            indexer = container "Search indexer" "Rebuilds the search index from the corpus database." "Rust, one-shot job"
            telemetryProxy = container "Telemetry proxy" "Forwards browser errors and sessions to Sentry from t.qafiyah.com." "Cloudflare Worker"
            avatars = container "Avatar store" "Poet avatar images at cdn.qafiyah.com." "Cloudflare R2" "Database"

            prometheus = container "Prometheus" "Scrapes and stores metrics, and receives the website's OTLP push." "Prometheus" "Observability"
            grafana = container "Grafana" "Seven dashboards over Prometheus and Loki, for the maintainer only." "Grafana" "Observability"
            loki = container "Loki" "Container logs, kept 7 days." "Loki" "Observability,Database"
            alloy = container "Alloy" "Ships container logs to Loki, and exposes container and host stats." "Grafana Alloy" "Observability"
            pgExporter = container "Postgres exporter" "Query statistics and health of the corpus database." "postgres_exporter" "Observability"
            esExporter = container "Elasticsearch exporter" "Cluster health, heap, and query totals." "elasticsearch_exporter" "Observability"
            blackbox = container "Blackbox exporter" "Probes /healthz on the API and the website." "blackbox_exporter" "Observability"
            monitorRole = container "Monitor role job" "Creates the read-only role the Postgres exporter signs in as." "PostgreSQL client, one-shot job" "Observability"
        }

        reader -> cloudflare "Reads poems on qafiyah.com through" "HTTPS"
        reader -> qafiyah "Loads poet avatars from, and sends browser errors to" "HTTPS, cdn. and t.qafiyah.com"
        reader -> posthog "Sends page views to" "HTTPS, ix.qafiyah.com"
        developer -> cloudflare "Calls api.qafiyah.com and manages keys on qafiyah.com through" "HTTPS"
        maintainer -> github "Pushes code to" "git over HTTPS"
        maintainer -> qafiyah "Deploys, reseeds, reindexes, and opens Grafana over" "SSH through the Cloudflare Tunnel"
        cloudflare -> qafiyah "Forwards qafiyah.com and api.qafiyah.com to" "HTTP over the Cloudflare Tunnel"
        qafiyah -> sentry "Reports errors and browser sessions to" "HTTPS"
        qafiyah -> google "Signs developers in with" "OAuth 2.0"
        qafiyah -> github "Signs developers in with" "OAuth 2.0"
        qafiyah -> github "Fetches main from, at each deploy" "git over HTTPS"

        cloudflare -> qafiyah.edge "Forwards qafiyah.com and api.qafiyah.com to" "HTTP over the Cloudflare Tunnel"
        qafiyah.edge -> qafiyah.web "Passes inspected requests to" "HTTP, edge network"
        qafiyah.web -> qafiyah.api "Fetches page data and makes account calls, and routes api.qafiyah.com to" "HTTP/JSON, backend network"
        qafiyah.api -> qafiyah.corpus "Reads poems, poets, and taxonomies from" "SQL, default network"
        qafiyah.api -> qafiyah.accountsDb "Reads and writes users, keys, sessions, and usage in" "SQL, default network"
        qafiyah.api -> qafiyah.search "Searches" "HTTP/JSON, default network"
        qafiyah.indexer -> qafiyah.corpus "Streams poems and poets from" "SQL, default network"
        qafiyah.indexer -> qafiyah.search "Writes a new versioned index and swaps the alias in" "HTTP/JSON, default network"
        reader -> qafiyah.avatars "Loads poet avatars from" "HTTPS, cdn.qafiyah.com"
        reader -> qafiyah.telemetryProxy "Sends browser errors and sessions to" "HTTPS, t.qafiyah.com"
        qafiyah.telemetryProxy -> sentry "Forwards browser events to" "HTTPS"
        qafiyah.web -> sentry "Reports server errors to" "HTTPS"
        qafiyah.api -> sentry "Reports errors to" "HTTPS"
        qafiyah.web -> google "Signs developers in with" "OAuth 2.0"
        qafiyah.web -> github "Signs developers in with" "OAuth 2.0"
        maintainer -> qafiyah.indexer "Runs forced reindexes with (bun run reindex:prod)" "SSH, docker compose run"
        maintainer -> qafiyah.grafana "Opens the dashboards in (bun run observe)" "HTTP over an SSH port forward"

        qafiyah.web -> qafiyah.prometheus "Pushes request durations to" "OTLP/HTTP, default network"
        qafiyah.prometheus -> qafiyah.api "Scrapes request and search histograms from" "HTTP, protobuf, default network"
        qafiyah.prometheus -> qafiyah.pgExporter "Scrapes" "HTTP, default network"
        qafiyah.prometheus -> qafiyah.esExporter "Scrapes" "HTTP, default network"
        qafiyah.prometheus -> qafiyah.blackbox "Runs /healthz probes through" "HTTP, default network"
        qafiyah.prometheus -> qafiyah.alloy "Scrapes container, host, and log pipeline metrics from" "HTTP, observability network"
        qafiyah.prometheus -> qafiyah.loki "Scrapes Loki's own metrics from" "HTTP, observability network"
        qafiyah.pgExporter -> qafiyah.corpus "Reads query statistics and health from" "SQL, default network"
        qafiyah.esExporter -> qafiyah.search "Reads cluster health and stats from" "HTTP/JSON, default network"
        qafiyah.blackbox -> qafiyah.api "Probes /healthz on" "HTTP, default network"
        qafiyah.blackbox -> qafiyah.web "Probes /healthz on" "HTTP, default network"
        qafiyah.alloy -> qafiyah.loki "Pushes container logs to" "HTTP, observability network"
        qafiyah.grafana -> qafiyah.prometheus "Queries" "PromQL over HTTP, observability network"
        qafiyah.grafana -> qafiyah.loki "Queries" "LogQL over HTTP, observability network"
        qafiyah.monitorRole -> qafiyah.corpus "Creates the qafiyah_monitor role in" "SQL, default network"

        qafiyah.edge -> qafiyah.web.nginx "Passes inspected requests to" "HTTP, edge network"
        qafiyah.web.nginx -> qafiyah.web.pages "Forwards page requests it has not cached to" "HTTP"
        qafiyah.web.nginx -> qafiyah.web.proxy "Forwards /api/v1 calls it has not cached to" "HTTP"
        qafiyah.web.nginx -> qafiyah.api "Routes api.qafiyah.com to" "HTTP, backend network"
        qafiyah.web.pages -> qafiyah.web.catalog "Fetches page data through" "TypeScript"
        qafiyah.web.pages -> qafiyah.web.identity "Signs developers in and loads accounts through" "TypeScript"
        qafiyah.web.pages -> qafiyah.web.islands "Ships to the browser" "HTML, JavaScript"
        qafiyah.web.islands -> qafiyah.web.proxy "Calls /api/v1 on, through Cloudflare, the edge gateway, and nginx" "HTTPS"
        qafiyah.web.timing -> qafiyah.web.pages "Wraps every request to" "Astro middleware"
        qafiyah.web.catalog -> qafiyah.api "Calls /v1 with the internal key on" "HTTP/JSON, backend network"
        qafiyah.web.proxy -> qafiyah.api "Forwards allowlisted calls with the internal key to" "HTTP/JSON, backend network"
        qafiyah.web.identity -> qafiyah.api "Makes account calls to" "HTTP/JSON, backend network"
        qafiyah.web.identity -> google "Exchanges sign-in codes with" "OAuth 2.0"
        qafiyah.web.identity -> github "Exchanges sign-in codes with" "OAuth 2.0"
        qafiyah.web.timing -> qafiyah.prometheus "Pushes request durations to" "OTLP/HTTP, default network"
        qafiyah.web.timing -> sentry "Reports server errors to" "HTTPS"

        qafiyah.web -> qafiyah.api.layers "Sends every request through" "HTTP/JSON, backend network"
        qafiyah.prometheus -> qafiyah.api.metrics "Scrapes" "HTTP, protobuf, default network"
        qafiyah.api.layers -> qafiyah.api.routes "Passes requests to" "Rust"
        qafiyah.api.layers -> qafiyah.api.accounts "Looks up API keys in" "Rust"
        qafiyah.api.layers -> qafiyah.api.metrics "Records each request in" "Rust"
        qafiyah.api.layers -> sentry "Reports server errors to" "HTTPS"
        qafiyah.api.routes -> qafiyah.api.parsing "Parses queries and paths with" "Rust"
        qafiyah.api.routes -> qafiyah.api.domain "Calls" "Rust"
        qafiyah.api.routes -> qafiyah.api.contract "Converts results to" "Rust"
        qafiyah.api.routes -> qafiyah.api.accounts "Manages users, keys, and sessions through" "Rust"
        qafiyah.api.domain -> qafiyah.api.postgres "Reads through the repository traits, implemented by" "Rust"
        qafiyah.api.domain -> qafiyah.api.elasticsearch "Searches through the SearchIndex trait, implemented by" "Rust"
        qafiyah.api.accounts -> qafiyah.api.postgres "Stores users, sessions, keys, and usage through" "Rust"
        qafiyah.api.elasticsearch -> qafiyah.api.metrics "Records each search in" "Rust"
        qafiyah.api.postgres -> qafiyah.corpus "Reads from" "SQL, default network"
        qafiyah.api.postgres -> qafiyah.accountsDb "Reads and writes" "SQL, default network"
        qafiyah.api.elasticsearch -> qafiyah.search "Searches" "HTTP/JSON, default network"

        production = deploymentEnvironment "Production" {
            cf = deploymentNode "Cloudflare" "The global network in front of every qafiyah.com host." "Cloudflare" {
                cloudflareEdge = infrastructureNode "Edge" "TLS, the page cache, rate limit rules, and the tunnel's public end." "Cloudflare"
                deploymentNode "Workers" "" "Cloudflare Workers" {
                    containerInstance qafiyah.telemetryProxy
                }
                deploymentNode "R2" "" "Cloudflare R2" {
                    containerInstance qafiyah.avatars
                }
            }
            vps = deploymentNode "VPS" "One small Linux server; nothing listens on a public port." "Linux" {
                cloudflared = infrastructureNode "cloudflared" "Dials out to Cloudflare and forwards each host to 127.0.0.1." "systemd service"
                sshd = infrastructureNode "sshd" "SSH on 127.0.0.1:22, reached only through the tunnel." "systemd service"
                compose = deploymentNode "Docker Compose project qafiyah" "Fourteen containers on four networks: edge, backend, default, and observability." "Docker Compose" {
                    edgeInstance = containerInstance qafiyah.edge
                    containerInstance qafiyah.web
                    containerInstance qafiyah.api
                    deploymentNode "qafiyah-db" "One Postgres server, two databases." "PostgreSQL 18" {
                        containerInstance qafiyah.corpus
                        containerInstance qafiyah.accountsDb
                    }
                    containerInstance qafiyah.search
                    containerInstance qafiyah.indexer
                    containerInstance qafiyah.prometheus
                    containerInstance qafiyah.grafana
                    containerInstance qafiyah.loki
                    containerInstance qafiyah.alloy
                    containerInstance qafiyah.pgExporter
                    containerInstance qafiyah.esExporter
                    containerInstance qafiyah.blackbox
                    containerInstance qafiyah.monitorRole
                }
            }
            production.cf.cloudflareEdge -> production.vps.cloudflared "Carries requests through the tunnel, which the VPS dials out to open" "Cloudflare Tunnel"
            production.vps.cloudflared -> production.vps.compose.edgeInstance "Forwards qafiyah.com and api.qafiyah.com to" "HTTP, 127.0.0.1:80"
            production.vps.cloudflared -> production.vps.sshd "Forwards ssh.qafiyah.com to" "SSH, 127.0.0.1:22"
        }
    }

    views {
        systemContext qafiyah "context" "Who uses Qafiyah and what it depends on." {
            title "Qafiyah: system context"
            include *
            include developer
            autoLayout lr
        }

        container qafiyah "containers" "The product's containers, without the observability stack." {
            title "Qafiyah: containers"
            include *
            include developer
            exclude "element.tag==Observability"
            autoLayout lr
        }

        container qafiyah "observability" "The private observability stack and what it watches." {
            title "Qafiyah: the observability stack"
            include "element.tag==Observability"
            include maintainer qafiyah.api qafiyah.web qafiyah.corpus qafiyah.search
            exclude "qafiyah.web -> qafiyah.api"
            exclude "qafiyah.api -> qafiyah.search"
            exclude "qafiyah.api -> qafiyah.corpus"
            autoLayout lr
        }

        deployment qafiyah production "deployment" "Where each container runs in production; the observability stack runs in the same Compose project." {
            title "Qafiyah: production deployment"
            include *
            exclude "element.tag==Observability"
            autoLayout lr
        }

        dynamic qafiyah "search" "A website search." {
            title "Qafiyah: a website search"
            reader -> cloudflare "Types a query, and the search island requests /api/v1/search"
            cloudflare -> qafiyah.edge "Forwards it uncached: the cache rule skips /api/"
            qafiyah.edge -> qafiyah.web "Passes it after the WAF checks"
            qafiyah.web -> qafiyah.api "On an nginx cache miss, the proxy forwards it with the internal key"
            qafiyah.api -> qafiyah.search "Runs the ranked poem query"
            autoLayout lr
        }

        dynamic qafiyah "poet-filters" "A poet page, then a change of filter." {
            title "Qafiyah: a poet page and its filters"
            reader -> cloudflare "Opens /poets/{slug}"
            cloudflare -> qafiyah.edge "Forwards it, unless its page cache has it"
            qafiyah.edge -> qafiyah.web "Passes it after the WAF checks"
            qafiyah.web -> qafiyah.api "Renders the page from the poet, its first poems, and their facets"
            qafiyah.api -> qafiyah.corpus "Reads them"
            reader -> cloudflare "Picks a meter, and the poet island requests /api/v1/poems for this poet"
            cloudflare -> qafiyah.edge "Forwards it uncached"
            qafiyah.edge -> qafiyah.web "Passes it after the WAF checks"
            qafiyah.web -> qafiyah.api "The proxy forwards it, because it names exactly one poet"
            qafiyah.api -> qafiyah.corpus "Reads the filtered poems and facets"
            autoLayout lr
        }

        dynamic qafiyah "reindex" "A forced reindex." {
            title "Qafiyah: a forced reindex (bun run reindex:prod)"
            maintainer -> qafiyah.indexer "Starts it with docker compose run, over SSH"
            qafiyah.indexer -> qafiyah.corpus "Streams every poem and poet"
            qafiyah.indexer -> qafiyah.search "Writes poems_vN and poets_vN, force-merges them, and swaps the aliases"
            qafiyah.api -> qafiyah.search "Searches the new index through the alias"
            autoLayout lr
        }

        component qafiyah.api "api-components" "The API's components, from the Shape section of apps/api/AGENTS.md." {
            title "Qafiyah: API components"
            include *
            exclude "qafiyah.web -> sentry"
            exclude "qafiyah.web -> qafiyah.prometheus"
            autoLayout tb
        }

        component qafiyah.web "web-components" "The website's components, from the Shape section of apps/web/AGENTS.md." {
            title "Qafiyah: website components"
            include *
            exclude "qafiyah.prometheus -> qafiyah.api"
            exclude "qafiyah.api -> sentry"
            autoLayout tb
        }

        properties {
            "c4plantuml.tags" "true"
            "plantuml.includes" "legend.puml"
        }

        styles {
            element "Person" {
                shape person
                background #08427b
                color #ffffff
            }
            element "Software System" {
                background #1168bd
                color #ffffff
            }
            element "Container" {
                background #438dd5
                color #ffffff
            }
            element "Component" {
                background #85bbf0
                color #000000
            }
            element "External" {
                background #999999
                color #ffffff
            }
            element "Database" {
                shape cylinder
            }
        }
    }
}
