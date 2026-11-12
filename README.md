# chat — the agentic chat UI (Rust → wasm)

The frontend half of **chat**: a fullscreen agentic chat UI, written in Rust
and compiled to wasm. It owns no agent logic — every `/api/*` call is
forwarded to [angela](../../angela) (`POST /api/chat` NDJSON, `GET /api/agents`,
`GET /api/sessions`, approvals, allowlist, context-window).

```
   edit app/**.rs ──► cargo build --target wasm32 ──► wasm-bindgen ──► web/pkg/*
        │                                                                  │
        │                        /__m_hmr  (one websocket)                 │
        └── chat-server ◄─────────────────────────────────────────────────┘
                   │                                   │
                   │  /api/* ──► angela  :8763         ▼
                   │                                new wasm instance,
                   │                                M.unmount() → M.mount()
                   ▼                                page never reloads,
            static + HMR + wasm rebuild             stores survive
```

## Run it

```bash
# the agent backend must be up first (or send() falls back to echo)
cd /workspace/angela && bun run src/serve.mjs &  # → http://127.0.0.1:8763
./dev.sh                # → http://127.0.0.1:3020
./dev.sh 4000           # another port
./dev.sh --rebuild      # force a fresh wasm build on start
```

`dev.sh` uses **blaze** when it is on `PATH` (see `/workspace/tmp/FAST_RUST.md`:
`blaze` is the `cargo`-compatible fast rebuild — same commands, in-memory
incremental, ~10–20× on edits) and falls back to `cargo` otherwise. It installs
the pinned `wasm-bindgen` CLI into `tools/` and the `wasm32-unknown-unknown`
target if either is missing, then hands the process to the rustex1 dev server,
which builds the frontend before answering the first request.

| you edit | what happens |
|---|---|
| `app/**.rs` | wasm rebuild (~350 ms cargo, less on blaze), hot-swapped — **no reload, state kept** |
| `server/**.rs` | server rebuild, process re-execs itself, client reconnects |
| `web/styles.css` | `<link>` cache-busted in place, zero redraws |
| `web/index.html` | full reload (the document shell changed) |

A compile error does not kill the dev server or cover the page: the last good
build keeps running and the error goes to the terminal.

## Layout

```
chat/
├── app/src/
│   ├── lib.rs       impl App — stores, routes, the 4s session poll
│   ├── store.rs     the one store (`chat`) + angela wire (Rust port of store.js)
│   ├── helpers.rs   esc/markdown/tool-card/telemetry (Rust port of helpers.js)
│   └── pages.rs     demo grid: fullscreen + sidebar, one shared store (port of demo + templates.js)
├── server/src/
│   ├── main.rs      Server::new(config).raw("/api/", …)
│   └── proxy.rs     the streamed forward to angela
├── web/
│   ├── index.html   the document shell
│   └── styles.css   demo-parity theme tokens (+2 kit-leak neutralizers) + chat.css verbatim
├── vendor/rustex1   the kit + dev server (symlink to ../sbox-a/vendor/rustex1)
└── rustex1.json     packages, port, watch and static roots
```

There is no JavaScript in this repository. The only JS on the client is the
two bridge files the rustex1 kit provides (`app.mjs` boots the wasm,
`hot.mjs` speaks the hot-reload protocol) plus the m-js bundle doing the
rendering.

## Where this came from

| source | what it is | what was kept |
|---|---|---|
| [`m-js`](../../m-js) | favourite UI framework — Alpine-style `x-*` directives on a keyed VDOM (`M.component` / `x-component` / `M.store`), docs at `m-js/docs/index.html` | the rendering model; this app drives it from Rust via `m()` hyperscript |
| [`m-js-cmp-chat`](../../m-js-cmp-chat) | reusable cowork-style chat UI for m.js v3 — sidebar + fullscreen, one store contract, `registerChat` + `createChatStore` + `wsTransport` + `helpers` + `templates` + `chat.css` | the whole contract, ported file-for-file: `helpers.rs` ← `helpers.js`, `store.rs` ← `store.js`, `pages.rs` ← `templates.js`, `styles.css` ⊃ `chat.css` |
| [`sbox-a`](../sbox-a) | prior JS→Rust conversion (sandbox wall, same kit) | the project shape: workspace + `rustex1.json` + `dev.sh` + `server/proxy.rs` + `app/{lib,store,pages}.rs` + HMR discipline (stores survive swaps, no serde round trip) |

