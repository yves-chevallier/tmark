#!/bin/sh
# Builds `tmark-lsp` in release mode and copies it under `bin/` so that
# `npm run package` produces a .vsix that carries the server for this
# platform. Per-platform downloads are a later item (design/08-lsp.md).
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
root=$(cd "$here/../.." && pwd)
cargo build --release -p tmark-lsp --manifest-path "$root/Cargo.toml"
mkdir -p "$here/bin"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) bin=tmark-lsp.exe ;;
  *) bin=tmark-lsp ;;
esac
cp "$root/target/release/$bin" "$here/bin/$bin"
echo "bundled $here/bin/$bin"
