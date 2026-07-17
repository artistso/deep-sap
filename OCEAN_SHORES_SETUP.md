# Public Coastal Demonstration Setup

This legacy filename is retained to avoid breaking links. The configuration does not describe a private location.

## Scenario coordinates

Use a coarse public demonstration region such as:

```text
latitude: 46.9
longitude: -124.1
```

These values are scenario inputs. They must not be described as a home address or returned in diagnostic headers.

## Local run

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve --address 0.0.0.0 --port 8080
```

Open `http://localhost:8080`. The application remains functional with deterministic environmental fixtures when public providers fail.

## Worker setup

Set non-secret Worker variables:

```text
ALLOWED_ORIGINS=https://deep-sap.soquarky.click,https://artistso.github.io
UPSTREAM_USER_AGENT=deep-sap-public-data-proxy/2.0 contact@example.com
DEMO_REGION_LAT=46.9
DEMO_REGION_LON=-124.1
```

Optionally configure a Cloudflare Rate Limiting binding named `RATE_LIMITER`. This release requires no third-party API keys and accepts no provider credentials from the browser.

## Field validation

Before displaying a public observation, verify provider, timestamp, unit, quality state, and plausibility bounds. GEBCO-derived values are contextual only and are not suitable for navigation or safety decisions.