## angela vs agl — which backend this talks to

Both live next to this repo; they are different layers, not alternatives.

| | `agl` (`/workspace/agl`) | `angela` (`/workspace/angela`) |
|---|---|---|
| what | minimalist Pydantic-AI clone in Bun JS — `Agent.factory({model, system_prompt, tools})`, providers (`generic` OpenAI-compat, `copilot`), `agent.run({prompt})` | thin harness **over agl**: MCP-only tools, sessions (`.angela/sessions/*.json` + delta `.jsonl`), allowlist + approval queue, project jail, `serve.mjs` HTTP API |
| model | `agl-ai` resolves `provider:model` via `~/.config/agl/config.yaml` (`default_model`, per-provider `endpoints`) | same, through agl; agent coffee may pin `model`, CLI `-m` and UI override it |
| tools | `agent.Tool(name, desc, schema, required, fn)` | MCP presets (`file-io`, `term-local`, `todo`, `context-loop`) + policy (`ask`/`strict`/`open`) + `onApproval` |
| talk to it via | `import Agent from 'agl-ai'` | `POST /api/chat` (NDJSON), `POST /api/approve`, `POST /api/abort`, `GET /api/agents|sessions|health|context-window`, `POST /api/event/meta` — this app's data plane |

So: **agl is the model layer, angela is the agent server, chat is the browser
UI**. The chat SPA never imports agl; it speaks angela's HTTP contract through
the local `/api/*` forward, exactly like sbox-a speaks sbox-b. Approval,
allowlist, visibility/goto, todos and the context pie all ride that contract.

## The store contract (parity with `m-js-cmp-chat/README.md`)

The widget reads `M.store("chat")` and calls its methods; missing methods are
no-ops in JS, present here. Field names are unchanged so the template port
stays mechanical: `messages` / `waiting` / `draft` / `allowlist`(+`Baseline`,
`Overridden`, `Open`, `Inited`) / `connected` / `error` / `model` / `models` /
`agents`+`agent` / `todos`(+`Open`) / `contextWindow` / `tokensUsed` /
`statusLabel`+`statusClass`+`elapsed` / `tabList`+`activeTabId` /
`historyList`+`activeSessionId`+`historyOpen`+`isFullscreen` /
`welcomeStarters` / `_stick`.

Methods: `send`, `stop`, `onKeydown`, `onDraftFocus`, `onAgentChange`,
`onModelChange`, `toggleAllowlist`, `onAllowlistInput`, `onAllowlistBlur`,
`decideApproval`, `toggleMsgVisible`, `toggleMsgGoto`, `onGotoLabelInput`,
`ensureGoto`, `isGotoLabelDuplicate`, `newTab`, `closeTab`, `selectTab`,
`toggleHistory`, `toggleFullscreen`, `openSession`, `deleteSession`,
`cleanAllSessions`, `useStarter`, `onMessagesScroll`, `toggleTodos`,
`renderMarkdown`, `toolCallCode`, `toolCallExplanation`,
`toolResultImageNote`, `formatTelemetryHtml`, `shortModel`, `fillModels`,
`fillAgents`. `handle`/`applyIncoming` is `apply_line` (+`apply_card`).

v1 deltas from the JS original (all reversible):
- WS (`wsTransport` to agent-desktop) → HTTP (`POST /api/chat` NDJSON to
  angela). Streaming deltas land on live rows; the `ReadableStream` reader is
  the next step — v1 buffers the body text, then applies lines.
- `pieDash`/`pieUsedK`/`contextTooltip` are computed getters over
  `tokensUsed`/`contextWindow` (same math, no `selectSyncKey`).
- `x-model` binds `draft`/`allowlist` directly (kit `Def` has getters but no
  setters, so the `draftInput` alias has no write side in Rust).
- `fillModels`/`fillAgents` run from `x-effect` with the same sig-guard as
  `fillSelect`, because `x-for` inside native `<select>` is unreliable.

## Environment

| var | default | meaning |
|---|---|---|
| `ANGELA_ORIGIN` | `http://127.0.0.1:8763` | the angela harness this server forwards `/api/*` to |

## Clone it

The kit is a symlink here (`vendor/rustex1 → ../sbox-a/vendor/rustex1`, itself
a submodule of sbox-a), so no `git submodule update` is needed for the kit —
it tracks sbox-a's checkout. (Upstream: `git@github.com:mikesmullin/rustex1.git`.)
