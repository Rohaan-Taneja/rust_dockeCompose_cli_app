#!/usr/bin/env bash

set -e

REPO="Rohaan-Taneja/rust_dockeCompose_cli_app"
BINARY="dockyard"

echo "Installing dockyard..."

# Detect OS
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux) PLATFORM="linux" ;;
    Darwin) PLATFORM="macos" ;;
    *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

# Detect architecture
case "$ARCH" in
    x86_64) ARCH="amd64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

FILE_NAME="$BINARY-$PLATFORM-$ARCH"

DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$FILE_NAME"

echo "⬇️ Downloading $FILE_NAME..."

curl -L "$DOWNLOAD_URL" -o "$BINARY"

chmod +x "$BINARY"

INSTALL_DIR="/usr/local/bin"

echo "Installing to $INSTALL_DIR..."

if [ -w "$INSTALL_DIR" ]; then
    mv "$BINARY" "$INSTALL_DIR/$BINARY"
else
    sudo mv "$BINARY" "$INSTALL_DIR/$BINARY"
fi

echo "Installation complete!"
echo "Run: DockYard Up path_of_docker_file"