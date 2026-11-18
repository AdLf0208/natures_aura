# Nature's Aura — a Horizon port

**Nature's Aura** is a Minecraft mod. A Minecraft mod about using Aura for unique mechanics. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Ellpeck/NaturesAura](https://github.com/Ellpeck/NaturesAura) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Nature's Aura ([source](https://github.com/Ellpeck/NaturesAura)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `naturesaura.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What works

- **Aura drain spots.** A chunk carries `pos -> amount` spots (its base is
  1,000,000 aura), kept in a typed chunk attachment and summed over a radius, as
  the original's `AuraChunk` does.
- **The aura bloom** (`naturesaura:aura_bloom`), the natural source: a plant that
  stores 150,000 aura in its chunk the first time it ticks, once.
- **The nature's altar** (`naturesaura:nature_altar`) inside its 9x4x9
  multiblock: it pulls up to 300 aura a tick out of the area into its own
  500,000 container (in the Overworld and the Nether, as upstream) and
  transmutes one item at a time.
- **Three recipes**, every altar recipe that needs no catalyst:
  `iron_ingot -> infused_iron` (15,000 aura, 80 ticks),
  `stone -> infused_stone` (7,500 aura, 40 ticks),
  `gold_ingot -> tainted_gold` (3,000 aura, 300 ticks).
- **Materials**: the gold bricks the multiblock needs are craftable in-game
  (`gold nuggets + short grass + leaves -> gold_fiber`, then
  `stone_bricks + gold_fiber -> gold_brick`).

## Not yet ported

No omissions are recorded for this slice; `notes.md` is the authoritative list.

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `naturesaura.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Nature's Aura was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Ellpeck/NaturesAura

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run naturesaura` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/naturesaura/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
