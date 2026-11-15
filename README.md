# chat — the agentic chat UI (Rust → wasm, angela embedded)

A fullscreen agentic chat app: multi-agent conversations with a branching
history, written in Rust and compiled to wasm, served by **one process**.

![The app: session history and agents on the left, a group conversation in
the middle with each agent's turn drawn down its own coloured rail, and the
inspector on the right](docs/screenshot.png)

That one process is the point. The agent half used to be a separate Bun
daemon the frontend proxied to on `:8763`; it is a Rust crate now
([angela](../../angela-rs)), so `chat-server` embeds it directly
(`server/src/api.rs`). One binary to start, one set of logs, no port to
coordinate, and no proxy hop between the model and the browser.

```
   edit app/**.rs ──► cargo build --target wasm32 ──► wasm-bindgen ──► web/pkg/*
        │                                                                  │
        │                       /__m_hmr  (one websocket)                  │
        └── chat-server ◄──────────────────────────────────────────────────┘
             │  │  │                                │
             │  │  └─ static + HMR + wasm rebuild   ▼
             │  │                                new wasm instance,
             │  └─ /api/*  angela, in-process     M.unmount() → M.mount()
             │        └─ agl ──► provider HTTP    page never reloads,
             └─ .angela/conversations.db          stores survive
```

| layer | crate | what it owns |
|---|---|---|
| browser | `chat-app` (`app/`) | every component, template, store and route — Rust → wasm |
| server | `chat-server` (`server/`) | the rustex1 dev server (static + HMR + rebuild + self-restart) with angela mounted on `/api/` |
| harness | [`angela`](../../angela-rs) | conversations (SQLite tree), participants, agent definitions, group rounds, memory, MCP tools, approval queue, allowlist policy, cancellation |
| model layer | [`agl`](../../agl-rs) | `Agent` + providers (OpenAI-compatible, copilot), `provider:model` resolution from `~/.config/agl/config.yaml` |

## Run it

```bash
./dev.sh                # → http://127.0.0.1:3020
./dev.sh 4000           # another port
./dev.sh --rebuild      # force a fresh wasm build on start
```

Nothing else to start. `dev.sh` adds the `wasm32-unknown-unknown` target and
installs the pinned `wasm-bindgen` CLI into `tools/` if either is missing,
exports `MUSE_API_KEY` from `.angela/.env` if it is not already in the
environment, builds `chat-server`, then hands the process to the rustex1 dev
server, which builds the frontend before answering the first request.

It uses **blaze** when that is on `PATH` (the `cargo`-compatible fast rebuild:
same commands, in-memory incremental, ~10–20× on edits) and `cargo` otherwise;
`CHAT_BUILD` forces either.

| you edit | what happens |
|---|---|
| `app/**.rs` | wasm rebuild (~350 ms cargo, less on blaze), hot-swapped — **no reload, state kept** |
| `server/**.rs` | server rebuild, process re-execs itself, client reconnects |
| `web/*.css` | `<link>` cache-busted in place, zero redraws |
| `web/index.html` | full reload (the document shell changed) |

A compile error does not kill the dev server or cover the page: the last good
build keeps running and the error goes to the terminal.

## The app

```
 /                                                    /storybook
 +-------------+------------------------+--------+    the design guide: 40
 | session     | tab bar (one/session)  |        |    routes, one per component,
 | history     +------------------------+ right  |    each mounting that very
 |             |                        | aside: |    component against the
 | agents menu | session panel:         | agent  |    `samples` store — not a
 |             |   transcript + roster  | editor |    drawing of it. Change a
 | settings    |   + composer           | or     |    component and both pages
 +-------------+------------------------+ settings    follow, because there is
   transparent    rounded --panel card  +--------+    only one of it
```

The transcript is built from cards (`user`, `assistant`, `reasoning`,
`tool_call`, `tool_response`, `system`, `error`, `approval`), each a storybook
component. Each card owns one definition — its markup, its CSS (a `CSS` const
beside it, injected by `css.rs`) and the `kind` it answers to — and the
transcript is the list of those cards, composed in `session::card_kids`. The
design guide mounts the same builders against the `samples` store, so a card
is changed in exactly one place and both pages follow. `web/styles.css` is now
the `--mjs-*` token skin and a handful of shared utilities, not the components' look. The composer carries the draft, the model and reasoning-effort
dropdowns, the tool allowlist with its policy mode, the context pie, the
last turn's metrics — and the stop button.

