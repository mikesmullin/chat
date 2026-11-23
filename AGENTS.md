# AGENTS.md — how to work in this repo

The one rule everything else follows:

> **The UI is a composable hierarchy of reusable parts. Every component is
> defined exactly once. There are no one-off derivations.**

If you change something on `/`, it changes on `/storybook` too, and the other
way around — not because the two were kept in sync, but because there is only
one of it to change.

## The lego brick metaphor

`/storybook` is the **design guide for the entire application**: the box of
lego bricks. When a new page is built, that is where its parts come from.
Everything presented to a user on a `/` route is composed **exclusively** of
those bricks.

So a brick is not an illustration of a component — it *is* the component. The
guide mounts the same builder the app mounts, with a different store behind
it. A second drawing of a card, for the guide to show "what it looks like", is
the thing this repo does not do.

The failure mode this exists to prevent is specific, and it had already
happened once: the app grew turn rails, dots, agent colours and fold cards,
while the guide went on showing flat bubbles and the old tool-card chrome,
because each card had **two** definitions — a static one the guide drew and a
store-bound one the transcript drew. Nobody was looking at the static one, so
it rotted. Worse, fixing a card meant being told "here also, you missed a
spot". There should be exactly one spot.

## What that means in practice

### A component owns one definition of itself

Each `app/src/components/*.rs` file owns, for one thing:

| | |
|---|---|
| its markup | one builder, taking **expressions**, never baked-in text |
| its CSS | a `CSS` const beside the markup, injected by `register()` |
| its condition | the `kind` of row it answers to, as its own `x-if` |
| its demo | the same builder, over the samples store |

Nothing about a card lives anywhere else — not in `web/styles.css`, not
inlined into whichever parent happens to draw it today.

### Builders take the store, not the data

The convention is a store **expression** threaded down from the page:

```rust
pub fn parts(st: &str, v: &str) -> Vec<Node>
//              ^ "$store.live" or "$store.samples"
//                        ^ the expression naming the row: "m", "t.message"
```

`/` passes `$store.live`; `/storybook` passes `$store.samples`. A builder that
hardcodes `$store.live` cannot be shown in the guide without being copied, and
a copy is the bug. The same applies to side effects: a dropdown that persists
its pick takes the store it persists to, so the guide's dropdown writes to the
guide's store and the app's writes to the session.

### Composition, not re-implementation

A parent lists its children; it does not draw them:

```rust
fn card_kids(st: &str, v: &str) -> Vec<Node> {
    let mut out = Vec::new();
    out.extend(user_card::parts(st, v));
    out.extend(assistant_card::parts(st, v));
    …
}
```

That function decides *which* cards a transcript can hold. Each card decides
what it looks like. Likewise `fold_card` owns collapse-and-click once, and
`reasoning_card`, `tool_call_card`, `prompt_card` and `decision_card` supply
only a glyph, a label, a note and a body.

### The samples store

`components/samples.rs` is the guide's data: one conversation with a row of
every kind, a session list, a model list. Its content is built with the app's
own renderers (`markdown_to_html`, `metrics_html`) rather than hand-written
HTML — a sample that spells out its own markup is one more copy to keep in
step. Its action methods are inert, except where being clickable is the point
(fold cards open, tabs switch).

When you add a row kind, add a sample for it and a route. A brick with no page
is a brick nobody can find.

## Checks that keep it honest

Before calling UI work done:

- **No duplicate selectors.** A selector must appear in exactly one place —
  `web/styles.css` *or* one component. Two copies drift; one of them silently
  wins and the other becomes fiction that reads as fact.
- **No orphan CSS.** A rule whose classes nothing renders is dead. When a
  component's markup changes, its old rules go with it.
- **`web/styles.css` is the skin, not the components' look.** What belongs
  there: the `--mjs-*` token ramp, the page frame, shared utilities. What does
  not: anything dressing one component.
- **Nothing hardcodes `$store.live` below the page level.**
- **The console is clean.** `[rustex1] template lint: clean` on boot, and no
  `could not set ...` errors — those mean a template is writing to a store
  field that no longer exists, and they fail silently in the UI.

The build must be warning-free: an unused component function usually means a
second definition was left behind, which is this document's whole subject.

