# Security Policy

## Scope and classification

DEEP SAP is an unclassified synthetic simulation. Do not add real acoustic-signature libraries, sensitive vessel tracks, patrol areas, restricted bathymetry, classified information, or operational targeting logic.

## Credential boundary

Private provider credentials must never be placed in:

- Browser JavaScript
- `localStorage`, `sessionStorage`, IndexedDB, or URLs
- Git history
- Cloudflare Worker source or Wrangler configuration
- Response headers, logs, error bodies, or client-visible upstream URLs

The release Worker exposes no third-party keyed routes. Copernicus password grants, public token minting, query-string bearer tokens, and browser credential forms are intentionally absent.

## CORS and proxy policy

- Allow only configured origins.
- Validate all route parameters.
- Proxy only fixed allowlisted upstream providers.
- Do not return stack traces or personal coordinates.
- Do not copy arbitrary upstream headers.
- Do not expose complete upstream URLs when they may contain credentials.
- Apply the optional `RATE_LIMITER` binding to public routes in deployed environments.

## Data boundary

Public environmental data may affect environmental simulation fields only. It must not be represented as direct target detection or operational intelligence.

## Dependency checks

CI runs Rust formatting, Clippy, unit tests, native/WASM checks, `cargo audit`, JavaScript syntax checks, and a compressed WASM budget.

## Reporting

Report vulnerabilities through a private GitHub security advisory. Do not publish credentials or exploit details in a public issue.
