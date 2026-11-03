#!/bin/bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "BetterCapture requires macOS." >&2
  exit 1
fi

if [[ ! -d /Applications/BetterCapture.app && ! -d "$HOME/Applications/BetterCapture.app" ]]; then
  if ! command -v brew >/dev/null 2>&1; then
    echo "Install Homebrew from https://brew.sh, then run ./start.sh again." >&2
    exit 1
  fi
  brew install --cask bettercapture
fi

exec /usr/bin/open -a BetterCapture