**Stopping a turn** (`POST /api/abort`) cancels the provider requests in
flight for that session, rather than only closing the browser's stream: the
turn is driven inside `tokio::select!` against a per-session switch, so a stop
*drops* the `reqwest` future, which closes the connection, which cancels the
request. Every model call on the turn path is covered — the inference, each
group participant's, the relevance decision, the lead's routing call. What was
generated before the stop is kept, followed by a `stopped` event that replays
as a "Stopped." row, because a reply that halted halfway and one that finished
must not look identical after a reload.

### Routes

| route | what |
|---|---|
| `/` | the app shell above (`app/src/pages/home.rs`) |
| `/storybook`, `/storybook/*` | the design guide, one route per component |

### Stores

| store | owns |
|---|---|
| `live` | the open conversation: `sessions`, `tabs`, `activeId`, `messages`, the streaming `streamText`/`streamReasoning`, participants + `handles`/`accents`/`icons`, branches + `cursor`, allowlist, metrics, `waiting`/`stopping` |
| `agents` | the Agents rail and the editor draft — kept apart from `live` because a definition outlives every conversation that uses it, and a draft must not vanish because somebody opened another session |
| `settings` | project settings, persisted to SQLite rather than browser storage: they describe the project, so they should be the same in any browser that opens it |
| `home` | the shell's aside collapse (a grid track has to shrink with it, so `x-show` is not enough) |
| `samples` | the design guide's conversation and session list: the same shape as `live`, filled with samples. **Every** `/storybook` route mounts the real component against it — there is no second, hand-built drawing of anything, so the guide cannot fall behind the app |
| `dropdown`, `tabbook`, `automationDraft` | widget-local state |

Nothing deserializes into Rust structs: angela answers JSON, `api::get` hands
back a `JsValue`, and that value goes into the store for m-js to read. A serde
round trip would buy nothing.

## Agent definitions

Agents are rows in the database, created and edited in the right-hand column
(`components/agent_editor.rs`), not files:

