#!/usr/bin/env bash
# T63 — dependency vulnerability gate.
#
# Checks every RESOLVED dependency version (npm from node_modules, cargo from
# Cargo.lock) against the OSV advisory database, then keeps only findings that
# actually reach the macOS build (npm always counts; cargo crates must appear in
# the aarch64-apple-darwin dependency graph).
#
# Exit 1 if anything in the shipped graph is affected. Linux-only and
# build-time-only crates are reported separately so they are visible but do not
# pretend to be macOS risk.
#
# Usage: scripts/dep-audit.sh [--verbose]
set -euo pipefail

cd "$(dirname "$0")/.."
VERBOSE="${1:-}"

echo "== npm audit (version-aware, registry-backed) =="
npm audit --audit-level=high

echo
echo "== resolving installed versions =="
python3 - <<'PY' > /tmp/ccm_osv_queries.json
import json, glob
queries = []
for pj in glob.glob("node_modules/**/package.json", recursive=True):
    try:
        d = json.load(open(pj))
    except Exception:
        continue
    if d.get("name") and d.get("version"):
        queries.append({"package": {"name": d["name"], "ecosystem": "npm"}, "version": d["version"]})
import re
lock = open("src-tauri/Cargo.lock").read()
for m in re.finditer(r'\[\[package\]\]\nname = "([^"]+)"\nversion = "([^"]+)"', lock):
    name, version = m.groups()
    queries.append({"package": {"name": name, "ecosystem": "crates.io"}, "version": version})
seen, unique = set(), []
for q in queries:
    key = (q["package"]["name"], q["version"])
    if key not in seen:
        seen.add(key)
        unique.append(q)
json.dump({"queries": unique}, open("/tmp/ccm_osv_queries.json", "w"))
print(len(unique), file=__import__("sys").stderr)
PY

echo "== querying OSV =="
curl -sf --max-time 90 -X POST \
  -H "Content-Type: application/json" \
  -d @/tmp/ccm_osv_queries.json \
  https://api.osv.dev/v1/querybatch \
  -o /tmp/ccm_osv_results.json

echo "== macOS dependency graph =="
(cd src-tauri && cargo tree -e normal --target aarch64-apple-darwin) \
  | sed -E 's/.* ([A-Za-z0-9_-]+) v.*/\1/' | sort -u > /tmp/ccm_macos_crates.txt

python3 - "$VERBOSE" <<'PY'
import json, sys
verbose = len(sys.argv) > 1 and sys.argv[1] == "--verbose"
queries = json.load(open("/tmp/ccm_osv_queries.json"))["queries"]
results = json.load(open("/tmp/ccm_osv_results.json")).get("results", [])
macos = set(open("/tmp/ccm_macos_crates.txt").read().split())

shipped, excluded = [], []
for i, res in enumerate(results):
    pkg = queries[i]["package"]
    for vuln in res.get("vulns", []):
        entry = (pkg["name"], queries[i].get("version", "?"), vuln["id"],
                 (vuln.get("summary") or "")[:80])
        if pkg["ecosystem"] == "npm" or pkg["name"] in macos:
            shipped.append(entry)
        else:
            excluded.append(entry)

print(f"checked {len(queries)} resolved packages")
print(f"affecting the macOS/shipped graph: {len(shipped)}")
for e in shipped:
    print("  SHIPPED  ", *e)
print(f"not in the macOS graph (Linux-only / build-time): {len(excluded)}")
if verbose:
    for e in excluded:
        print("  excluded ", *e)
else:
    for e in excluded:
        print(f"  excluded  {e[0]} {e[1]} ({e[2]})")

if shipped:
    print("\nFAIL: advisories affect dependencies that ship in the macOS build")
    sys.exit(1)
print("\nPASS: no advisory affects a dependency of the macOS build")
PY
