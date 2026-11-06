#!/usr/bin/env bash
set -euo pipefail

REPO="YOUR_GITHUB_USERNAME/rax"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="rax"

echo "=> Fetching latest release of $BINARY_NAME from GitHub..."
LATEST_TAG=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
  echo "Error: Could not find a release for $REPO."
  exit 1
fi

ARCHIVE="rax-${LATEST_TAG}-x86_64-unknown-linux-gnu.tar.gz"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/${LATEST_TAG}/${ARCHIVE}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "=> Downloading $ARCHIVE ($LATEST_TAG)..."
curl -sSL "$DOWNLOAD_URL" -o "$TMP_DIR/$ARCHIVE"
curl -sSL "${DOWNLOAD_URL}.sha256" -o "$TMP_DIR/${ARCHIVE}.sha256"

echo "=> Verifying SHA-256 checksum..."
(cd "$TMP_DIR" && sha256sum -c "${ARCHIVE}.sha256")

echo "=> Extracting binary..."
tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"

echo "=> Installing $BINARY_NAME to $INSTALL_DIR (may require sudo)..."
if [ -w "$INSTALL_DIR" ]; then
  install -m 755 "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
else
  sudo install -m 755 "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
fi

echo "✔ $BINARY_NAME $LATEST_TAG successfully installed! Run 'rax --help' to get started."
