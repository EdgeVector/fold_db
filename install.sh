#!/bin/sh
# FoldDB Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/shiba4life/fold_db/master/install.sh | sh

set -e

REPO="shiba4life/fold_db"
BINARIES="folddb_server folddb"

# Detect OS
OS="$(uname -s)"
case "$OS" in
  Darwin) OS_LABEL="macos" ;;
  Linux)  OS_LABEL="linux" ;;
  *)
    echo "Error: Unsupported operating system: $OS"
    exit 1
    ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  arm64|aarch64) ARCH_LABEL="aarch64" ;;
  x86_64|amd64)  ARCH_LABEL="x86_64" ;;
  *)
    echo "Error: Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

# Linux only supports x86_64 for now
if [ "$OS_LABEL" = "linux" ] && [ "$ARCH_LABEL" = "aarch64" ]; then
  echo "Error: Linux arm64/aarch64 builds are not yet available."
  echo "Please build from source: cargo install --git https://github.com/$REPO"
  exit 1
fi

echo "Detected platform: ${OS_LABEL}-${ARCH_LABEL}"

# Get latest release tag
echo "Fetching latest release..."
LATEST_URL="https://api.github.com/repos/$REPO/releases/latest"

if command -v curl >/dev/null 2>&1; then
  RELEASE_JSON="$(curl -fsSL "$LATEST_URL")"
elif command -v wget >/dev/null 2>&1; then
  RELEASE_JSON="$(wget -qO- "$LATEST_URL")"
else
  echo "Error: curl or wget is required to download FoldDB."
  exit 1
fi

# Extract tag name (works without jq)
TAG="$(echo "$RELEASE_JSON" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"

if [ -z "$TAG" ]; then
  echo "Error: Could not determine latest release version."
  echo "Visit https://github.com/$REPO/releases to download manually."
  exit 1
fi

VERSION="$(echo "$TAG" | sed 's/^v//')"
echo "Latest version: $VERSION"

# Determine install directory
INSTALL_DIR="/usr/local/bin"
NEED_SUDO=false
if [ ! -w "$INSTALL_DIR" ]; then
  if command -v sudo >/dev/null 2>&1; then
    NEED_SUDO=true
  else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
  fi
fi

TMP_DIR="$(mktemp -d)"

# Download SHA256SUMS.txt (best-effort — older releases may not have it)
CHECKSUMS_URL="https://github.com/$REPO/releases/download/$TAG/SHA256SUMS.txt"
CHECKSUMS_FILE="$TMP_DIR/SHA256SUMS.txt"
HAS_CHECKSUMS=false
echo "Downloading checksums..."
if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "$CHECKSUMS_FILE" "$CHECKSUMS_URL" 2>/dev/null && HAS_CHECKSUMS=true
elif command -v wget >/dev/null 2>&1; then
  wget -q -O "$CHECKSUMS_FILE" "$CHECKSUMS_URL" 2>/dev/null && HAS_CHECKSUMS=true
fi

if [ "$HAS_CHECKSUMS" = false ]; then
  echo "Warning: SHA256SUMS.txt not found for this release — skipping verification."
fi

# Download and install each binary
for BINARY_NAME in $BINARIES; do
  ARTIFACT="${BINARY_NAME}-${OS_LABEL}-${ARCH_LABEL}"
  DOWNLOAD_URL="https://github.com/$REPO/releases/download/$TAG/$ARTIFACT"
  TMP_FILE="$TMP_DIR/$ARTIFACT"

  echo "Downloading $ARTIFACT..."
  if command -v curl >/dev/null 2>&1; then
    curl -fSL --progress-bar -o "$TMP_FILE" "$DOWNLOAD_URL"
  elif command -v wget >/dev/null 2>&1; then
    wget -q --show-progress -O "$TMP_FILE" "$DOWNLOAD_URL"
  fi

  # Verify checksum if available
  if [ "$HAS_CHECKSUMS" = true ]; then
    EXPECTED_SHA="$(grep "  ${ARTIFACT}\$" "$CHECKSUMS_FILE" | cut -d' ' -f1 || true)"
    if [ -n "$EXPECTED_SHA" ]; then
      if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL_SHA="$(sha256sum "$TMP_FILE" | cut -d' ' -f1)"
      elif command -v shasum >/dev/null 2>&1; then
        ACTUAL_SHA="$(shasum -a 256 "$TMP_FILE" | cut -d' ' -f1)"
      else
        ACTUAL_SHA=""
        echo "Warning: No sha256sum or shasum found — skipping verification for $ARTIFACT."
      fi
      if [ -n "$ACTUAL_SHA" ] && [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
        echo "Error: Checksum mismatch for $ARTIFACT!"
        echo "  Expected: $EXPECTED_SHA"
        echo "  Actual:   $ACTUAL_SHA"
        rm -rf "$TMP_DIR"
        exit 1
      elif [ -n "$ACTUAL_SHA" ]; then
        echo "Checksum verified for $ARTIFACT."
      fi
    fi
  fi

  chmod +x "$TMP_FILE"

  if [ "$NEED_SUDO" = true ]; then
    sudo mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
  else
    mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
  fi
done

# Cleanup
rm -rf "$TMP_DIR"

# Check PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo ""
    echo "NOTE: $INSTALL_DIR is not in your PATH."
    echo "Add it by running:  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac

echo ""
echo "FoldDB v${VERSION} installed!"
echo "  Run 'folddb_server' to start, then visit http://localhost:9001"
echo "  Run 'folddb --help' for the CLI"
