// DEEP SAP public environmental-data proxy.
// This release exposes only credential-free, fixed-host public providers.
// It never accepts provider secrets, mints OAuth tokens, or returns upstream URLs.

const PUBLIC_PROVIDERS = new Set([
  "ndbc",
  "gebco",
  "coops",
  "open-meteo-marine",
  "open-meteo-weather",
  "weather-gov",
  "usgs",
]);

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const traceId = crypto.randomUUID();
    const origin = request.headers.get("Origin");
    const corsHeaders = buildCorsHeaders(origin, env);

    if (request.method === "OPTIONS") {
      if (!corsHeaders) {
        return jsonResponse({ error: "origin_not_allowed", trace_id: traceId }, 403, null);
      }
      return new Response(null, { status: 204, headers: corsHeaders });
    }

    if (!new Set(["GET", "HEAD"]).has(request.method)) {
      return jsonResponse({ error: "method_not_allowed", trace_id: traceId }, 405, corsHeaders, {
        Allow: "GET, HEAD, OPTIONS",
        "Cache-Control": "no-store",
      });
    }

    if (!corsHeaders && origin) {
      return jsonResponse({ error: "origin_not_allowed", trace_id: traceId }, 403, null);
    }

    if (env.RATE_LIMITER) {
      const client = request.headers.get("CF-Connecting-IP") || "anonymous";
      const { success } = await env.RATE_LIMITER.limit({ key: `${client}:${url.pathname}` });
      if (!success) {
        return jsonResponse({ error: "rate_limited", trace_id: traceId }, 429, corsHeaders, {
          "Cache-Control": "no-store",
          "Retry-After": "60",
        });
      }
    }

    try {
      const route = resolveRoute(url, env);
      if (route.localResponse) {
        return jsonResponse(route.localResponse.body, route.localResponse.status, corsHeaders, {
          "X-Trace-Id": traceId,
          "Cache-Control": route.localResponse.cacheControl || "no-store",
        });
      }

      if (!PUBLIC_PROVIDERS.has(route.provider)) {
        throw new Error("provider outside release allowlist");
      }

      const upstreamResponse = await fetch(route.upstream, route.init);
      const headers = new Headers();
      copySafeResponseHeaders(upstreamResponse.headers, headers);
      applyHeaders(headers, corsHeaders);
      headers.set("X-Trace-Id", traceId);
      headers.set("X-Data-Provider", route.provider);
      headers.set("X-Fetch-Time", new Date().toISOString());
      headers.set(
        "Cache-Control",
        route.cacheControl || "public, max-age=300, stale-while-revalidate=900",
      );
      headers.set("X-Content-Type-Options", "nosniff");
      headers.set("Referrer-Policy", "no-referrer");

      return new Response(upstreamResponse.body, {
        status: upstreamResponse.status,
        statusText: upstreamResponse.statusText,
        headers,
      });
    } catch (error) {
      console.error("DEEP_SAP_PROXY_ERROR", {
        trace_id: traceId,
        message: error instanceof Error ? error.message : String(error),
      });
      return jsonResponse(
        { error: "upstream_request_failed", trace_id: traceId },
        502,
        corsHeaders,
        { "Cache-Control": "no-store" },
      );
    }
  },
};

