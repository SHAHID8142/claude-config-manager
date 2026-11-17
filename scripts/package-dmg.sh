#!/bin/bash
# Package the signed Tauri .app without Finder/AppleScript automation.
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
version="$(node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).version' "$repo_dir/package.json")"
app_path="$repo_dir/src-tauri/target/release/bundle/macos/Claude Config Manager.app"
output_dir="$repo_dir/src-tauri/target/release/bundle/dmg"
output_path="$output_dir/Claude Config Manager_${version}_aarch64.dmg"
[[ -d "$app_path" ]] || { echo "Build the macOS app bundle first." >&2; exit 1; }
codesign --verify --deep --strict "$app_path"
stage_dir="$(mktemp -d "${TMPDIR:-/tmp/}ccm-dmg.XXXXXX")"
trap 'rm -rf "$stage_dir"' EXIT
mkdir -p "$stage_dir/Claude Config Manager" "$output_dir"
ditto "$app_path" "$stage_dir/Claude Config Manager/Claude Config Manager.app"
ln -s /Applications "$stage_dir/Claude Config Manager/Applications"
# Create beside the final file so an interrupted package cannot replace a
# previously verified release image. No GUI or Finder access is required.
staged_image="$stage_dir/release.dmg"
if diskutil image create from --help >/dev/null 2>&1; then
  diskutil image create from --format UDZO --volumeName "Claude Config Manager" \
    "$stage_dir/Claude Config Manager" "$staged_image"
else
  # Compatibility for macOS releases before `diskutil image` was available.
  hdiutil create -format UDZO -volname "Claude Config Manager" \
    -srcfolder "$stage_dir/Claude Config Manager" "$staged_image"
fi
hdiutil verify "$staged_image"
mv -f "$staged_image" "$output_path"
(cd "$output_dir" && shasum -a 256 "$(basename "$output_path")" > SHA256SUMS)
echo "Packaged: $output_path"
