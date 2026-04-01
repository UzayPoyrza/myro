#!/usr/bin/env bash
# Myro installer — curl -fsSL <url>/install.sh | bash
set -euo pipefail

REPO_API="${MYRO_REPO_API:-https://server.taild22ffc.ts.net/api/v1/repos/kalpturer/myro}"
INSTALL_DIR="${MYRO_INSTALL_DIR:-$HOME/.local/bin}"

info() { printf '\033[0;34m%s\033[0m\n' "$1"; }
error() { printf '\033[0;31merror: %s\033[0m\n' "$1" >&2; exit 1; }

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    *)      error "unsupported OS: $OS" ;;
esac

# Detect arch (with Rosetta 2 detection on macOS)
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        if [ "$os" = "macos" ] && sysctl -n sysctl.proc_translated 2>/dev/null | grep -q 1; then
            arch="aarch64"
        else
            arch="x86_64"
        fi
        ;;
    arm64|aarch64) arch="aarch64" ;;
    *)             error "unsupported architecture: $ARCH" ;;
esac

# Build target triple
case "$os" in
    linux) target="${arch}-unknown-linux-gnu" ;;
    macos) target="${arch}-apple-darwin" ;;
esac

info "detected: ${target}"

# Fetch latest release tag
info "fetching latest release..."
RELEASE_JSON="$(curl -fsSL "${REPO_API}/releases/latest")"

# Parse tag_name (no jq dependency)
TAG="$(echo "$RELEASE_JSON" | grep -o '"tag_name"\s*:\s*"[^"]*"' | head -1 | cut -d'"' -f4)"
[ -z "$TAG" ] && error "could not determine latest version"

VERSION="${TAG#v}"
info "latest version: ${VERSION}"

# Find download URL for our target
TARBALL="myro-${TAG}-${target}.tar.gz"
DOWNLOAD_URL="$(echo "$RELEASE_JSON" | grep -o '"browser_download_url"\s*:\s*"[^"]*'"${TARBALL}"'"' | head -1 | cut -d'"' -f4)"
[ -z "$DOWNLOAD_URL" ] && error "no release artifact for ${target}"

# Also try to find checksums
CHECKSUM_URL="$(echo "$RELEASE_JSON" | grep -o '"browser_download_url"\s*:\s*"[^"]*checksums\.sha256"' | head -1 | cut -d'"' -f4)"

# Download tarball
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

info "downloading ${TARBALL}..."
curl -fsSL -o "${TMPDIR}/${TARBALL}" "$DOWNLOAD_URL"

# Verify checksum if available
if [ -n "$CHECKSUM_URL" ]; then
    info "verifying checksum..."
    curl -fsSL -o "${TMPDIR}/checksums.sha256" "$CHECKSUM_URL"

    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL="$(sha256sum "${TMPDIR}/${TARBALL}" | cut -d' ' -f1)"
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL="$(shasum -a 256 "${TMPDIR}/${TARBALL}" | cut -d' ' -f1)"
    else
        info "warning: no sha256sum found, skipping checksum verification"
        ACTUAL=""
    fi

    if [ -n "$ACTUAL" ]; then
        EXPECTED="$(grep "${TARBALL}" "${TMPDIR}/checksums.sha256" | cut -d' ' -f1)"
        if [ -n "$EXPECTED" ] && [ "$ACTUAL" != "$EXPECTED" ]; then
            error "checksum mismatch: expected ${EXPECTED}, got ${ACTUAL}"
        fi
    fi
fi

# Extract
info "extracting..."
tar -xzf "${TMPDIR}/${TARBALL}" -C "${TMPDIR}"

# Install
mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/myro" "${INSTALL_DIR}/myro"
chmod +x "${INSTALL_DIR}/myro"

info "installed myro v${VERSION} to ${INSTALL_DIR}/myro"

# PATH hint
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        echo ""
        info "add ${INSTALL_DIR} to your PATH:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
        ;;
esac

info "run 'myro' to get started!"
