#!/usr/bin/env bash
# Myro installer — one-line setup:
#   curl -fsSL https://raw.githubusercontent.com/UzayPoyrza/myro/main/scripts/install.sh | bash
set -euo pipefail

GITHUB_REPO="UzayPoyrza/myro"
INSTALL_DIR="${MYRO_INSTALL_DIR:-$HOME/.local/bin}"

info()  { printf '\033[0;34m  %s\033[0m\n' "$1"; }
warn()  { printf '\033[0;33m  %s\033[0m\n' "$1"; }
error() { printf '\033[0;31m  error: %s\033[0m\n' "$1" >&2; exit 1; }

echo ""
echo "  ╔══════════════════════════════╗"
echo "  ║     myro — cp trainer        ║"
echo "  ╚══════════════════════════════╝"
echo ""

# ── Detect OS ──────────────────────────────────────────────────────────
OS="$(uname -s)"
case "$OS" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    MINGW*|MSYS*|CYGWIN*) error "Windows is not supported yet. Use WSL." ;;
    *)      error "unsupported OS: $OS" ;;
esac

# ── Detect arch (with Rosetta 2 detection on macOS) ───────────────────
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        if [ "$os" = "macos" ] && sysctl -n sysctl.proc_translated 2>/dev/null | grep -q 1; then
            arch="aarch64"
            info "detected Rosetta 2 — installing native arm64 binary"
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

info "platform: ${os} ${arch} (${target})"

# ── Fetch latest release from GitHub ──────────────────────────────────
info "fetching latest release..."
API_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
RELEASE_JSON="$(curl -fsSL "$API_URL" 2>/dev/null)" \
    || error "failed to fetch release info from GitHub. Check your internet connection."

# Parse tag_name (no jq dependency)
TAG="$(echo "$RELEASE_JSON" | grep -o '"tag_name"\s*:\s*"[^"]*"' | head -1 | cut -d'"' -f4)"
[ -z "$TAG" ] && error "could not determine latest version. Have you published a release?"

VERSION="${TAG#v}"
info "latest version: ${VERSION}"

# ── Find download URL for our target ──────────────────────────────────
TARBALL="myro-${TAG}-${target}.tar.gz"
DOWNLOAD_URL="$(echo "$RELEASE_JSON" | grep -o '"browser_download_url"\s*:\s*"[^"]*'"${TARBALL}"'"' | head -1 | cut -d'"' -f4)"
[ -z "$DOWNLOAD_URL" ] && error "no pre-built binary for ${target}. File an issue at github.com/${GITHUB_REPO}/issues"

CHECKSUM_URL="$(echo "$RELEASE_JSON" | grep -o '"browser_download_url"\s*:\s*"[^"]*checksums\.sha256"' | head -1 | cut -d'"' -f4)"

# ── Download ──────────────────────────────────────────────────────────
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

info "downloading ${TARBALL}..."
curl -fsSL -o "${TMPDIR}/${TARBALL}" "$DOWNLOAD_URL" \
    || error "download failed"

# ── Verify checksum ──────────────────────────────────────────────────
if [ -n "${CHECKSUM_URL:-}" ]; then
    info "verifying checksum..."
    curl -fsSL -o "${TMPDIR}/checksums.sha256" "$CHECKSUM_URL"

    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL="$(sha256sum "${TMPDIR}/${TARBALL}" | cut -d' ' -f1)"
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL="$(shasum -a 256 "${TMPDIR}/${TARBALL}" | cut -d' ' -f1)"
    else
        warn "no sha256sum found, skipping checksum verification"
        ACTUAL=""
    fi

    if [ -n "$ACTUAL" ]; then
        EXPECTED="$(grep "${TARBALL}" "${TMPDIR}/checksums.sha256" | cut -d' ' -f1)"
        if [ -n "$EXPECTED" ] && [ "$ACTUAL" != "$EXPECTED" ]; then
            error "checksum mismatch — download may be corrupted"
        fi
        info "checksum ok"
    fi
else
    warn "no checksums published for this release, skipping verification"
fi

# ── Extract & install ─────────────────────────────────────────────────
info "extracting..."
tar -xzf "${TMPDIR}/${TARBALL}" -C "${TMPDIR}"

mkdir -p "$INSTALL_DIR"
mv "${TMPDIR}/myro" "${INSTALL_DIR}/myro"
chmod +x "${INSTALL_DIR}/myro"

info "installed myro v${VERSION} to ${INSTALL_DIR}/myro"

# ── macOS: remove quarantine attribute ────────────────────────────────
if [ "$os" = "macos" ]; then
    xattr -d com.apple.quarantine "${INSTALL_DIR}/myro" 2>/dev/null || true
fi

# ── Add to PATH automatically ─────────────────────────────────────────
EXPORT_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*)
        ;;
    *)
        SHELL_NAME="$(basename "${SHELL:-/bin/bash}")"
        case "$SHELL_NAME" in
            zsh)  RC_FILE="$HOME/.zshrc" ;;
            bash)
                # Prefer .bash_profile on macOS, .bashrc on Linux
                if [ "$os" = "macos" ]; then
                    RC_FILE="$HOME/.bash_profile"
                else
                    RC_FILE="$HOME/.bashrc"
                fi
                ;;
            fish) RC_FILE="" ;;
            *)    RC_FILE="$HOME/.profile" ;;
        esac

        if [ "$SHELL_NAME" = "fish" ]; then
            fish -c "fish_add_path ${INSTALL_DIR}" 2>/dev/null || true
            info "added ${INSTALL_DIR} to fish PATH"
        elif [ -n "$RC_FILE" ]; then
            # Only add if not already present
            if ! grep -qF "$INSTALL_DIR" "$RC_FILE" 2>/dev/null; then
                echo "" >> "$RC_FILE"
                echo "# myro" >> "$RC_FILE"
                echo "$EXPORT_LINE" >> "$RC_FILE"
                info "added ${INSTALL_DIR} to PATH in ${RC_FILE}"
            fi
        fi

        # Make it available in the current session hint
        export PATH="${INSTALL_DIR}:$PATH"
        ;;
esac

echo ""
info "restart your terminal and run 'myro' to get started!"
echo ""