## The build

`./dev.sh` uses **blaze** when it is on `PATH`, cargo otherwise
(`CHAT_BUILD=cargo` forces cargo). Blaze links *incremental patch objects*,
and across a big structural change — a new module, a new trait-object type,
new parameters threaded through a call chain — those patches can go stale
and link into a binary that **segfaults in perfectly safe code** (seen once:
a new `Arc<dyn Fn>` hook crashed inside a tool closure chain; both cores
shared a build-id that matched neither a clean cargo nor a clean blaze
build, and `rm -rf target/blaze` fixed it). A wild SIGSEGV after a refactor
is suspect stale blaze state first, your code second: wipe `target/blaze`,
rebuild, and only then start reading stack traces.

The devserver's watcher covers `server/` and `vendor/rustex1`, and HMR
covers the wasm and static files — but the path dependencies (`angela-rs`,
`agl-rs`, whatever else `Cargo.toml` points at sideways) are **not** in the
watch set. After changing one, restart the server (`pkill -x chat-server`
then `./dev.sh`) or the running binary keeps serving the old code — which
looks exactly like your change not working.

## Template gotchas (m-js)

- **Class bindings clobber; class *calls* replace.** Two
  `bind("class", …)` on one element are one binding — the later wins and
  the earlier silently disappears (a `ph ph-…` prefix vanished this way and
  the glyph with it). And `Node::icon("x").class("y")` does **not** append:
  it replaces the icon's classes. The fix is always the same: one binding
  (or one `.class()`) carrying the **full** class list per state.
- **`model` syncs on `change`, not `input`.** A handler that reads the
  store in the same event that typed the text — or a debounce firing before
  any blur — sees the **previous** value. Read `$event.target.value` at the
  moment of the event; that is what the user actually typed.
- **`for_each` repeats the element it is bound to**, not its children.
  Bind it to the row; binding it to a list container clones the whole
  container once per item.

## Verification and tests

- **Measure the visible tree, after it settles.** The app renders hidden
  duplicate DOM (the mobile overlay shares class names with the desktop
  rail) — unfiltered probes return zero-height phantoms and lie about
  geometry. Filter by `getBoundingClientRect().height > 0 && offsetParent`,
  and read computed styles after transitions finish: a 140ms color fade
  read at t=0 reported the old color. Screenshots — looked at with your own
  eyes — caught what every probe missed.
- **Parallel tests need unique temp dirs.** Two suites using
  `temp_dir().join(format!("…-{}", now_ms()))` collided within one
  millisecond, shared one database, and failed in ways that pointed
  everywhere except at the collision. Use pid plus an atomic counter.

## Where things live

```
app/src/
├── pages.rs            the router, and nothing else
├── pages/home.rs       `/` — the three-column shell
├── pages/storybook.rs  `/storybook` — one route per component
└── components/
    ├── samples.rs      the guide's store
    ├── live.rs         the app's store
    ├── session.rs      the transcript: composes the cards, owns the turn rail
    ├── fold_card.rs    collapse-and-click, for every evidence row
    ├── name_tag.rs     the byline
    └── *_card.rs       one row kind each
```

`web/styles.css` is the shared skin. `css.rs` injects each component's CSS as
`<style id="css-<name>">`, updated on hot swap.

## House rules

- Don't commit unless asked.
- Use `rg`, never `grep`.
- Use the VS Code integrated browser to look at the app; reuse one tab.
- Don't create or translate MCP servers — run the ones that exist.
- The running app at `/` is **authoritative** for how something should look.
  If the guide disagrees, the guide is the thing that is wrong — and the fix
  is never to edit the guide's copy, because there should not be one.
- When a citation points at a DOM element (a pasted xpath + bounds), verify
  the change against *that* element's geometry and behavior — not against
  something that merely looks similar in a screenshot.
- **No real names in committed files.** The placeholders are **Bob** and
  **Alice**; the databases may keep real names (they are gitignored), git
  may not. cover ALL-CAPS variants too (`\bBob\b` misses `BOB`).
- Say so when a request seems off, and propose the correction instead of
  building it — a wrong premise caught early is cheaper than a wrong build.
