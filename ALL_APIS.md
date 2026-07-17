# Public Data Provider Matrix

DEEP SAP uses external data only to vary **environmental context**. It does not use public feeds to detect, identify, classify, or locate game targets.

| Provider | Intended field | Browser route | Credential policy | Release status |
|---|---|---|---|---|
| NOAA CO-OPS | Water temperature | Fixed validated public route/direct endpoint | No credential | Compiled |
| GEBCO via OpenTopoData | Bathymetric context | Fixed validated public route/direct endpoint | No credential | Compiled |
| Open-Meteo Marine | Wave/environment dashboard | Fixed validated public route | No credential | Dashboard only |
| NOAA NDBC | Public buoy observations | Allowlisted station route | No credential | Proxy only |
| Weather.gov | Forecast metadata | Fixed points route | No credential | Proxy only |
| USGS | Public earthquake context | Fixed query route | No credential | Proxy only |

Credential-bearing satellite, private imagery, and defense-named experimental modules were removed from the release package.

## Hard boundary

```text
Public environmental observation -> environmental field with provenance
Synthetic truth -> sensor model -> noisy observation -> track estimate
```

There is no path from an external provider response to a synthetic target entity or track classification.

## Data contract

Every accepted environmental value records:

- value and field-defined units
- provider
- observation timestamp
- local fetch time
- quality state: unknown, synthetic, observed, or stale

A provider can update only fields it actually supplied. Missing values never overwrite another provider's valid measurement.

## Security rules

- No API credentials in browser storage, source files, query strings, or response headers.
- No complete upstream URLs returned to callers.
- Only credential-free providers on fixed allowlisted hosts are exposed.
- Exact CORS origins are configured explicitly.
- Parameters and coordinates are validated.
- Optional Cloudflare rate limiting can be bound as `RATE_LIMITER`.
- Error responses omit stack traces and upstream details.
