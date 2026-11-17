// Endpoint security classification (T19) — TS mirror of Rust
// `provider_catalog::classify_endpoint` (apply-time authority). Keep both in
// sync; divergence is caught by contract audits (T19/T67).

export type EndpointCheck =
  | { check: "invalid"; detail: string }
  | { check: "insecure" }
  | { check: "local" }
  | { check: "remote_ok" };

const LOCAL_HOSTS = new Set(["localhost", "127.0.0.1", "::1"]);

export function classifyEndpoint(url: string): EndpointCheck {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return { check: "invalid", detail: "not a valid URL" };
  }
  if (parsed.protocol !== "https:") {
    return { check: "insecure" };
  }
  const host = parsed.hostname.toLowerCase();
  const isLocal = LOCAL_HOSTS.has(host) || host === "[::1]" || host.endsWith(".local");
  return isLocal ? { check: "local" } : { check: "remote_ok" };
}

export function endpointCheckMessage(check: EndpointCheck): string | null {
  switch (check.check) {
    case "invalid":
      return "This is not a valid URL.";
    case "insecure":
      return "Refused: credentials would be sent over an insecure connection. Use https.";
    case "local":
      return "Local endpoint — API traffic stays on this machine.";
    case "remote_ok":
      return null;
  }
}

/** Stable local profile ID; never contains a URL or credential. */
export function customProviderId(endpoint: string): string {
  let value = endpoint.trim().replace(/\/+$/, "");
  try {
    value = new URL(value).href.replace(/\/+$/, "");
  } catch {
    /* validation is separate */
  }
  let hash = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(value)) {
    hash ^= BigInt(byte);
    hash = BigInt.asUintN(64, hash * 0x100000001b3n);
  }
  return `custom-${hash.toString(16)}`;
}
