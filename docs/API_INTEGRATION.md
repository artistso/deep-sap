# API Integration Contract

External services supply environmental context only.

## Required adapter output

```text
value
provider
observed_at
fetched_at
quality
```

Adapters must reject malformed payloads, non-finite numbers, impossible ranges, missing required units, and unsupported coordinate bounds. They must not silently substitute a provider's missing field with an unrelated value.

## Security

The browser may call only credential-free public routes. The release proxy contains no third-party API keys or OAuth flows and uses fixed hosts, fixed paths, parameter validation, explicit CORS origins, safe error bodies, optional rate limiting, and route-specific cache policy.

## Simulation separation

External results can update `OceanProfile` fields. They cannot spawn `TruthTarget`, `BearingObservation`, or `SapTrack` entities and cannot influence target identity labels.
