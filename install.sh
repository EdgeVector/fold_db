#!/bin/sh
# FoldDB CLI Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/shiba4life/fold_db/master/install.sh | sh

set -e

REPO="shiba4life/fold_db"
BINARY_NAME="datafold_cli"

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
  echo "Please build from source: cargo install --git https://github.com/$REPO --bin $BINARY_NAME"
  exit 1
fi

ARTIFACT="${BINARY_NAME}-${OS_LABEL}-${ARCH_LABEL}"
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

# Download binary
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$TAG/$ARTIFACT"
TMP_DIR="$(mktemp -d)"
TMP_FILE="$TMP_DIR/$BINARY_NAME"

echo "Downloading $ARTIFACT..."
if command -v curl >/dev/null 2>&1; then
  curl -fSL --progress-bar -o "$TMP_FILE" "$DOWNLOAD_URL"
elif command -v wget >/dev/null 2>&1; then
  wget -q --show-progress -O "$TMP_FILE" "$DOWNLOAD_URL"
fi

chmod +x "$TMP_FILE"

# Install binary
INSTALL_DIR="/usr/local/bin"
if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
  echo "Installed to $INSTALL_DIR/$BINARY_NAME"
elif command -v sudo >/dev/null 2>&1; then
  echo "Installing to $INSTALL_DIR (requires sudo)..."
  sudo mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
  echo "Installed to $INSTALL_DIR/$BINARY_NAME"
else
  # Fallback to ~/.local/bin
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
  mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
  echo "Installed to $INSTALL_DIR/$BINARY_NAME"
  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
      echo ""
      echo "NOTE: $INSTALL_DIR is not in your PATH."
      echo "Add it by running:  export PATH=\"$INSTALL_DIR:\$PATH\""
      ;;
  esac
fi

# Cleanup
rm -rf "$TMP_DIR"

echo ""
echo "FoldDB CLI $VERSION installed successfully!"
echo "Run '$BINARY_NAME --help' to get started."
