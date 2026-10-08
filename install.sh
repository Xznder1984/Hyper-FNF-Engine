#!/bin/sh
# Hyper Engine 'hfe' installer for Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/Xznder1984/Hyper-FNF-Engine/main/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --dir "$HOME/bin" --version 1.0.4
#
# Installs only the 'hfe' command-line tool. The optional Tauri GUI launcher is
# published separately on the releases page.
#
# Environment overrides: HFE_VERSION, HFE_DIR, HFE_REPO, GITHUB_TOKEN (optional).

set -eu

REPO="${HFE_REPO:-Xznder1984/Hyper-FNF-Engine}"
VERSION="${HFE_VERSION:-latest}"
DEST="${HFE_DIR:-}"
ASSUME_YES=0

say()  { printf '%s\n' "$*"; }
info() { printf '  %s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<EOF
Hyper Engine 'hfe' installer (Linux/macOS)

Usage: install.sh [options]

Options:
  -d, --dir DIR        install directory      (default: \$HOME/.local/bin)
  -V, --version VER    release tag or 'latest' (default: latest)
  -r, --repo OWNER/R   GitHub repository       (default: $REPO)
  -y, --yes            do not prompt
  -h, --help           show this help

Environment: HFE_VERSION, HFE_DIR, HFE_REPO, GITHUB_TOKEN (optional)
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        -d|--dir)     [ $# -ge 2 ] || fail "missing value for $1"; DEST="$2"; shift 2 ;;
        -V|--version) [ $# -ge 2 ] || fail "missing value for $1"; VERSION="$2"; shift 2 ;;
        -r|--repo)    [ $# -ge 2 ] || fail "missing value for $1"; REPO="$2"; shift 2 ;;
        -y|--yes)     ASSUME_YES=1; shift ;;
        -h|--help)    usage; exit 0 ;;
        *)            fail "unknown option: $1 (try --help)" ;;
    esac
done

command -v curl >/dev/null 2>&1 || fail "curl is required"

# --- platform detection ----------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$os" in
    Linux)  os_part="unknown-linux-gnu" ;;
    Darwin) os_part="apple-darwin" ;;
    *)      fail "unsupported OS: $os (on Windows use install.ps1)" ;;
esac
case "$arch" in
    x86_64|amd64)  arch_part="x86_64" ;;
    aarch64|arm64) arch_part="aarch64" ;;
    *)             fail "unsupported architecture: $arch" ;;
esac
target="$arch_part-$os_part"

# --- resolve version -------------------------------------------------------
if [ "$VERSION" = "latest" ]; then
    say "resolving latest Hyper Engine release from $REPO ..."
    api="https://api.github.com/repos/$REPO/releases/latest"
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        json="$(curl -fsSL -H "Authorization: Bearer $GITHUB_TOKEN" \
            -H 'Accept: application/vnd.github+json' "$api" 2>/dev/null || true)"
    else
        json="$(curl -fsSL -H 'Accept: application/vnd.github+json' "$api" 2>/dev/null || true)"
    fi
    tag="$(printf '%s\n' "$json" \
        | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | tail -n1)"
    [ -n "$tag" ] || fail "no published release found for $REPO (set --version or HFE_VERSION)"
else
    tag="$VERSION"
fi

asset="hfe-$target.tar.gz"
base="https://github.com/$REPO/releases/download/$tag"
url="$base/$asset"

# --- destination -----------------------------------------------------------
[ -n "$DEST" ] || DEST="$HOME/.local/bin"

if [ "$ASSUME_YES" -eq 0 ] && [ -t 0 ]; then
    printf 'Install hfe %s (%s) to %s? [y/N] ' "$tag" "$target" "$DEST"
    read -r reply || reply=""
    case "$reply" in
        y|Y|yes|YES) ;;
        *) say "aborted"; exit 0 ;;
    esac
fi

mkdir -p "$DEST" || fail "cannot create $DEST"
[ -w "$DEST" ] || fail "$DEST is not writable (use --dir, or run with sudo)"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

info "downloading $asset ($tag)"
curl -fsSL -o "$tmp/$asset" "$url" || fail "download failed: $url"

# --- checksum --------------------------------------------------------------
if curl -fsSL -o "$tmp/$asset.sha256" "$url.sha256" 2>/dev/null; then
    if command -v sha256sum >/dev/null 2>&1; then
        have="$(sha256sum "$tmp/$asset" | awk '{print $1}')"
    else
        have="$(shasum -a 256 "$tmp/$asset" | awk '{print $1}')"
    fi
    want="$(awk '{print $1}' "$tmp/$asset.sha256")"
    [ "$have" = "$want" ] || fail "checksum mismatch (have $have, want $want)"
    info "sha256 verified"
else
    info "warning: no .sha256 published for this release; skipping verification"
fi

# --- extract + install -----------------------------------------------------
tar -xzf "$tmp/$asset" -C "$tmp" || fail "could not extract $asset"
bin=""
for cand in "$tmp/hfe" "$tmp"/*/hfe; do
    [ -f "$cand" ] && bin="$cand" && break
done
[ -n "$bin" ] || fail "archive did not contain an 'hfe' binary"

install_bin="$DEST/hfe"
cp "$bin" "$install_bin" || fail "cannot write $install_bin"
chmod +x "$install_bin"

say "installed hfe $tag -> $install_bin"
case ":$PATH:" in
    *":$DEST:"*) ;;
    *)
        say ""
        say "note: $DEST is not on your PATH. Add it with:"
        say "      export PATH=\"$DEST:\$PATH\""
        ;;
esac
say "run: hfe --help"