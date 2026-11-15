#!/usr/bin/env bash
# Usage: scripts/bump-version.sh 1.1.0
# Sets the version everywhere it must match. Then: update CHANGELOG.md, commit, and tag
#   git tag -a v1.1.0 -m "Dial Control v1.1.0" && git push origin main --tags
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
v="${1:?usage: $0 <version, e.g. 1.1.0>}"
[[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.]+)?$ ]] || { echo "not a semver version: $v" >&2; exit 1; }

sed -i -E "0,/^version = \".*\"/s//version = \"$v\"/" src-tauri/Cargo.toml
node -e '
const fs=require("fs"),v=process.argv[1];
for (const f of ["package.json","src-tauri/tauri.conf.json"]) {
  const j=JSON.parse(fs.readFileSync(f,"utf8")); j.version=v;
  fs.writeFileSync(f, JSON.stringify(j,null,2)+"\n");
}' "$v"
[ -f dist-packages/PKGBUILD ] && sed -i -E "s/^pkgver=.*/pkgver=$v/" dist-packages/PKGBUILD
# keep the lockfile's own crate entry in sync
(cd src-tauri && cargo update -p dial-control --offline -q 2>/dev/null || true)
echo "Version set to $v. Next: edit CHANGELOG.md, commit, then tag v$v."
