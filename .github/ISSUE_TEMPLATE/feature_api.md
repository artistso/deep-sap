---
name: New API / Real Ocean Source
about: Add new real data source (CMEMS, USGS, NASA, etc.)
title: "[API] "
labels: enhancement, api
---

**Source:** e.g., Copernicus Marine CMEMS OSTIA SST, NASA MODIS, USGS tide?
**URL / Docs:**
**CORS?** yes/no, needs proxy?
**License / DOI:**
**How it helps sonar:**
SST? Salinity? Bottom depth? Currents for sonobuoy drift? Ambient noise?

**Proposed RealOceanSample mapping:**
```rust
sst_c: Some(...)
```

**Mock fallback plan:**