function resolveRoute(url, env) {
  const path = url.pathname;
  const userAgent = env.UPSTREAM_USER_AGENT || "deep-sap-public-data-proxy/2.0";
  const init = { method: "GET", headers: { "User-Agent": userAgent } };

  if (path === "/" || path === "") {
    return {
      localResponse: {
        status: 200,
        cacheControl: "public, max-age=300",
        body: {
          service: "DEEP SAP public environmental-data proxy",
          security_model: "credential-free providers on fixed allowlisted hosts",
          public_routes: [
            "/ndbc/:station",
            "/gebco?lat=&lon=",
            "/coops?...",
            "/open-meteo/marine?lat=&lon=",
            "/open-meteo/weather?lat=&lon=",
            "/weather-gov/points?lat=&lon=",
            "/usgs/quake?lat=&lon=",
          ],
          disabled: [
            "provider-secret relay",
            "OAuth token minting",
            "query-string bearer tokens",
            "browser-stored credentials",
          ],
        },
      },
    };
  }

  if (path.startsWith("/sentinel/") || path === "/sentinel") {
    return {
      localResponse: {
        status: 410,
        body: {
          error: "credentialed_browser_flow_removed",
          guidance: "Use a separately authenticated private backend for any future imagery integration.",
        },
      },
    };
  }

  if (path.startsWith("/ndbc/")) {
    const station = (path.split("/")[2] || "46211").toUpperCase();
    if (!/^[A-Z0-9]{3,8}$/.test(station)) throw new Error("invalid station");
    return {
      provider: "ndbc",
      upstream: `https://www.ndbc.noaa.gov/data/realtime2/${station}.txt`,
      init,
    };
  }

  if (path === "/gebco") {
    const { lat, lon } = validatedCoordinates(url, env);
    return {
      provider: "gebco",
      upstream: `https://api.opentopodata.org/v1/gebco2020?locations=${lat},${lon}`,
      init,
      cacheControl: "public, max-age=86400, stale-while-revalidate=604800",
    };
  }

  if (path === "/coops") {
    const params = new URLSearchParams(url.searchParams);
    params.set("application", "deep-sap-public-simulation");
    if (!params.get("station") || !params.get("product")) {
      throw new Error("CO-OPS station and product are required");
    }
    return {
      provider: "coops",
      upstream: `https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?${params}`,
      init,
    };
  }

  if (path === "/open-meteo/marine") {
    const { lat, lon } = validatedCoordinates(url, env);
    return {
      provider: "open-meteo-marine",
      upstream: `https://marine-api.open-meteo.com/v1/marine?latitude=${lat}&longitude=${lon}&hourly=wave_height,swell_wave_height,sea_surface_temperature,ocean_current_velocity,wind_wave_height&timezone=auto`,
      init,
    };
  }

  if (path === "/open-meteo/weather") {
    const { lat, lon } = validatedCoordinates(url, env);
    return {
      provider: "open-meteo-weather",
      upstream: `https://api.open-meteo.com/v1/forecast?latitude=${lat}&longitude=${lon}&current=temperature_2m,wind_speed_10m&hourly=temperature_2m,wind_speed_10m&timezone=auto`,
      init,
    };
  }

  if (path === "/weather-gov/points") {
    const { lat, lon } = validatedCoordinates(url, env);
    init.headers.Accept = "application/geo+json";
    return {
      provider: "weather-gov",
      upstream: `https://api.weather.gov/points/${lat},${lon}`,
      init,
    };
  }

  if (path === "/usgs/quake") {
    const { lat, lon } = validatedCoordinates(url, env);
    return {
      provider: "usgs",
      upstream: `https://earthquake.usgs.gov/fdsnws/event/1/query?format=geojson&latitude=${lat}&longitude=${lon}&maxradiuskm=200&minmagnitude=2.5&orderby=time`,
      init,
    };
  }

  return { localResponse: { status: 404, body: { error: "unknown_route" } } };
}

function validatedCoordinates(url, env) {
  const fallbackLat = parseEnvironmentCoordinate(env.DEMO_REGION_LAT, 46.9);
  const fallbackLon = parseEnvironmentCoordinate(env.DEMO_REGION_LON, -124.1);
  const lat = parseFinite(url.searchParams.get("lat"), fallbackLat);
  const lon = parseFinite(url.searchParams.get("lon"), fallbackLon);
  if (lat < -90 || lat > 90 || lon < -180 || lon > 180) {
    throw new Error("coordinates out of range");
  }
  return { lat: lat.toFixed(4), lon: lon.toFixed(4) };
}

function parseEnvironmentCoordinate(value, fallback) {
  if (value === undefined || value === null || value === "") return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function parseFinite(value, fallback) {
  if (value === null || value === "") return fallback;
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) throw new Error("invalid number");
  return parsed;
}

function buildCorsHeaders(origin, env) {
  const allowed = new Set(
    (env.ALLOWED_ORIGINS || "https://deep-sap.soquarky.click,https://artistso.github.io")
      .split(",")
      .map(value => value.trim())
      .filter(Boolean),
  );
  if (!origin) return new Headers();
  if (!allowed.has(origin)) return null;

  const headers = new Headers();
  headers.set("Access-Control-Allow-Origin", origin);
  headers.set("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS");
  headers.set("Access-Control-Allow-Headers", "Content-Type");
  headers.set("Access-Control-Max-Age", "86400");
  headers.set("Vary", "Origin");
  return headers;
}

function copySafeResponseHeaders(source, destination) {
  for (const name of ["Content-Type", "ETag", "Last-Modified"]) {
    const value = source.get(name);
    if (value) destination.set(name, value);
  }
}

function applyHeaders(target, source) {
  if (!source) return;
  for (const [name, value] of source.entries()) target.set(name, value);
}

function jsonResponse(body, status, corsHeaders, extraHeaders = {}) {
  const headers = new Headers({
    "Content-Type": "application/json; charset=utf-8",
    "X-Content-Type-Options": "nosniff",
    "Referrer-Policy": "no-referrer",
    ...extraHeaders,
  });
  applyHeaders(headers, corsHeaders);
  return new Response(JSON.stringify(body), { status, headers });
}
