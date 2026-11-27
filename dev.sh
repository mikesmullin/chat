#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

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

V="$(grep -o 'wasm-bindgen = "=[0-9.]*"' vendor/rustex1/kit/Cargo.toml | grep -o '[0-9.]*' | head -1)"
V="${V:-0.2.129}"

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

BUILD="${CHAT_BUILD:-}"
if [[ -z "$BUILD" ]]; then
  BUILD=cargo
  command -v blaze >/dev/null 2>&1 && BUILD=blaze
fi
$BUILD build -q -p chat-server

exec ./target/debug/chat-server "$@"
