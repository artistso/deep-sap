# Environmental API Layer

The release API layer is intentionally narrow. `src/api/mod.rs` compiles only the NOAA and GEBCO adapters used by the vertical slice. Credential-bearing and defense-named experimental adapters were removed from the package.

## Runtime flow

1. `fetch_orchestrator_system` starts one asynchronous fetch task at a time.
2. `poll_fetch_tasks` polls the Bevy task and applies completed samples.
3. `ApiState` merges values by field and preserves provider provenance.
4. On failure, the deterministic environment remains active.

## Proxy routes

The Cloudflare Worker exposes fixed credential-free public-provider routes rather than an arbitrary URL proxy. It validates provider-specific parameters, applies exact-origin CORS, supports an optional Cloudflare rate-limit binding, and never mints or relays access tokens.

## Adding a provider

A provider is eligible for the release build only after it has:

- documented licensing and attribution
- a typed parser with malformed-response tests
- units and timestamp handling
- timeout, retry, and stale-data behavior
- a fixed outbound host allowlist
- no browser-side private credential requirement
- proof that its values cannot create or classify game targets