| field | |
|---|---|
| `name`, `description` | the handle it is mentioned by, and what it is for |
| `soul`, `goals`, `memory` | the system prompt's three parts; `memory` is the one the agent may rewrite itself |
| `engagement` | `engaged` (take part), `invoked` (only when `@mentioned`), `relevant` (a decision model judges each turn) |
| `model`, `reasoningEffort`, `maxTurns` | per-agent overrides; blank means the project default, which is a different thing from `0` |
| `allowlist`, `policyMode`, `mcp` | which tools it may call, and whether calls are `ask`/`strict`/`open` |
| `accentColor`, `icon` | how it is drawn — a hex accent, and a bare [Phosphor](https://phosphoricons.com) name (`robot`, `dog`, `terminal`). Both default when blank, so clearing the box is how an agent goes back to the app's own look |

The appearance fields are honoured everywhere an agent appears: the rail, the
roster seats, the transcript byline and the editor's own header.

## Layout

```
chat/
├── app/src/
│   ├── lib.rs                impl App — stores, routes, the 4s session poll
│   ├── api.rs                every /api/* call + the NDJSON reader and its AbortController
│   ├── helpers.rs            escaping, markdown, telemetry formatting + tests
│   ├── css.rs                per-component <style> injection, updated on HMR swap
│   ├── pages.rs              router wiring; pages/home.rs the shell, pages/storybook.rs the guide
│   └── components/*.rs       ~50 components: live + agents + settings stores, the cards,
│                             the composer, the rail, the roster, the form widgets
├── server/src/
│   ├── main.rs               Server::new(config).raw("/api/", api::handle_api)
│   └── api.rs                angela in-process: ResponseSink → the dev server's Stream,
│                             one NDJSON frame per row (plus /api/models, /api/composer-settings)
├── web/
│   ├── index.html            the document shell
│   ├── styles.css            theme tokens + chat.css
│   └── storybook.css         the `:is(#sb, #home)` token ramp the app and the guide share
├── .angela/                  the project's own store (gitignored) — see below
├── tmp/*.md                  design notes: STORYBOOKING, GROUP_CHAT, DB, COMPACT, AGENTS, …
├── vendor/rustex1            the kit + dev server (symlink to ../sbox-a/vendor/rustex1)
└── rustex1.json              packages, port, watch and static roots
```

There is no JavaScript in this repository. The only JS on the client is the
two bridge files the rustex1 kit provides (`app.mjs` boots the wasm, `hot.mjs`
speaks the hot-reload protocol) plus the m-js bundle doing the rendering.

## Storage

Everything the project knows lives beside it, under `.angela/` (gitignored, or
wherever `ANGELA_ROOT` points):

| path | |
|---|---|
| `conversations.db` | SQLite. The conversation is a **tree**: nodes name a parent, a cursor names the active leaf, and forking is "move the cursor, then append". Also participants, agent definitions, settings, and the memory summaries. |
| `agents/*.json` | import-only: the old file-based definitions, read once by the migration |
| `sessions/` | the legacy `.json`/`.jsonl` pair, kept as the importer's input and the rollback plan |
| `mcp.json` | MCP server definitions the tool presets resolve against |
| `.env` | `MUSE_API_KEY`, read by `dev.sh` without sourcing the file |

## The HTTP contract

Served in-process on `/api/*`. The same routes serve from angela's standalone
`angela-serve` binary, which is what keeps the seam honest.

| | |
|---|---|
| conversation | `POST /api/chat` (NDJSON stream), `POST /api/abort`, `POST /api/approve`, `POST /api/event/meta` |
| sessions | `GET /api/sessions`, `GET|DELETE /api/sessions/{id}`, `POST /api/session/new`, `DELETE /api/sessions` |
| the tree | `GET /api/sessions/{id}/tree`, `POST /api/sessions/{id}/cursor`, `POST /api/nodes/{id}/visibility`, `POST /api/nodes/{id}/goto` |
| the room | `GET|POST /api/sessions/{id}/participants`, `POST|DELETE /api/participants/{pid}` |
| memory | `GET /api/sessions/{id}/view`, `GET /api/sessions/{id}/zoom`, `POST /api/sessions/{id}/compact` |
| agents | `GET|POST /api/agents`, `GET|POST|DELETE /api/agents/{name}` |
| settings | `GET|POST /api/settings`, `POST /api/sessions/{id}/settings`, `POST /api/allowlist`, `POST /api/composer-settings` |
| the rest | `GET /api/health`, `GET /api/models` (from agl's user config) |

## Environment

Nothing is required to chat: the defaults are the project directory and agl's
own config. The one default that points elsewhere is the decision endpoint,
which only agents set to `relevant` engagement ever call.

| var | default | meaning |
|---|---|---|
| `ANGELA_ROOT` | the server's working directory | where `.angela/` lives |
| `ANGELA_MCP_ROOT` | `/workspace/mcp` | where MCP servers are looked up |
| `CHAT_MCP_CONFIG` | `<root>/.angela/mcp.json` | the MCP definitions file |
| `ANGELA_COMPACTOR` | the session's model | the model that writes memory summaries |
| `ANGELA_DECISIONS_URL` | `http://127.0.0.1:1237/v1` | the decision endpoint — deliberately a *different*, smaller server from the one that generates, because deciding and generating want different models (`_MODEL` and `_API_KEY` go with it) |
| `MUSE_API_KEY` | from `.angela/.env` | the provider credential |
| `CHAT_BUILD` | `blaze` if present, else `cargo` | the build command `dev.sh` uses |

## Where this came from

| source | what it is | what was kept |
|---|---|---|
| [`m-js`](../../m-js) | favourite UI framework — Alpine-style `x-*` directives on a keyed VDOM (`M.component` / `x-component` / `M.store`), docs at `m-js/docs/index.html` | the rendering model; this app drives it from Rust via `m()` hyperscript |
| [`m-js-cmp-chat`](../../m-js-cmp-chat) | reusable cowork-style chat UI for m.js v3 — sidebar + fullscreen, one store contract, `registerChat` + `createChatStore` + `wsTransport` + `helpers` + `templates` + `chat.css` | ported file-for-file to begin with; the app has since grown past it into components, and in Nov 2026 the ported demo page, its store and its markup were deleted — what survives is `helpers.rs`, the `--mjs-*` token skin in `styles.css`, and the layout ideas |
| [`sbox-a`](../sbox-a) | prior JS→Rust conversion (sandbox wall, same kit) | the project shape: workspace + `rustex1.json` + `dev.sh` + HMR discipline (stores survive swaps, no serde round trip) |
| [`angela`](../../angela) / [`agl`](../../agl) | the original Bun/JS harness and model layer | the on-disk format and the HTTP contract, both reproduced by the Rust ports — including the surprises, recorded rather than quietly fixed |

## Clone it

The kit is a symlink here (`vendor/rustex1 → ../sbox-a/vendor/rustex1`, itself
a submodule of sbox-a), so no `git submodule update` is needed for the kit —
it tracks sbox-a's checkout. (Upstream: `git@github.com:mikesmullin/rustex1.git`.)
