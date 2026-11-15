#!/usr/bin/env bash
# chat dev server — Rust backend with angela embedded, Rust→wasm frontend,
# on the rustex1 kit.
#
#   ./dev.sh                 build if needed, serve on the port in rustex1.json
#   ./dev.sh 4000            override the port
#   ./dev.sh --rebuild       force a fresh wasm build on start
#
# angela runs inside chat-server: there is no second daemon to start and no
# port to coordinate. Its store is <ANGELA_ROOT>/.angela, defaulting to this
# directory.
set -euo pipefail
cd "$(dirname "$0")"

# Honour exported credentials first; read the chat Muse key without
# sourcing arbitrary shell commands from the .env file.
if [[ -z "${META_API_KEY:-}" && -z "${MUSE_API_KEY:-}" ]]; then
  CHAT_ENV="${ANGELA_ROOT:-.}/.angela/.env"
  if [[ -r "$CHAT_ENV" ]]; then
    while IFS= read -r line || [[ -n "$line" ]]; do
      line="${line#export }"
      if [[ "$line" =~ ^[[:space:]]*MUSE_API_KEY[[:space:]]*=(.*)$ ]]; then
        value="${BASH_REMATCH[1]}"
        value="${value#"${value%%[![:space:]]*}"}"
        value="${value%"${value##*[![:space:]]}"}"
        if [[ "$value" == \"*\" || "$value" == \'*\' ]]; then value="${value:1:${#value}-2}"; fi
        export MUSE_API_KEY="$value"
        break
      fi
    done < "$CHAT_ENV"
  fi
fi

if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
  echo "chat: adding the wasm32-unknown-unknown target"
  rustup target add wasm32-unknown-unknown
fi

# Pin the wasm-bindgen version the kit pins — a mismatched CLI produces a
# module the browser will not instantiate.
V="$(grep -o 'wasm-bindgen = "=[0-9.]*"' vendor/rustex1/kit/Cargo.toml | grep -o '[0-9.]*' | head -1)"
V="${V:-0.2.129}"

# The dev server looks for a wasm-bindgen CLI in ./tools, vendor/rustex1/tools
# and ../rustex1/tools before falling back to PATH. Keep that promise.
have_cli() {
  ls tools/*/wasm-bindgen >/dev/null 2>&1 \
    || ls vendor/rustex1/tools/*/wasm-bindgen >/dev/null 2>&1 \
    || command -v wasm-bindgen >/dev/null 2>&1
}

if ! have_cli; then
  case "$(uname -s)" in
    Darwin) ARCH="$(uname -m)-apple-darwin" ;;
    *)      ARCH="$(uname -m)-unknown-linux-musl" ;;
  esac
  echo "chat: installing wasm-bindgen ${V} (${ARCH}) into tools/"
  mkdir -p tools
  curl -fsSL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${V}/wasm-bindgen-${V}-${ARCH}.tar.gz" \
    | tar xz -C tools/
fi

# blaze if it is here (seconds off every incremental rebuild), cargo if not
BUILD="${CHAT_BUILD:-}"
if [[ -z "$BUILD" ]]; then
  BUILD=cargo
  command -v blaze >/dev/null 2>&1 && BUILD=blaze
fi
$BUILD build -q -p chat-server

exec ./target/debug/chat-server "$@"
