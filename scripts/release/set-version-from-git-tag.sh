#!/usr/bin/env bash
set -euo pipefail

# set-version-from-git-tag.sh — Stamp [workspace.package].version (and the
# matching path-dependency version pins) in the root Cargo.toml from GIT_TAG,
# for local/dev builds that want the binary's `--version` to reflect exactly
# which commit it was built from.
#
# Usage:
#   GIT_TAG=v0.7.5-12-gabc123 scripts/release/set-version-from-git-tag.sh
#   scripts/release/set-version-from-git-tag.sh          # computes GIT_TAG itself
#
# Unlike scripts/release/bump-version.sh (used for tagged releases), this
# script only touches Cargo.toml + Cargo.lock — it deliberately does not
# touch README badges, docs examples, or marketplace templates, since those
# track released versions, not every dev build.

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CARGO_TOML="$REPO_ROOT/Cargo.toml"

RAW_TAG="${GIT_TAG:-$(git -C "$REPO_ROOT" describe --tags --always --dirty=-dirty --abbrev=7)}"

# git tags are vX.Y.Z; Cargo versions must not have the leading v.
CANDIDATE="${RAW_TAG#v}"
CANDIDATE="${CANDIDATE#V}"

SEMVER_RE='^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$'
if [[ "$CANDIDATE" =~ $SEMVER_RE ]]; then
  VERSION="$CANDIDATE"
else
  # No reachable tag (git describe --always fell back to a bare commit hash)
  # or an unexpected tag shape. Keep the current released version as the
  # base and fold the raw describe output in as a dev prerelease identifier,
  # so the build stays traceable without producing an invalid Cargo version.
  CURRENT="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$CARGO_TOML" | head -1)"
  BASE="${CURRENT%%-*}"
  SUFFIX="$(echo "$CANDIDATE" | tr -c '0-9A-Za-z' '-')"
  VERSION="${BASE}-dev.${SUFFIX}"
  echo "warn: '$RAW_TAG' isn't a vX.Y.Z tag; using derived dev version $VERSION" >&2
fi

echo "Stamping workspace version: $VERSION (from git: $RAW_TAG)"

# [workspace.package] version — first bare `version = "..."` line in the file
sed -i -E '0,/^version = "[^"]+"/s||version = "'"$VERSION"'"|' "$CARGO_TOML"

# [workspace.dependencies] path-dep version pins, skipping aardvark* (which
# tracks an independent version — see bump-version.sh for the same rule)
sed -i -E '/path = "crates\/aardvark/!s|(path = "crates/[^"]+", version = ")[^"]+(")|\1'"$VERSION"'\2|' "$CARGO_TOML"

# Keep Cargo.lock's workspace-member entries in sync so a `--locked` build
# (e.g. install.sh) doesn't choke on a stale lockfile.
if command -v cargo >/dev/null 2>&1; then
  ( cd "$REPO_ROOT" && cargo update --workspace --offline >/dev/null 2>&1 ) \
    || ( cd "$REPO_ROOT" && cargo update --workspace >/dev/null 2>&1 ) \
    || echo "warn: cargo update --workspace failed; review Cargo.lock manually" >&2
fi

echo "Done."
