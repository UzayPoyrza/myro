#!/usr/bin/env bash
# Build release artifacts for myro.
# Usage: ./scripts/release.sh v0.1.0
set -euo pipefail

VERSION="${1:?usage: release.sh <version>}"
DIST="dist"

TARGETS=(
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

info() { printf '\033[0;34m%s\033[0m\n' "$1"; }
error() { printf '\033[0;31merror: %s\033[0m\n' "$1" >&2; exit 1; }

rm -rf "$DIST"
mkdir -p "$DIST"

for target in "${TARGETS[@]}"; do
    info "building for ${target}..."

    # Use cross if available, otherwise cargo (native only)
    if command -v cross >/dev/null 2>&1; then
        cross build --release --target "$target" -p myro-tui
    else
        cargo build --release --target "$target" -p myro-tui
    fi

    BINARY="target/${target}/release/myro"
    [ -f "$BINARY" ] || error "binary not found: ${BINARY}"

    TARBALL="myro-${VERSION}-${target}.tar.gz"
    info "packaging ${TARBALL}..."
    tar -czf "${DIST}/${TARBALL}" -C "target/${target}/release" myro
done

# Generate checksums
info "generating checksums..."
cd "$DIST"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum *.tar.gz > checksums.sha256
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 *.tar.gz > checksums.sha256
else
    error "no sha256sum or shasum found"
fi
cd ..

info "release artifacts in ${DIST}/:"
ls -lh "${DIST}/"

info "done! upload these to your release."
