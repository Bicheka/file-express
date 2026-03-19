#!/usr/bin/env bash

VERSION="v0.1.0"

OS="$(uname | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

# Determine the binary name based on OS and ARCH
if [[ "$OS" == "linux" && "$ARCH" == "x86_64" ]]; then
    BIN="fexpress-linux-amd64"
elif [[ "$OS" == "darwin" && "$ARCH" == "x86_64" ]]; then
    BIN="fexpress-macos-amd64"
elif [[ "$OS" == "darwin" && "$ARCH" == "arm64" ]]; then
    BIN="fexpress-macos-arm64"
else
    echo "Unsupported OS/ARCH: $OS/$ARCH"
    exit 1
fi

URL="https://github.com/bicheka/file-express/releases/download/$VERSION/$BIN"

# Download the binary
curl -Lo /usr/local/bin/fexpress "$URL"
chmod +x /usr/local/bin/fexpress

echo "file-express installed! Run 'fexpress --help'"
