# Nature's Aura — slice 1 notes

**Mod:** Nature's Aura (`naturesaura`) 41.10, by Ellpeck, MIT.
**Class / tier:** `content` / `deep` — but slice 1 is the **logic** shape of it:
one aura/ritual loop with its natural source, no menu, no screen, no fluids, no
energy.
**Approach** (`horizon/docs/modpacks.md`): an **agent port** of the behaviour
(`runtime.rs` + glue) with a **carry-over** of the slice's assets and data
(`content/`, script-generated from the jar).
**Licence:** MIT read from the jar (`META-INF/neoforge.mods.toml`), **rank 0**;
local-only until the maintainer's approval (ARCHITECTURE.md §8).
**Pin:** Horizon nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (SDK,
2026-11-18), horizon-mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher
0.2.78; Minecraft 26.3. Before the migration (2026-11-18) the campaign pin was
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` / mixins `5a64af938ec86b00d53d16a408aabc0038fb3e57`.
**Jar:** `ports/_shared/licence-cache/NaturesAura-41.10.jar`, SHA-1
`58e4f9a07e5320432d11aaa3d876337edeeeb8db` (the pack record's), **kept**.
**Upstream:** tag `41.10` = `70b93d37037f3559312834f0f42e824431090b64`; the
analysis clone was `/tmp/opencode/naturesaura-upstream` (not shipped).

**Format: the SDK package (2026-11-18).** This port was migrated from the retired
`.hnmod` container to the flat SDK package — one `naturesaura.hmod` carrying
`horizon-mod.toml`, `Cargo.toml`, `src/lib.rs` (one `impl Hooks` plus three Parts),
`src/runtime.rs`, `assets/` and `data/`. See
[`## Migration to the SDK format`](#migration-to-the-sdk-format) for the file map,
the hook table and every trap. `src/runtime.rs` is **byte-identical** to the retired
`mixins/naturesaura/runtime.rs` (sha256
`fa7273e5d8a238447f93fb43f5ef7df094941e44a78d1dc1c3412d0f6029a393`), including the
file inside the built package.

## The slice boundary (and why)

Implemented: the chunk aura drain spots (base 1,000,000 + spots), the aura bloom
(150,000 aura, once, when it appears), the nature's altar's 9x4x9 multiblock, its
500,000 container (300 aura a tick from the area, only in the Overworld and the
Nether), the ritual for the three catalyst-free recipes, the right-click
insert/take, and the materials that make the multiblock buildable in-game.

Why this is the smallest honest loop: the altar's only inputs are aura and one
item, its only output one item, so the loop closes with no GUI, no energy, no
fluid and no multiblock-machine API; yet every piece of it is the mod's own core
(the aura store, the natural source, the ritual, the completion gate) and every
piece is observable from `horizon-ctl` (aura numbers, the spot list, the altar's
container, the slot's item).

Not implemented (each is a later slice, listed in `FINDINGS.md` §3.1): the other
74 blocks and 142 items; the 31 catalyst altar recipes and the 2
component-output ones; the drain-spot passive effects; the item aura containers
(`aura_bottle`, `aura_cache`, `aura_trove`); the RF converter; the other
generators; the tree ritual, the offering table and the altars of birthing; the
bloom's worldgen (worldgen sorts last); the guide book; the client aura overlay
and `PacketAuraChunk` sync.

## Gaps and deviations (what the API or the slice changes)

| Gap | Shape | What the slice does instead |
|---|---|---|
| `IAuraType` per dimension | the pin has no aura-type registry | `game.native_dimension()`: the altar pulls in `minecraft:overworld` and `minecraft:nether`, and not in the End — upstream's rule, no type objects |
| Aura spot storage | NeoForge attaches to `LevelChunk`; the pin has `Holder::Chunk` attachments | a `ChunkAura` codec compound `{spots: [{x,y,z,amount}]}`; same arithmetic, same NBT spirit |
| `PacketAuraChunk` / client aura overlay | no client HUD in the slice | `sync: false`; the readouts are commands. A later slice adds the overlay and then needs `sync: true` |
| Per-item container rules | `ModContainer` has per-slot, not per-item, `inputs`/`outputs` | every side reaches the altar's one slot; the click refuses a non-recipe insert, and only a finished product leaves by hand (a hopper may take the input — recorded) |
| The click is asked more than once | the game asks `right_click_block` again for the second hand of one click (the probe logged `held=Some(iron)` then `held=None` on the same click) | taking only a **finished product** by hand (upstream's `canExtract` refuses a recipe input) keeps the second ask from undoing the insert; upstream's `putStackOnTile` would take an input back by hand, and here only automation can (recorded) |
| The altar's waterlogging | the slice's block templates have no waterlogged state | not carried (`BlockNatureAltar.hasWaterlogging`) |
| The bloom's ground set | `minecraft:poppy` allows mycelium, moss, mud and rooted dirt too | upstream's five grounds are covered; the four extra ones are a template deviation |
| Aura-container items | the altar's "fill the held item's aura" branch | not in the slice (no such items carried); the tick goes straight to the recipe |
| The generators' cap (`area + toAdd <= 2 * DEFAULT_AURA`) | upstream `canGenerateRightNow` on generators | the bloom is **uncapped** upstream (it calls `generateAura` directly), so the slice's only generator needs no cap; the rule belongs to the moss/flower/oak generators of the next slice |
| The altar's own recipe | upstream has none (the tree ritual makes it) | given by the harness in the e2e; a player gets it from the tree ritual, which is a later slice |
| Block-entity writes in a tick | the altar writes its own state each tick | it writes the block entity's saved compound the tick part is handed (`event.entity.nbt`, the SDK's checked view) and leaves the reply `Changed`, so the chunk saves what the tick did; the 0.1.0 e2e proved the state survives across ticks and the ritual completes |

## Verification (evidence in `VALIDATION.md` and `ports/_shared/evidence/naturesaura/`)

| Step | Command | Result |
|---|---|---|
| Runtime tests | `cargo test --locked --offline` | red 3/19, then 18 passed, 0 failed |
| Lints | `cargo clippy --all-targets -- -D warnings`, `cargo fmt` | clean |
| Stage | `python3 tools/horizon.py stage` | `stage_changed: true` |
| Game check | `python3 tools/horizon.py check` | `cargo_exit: 0`, 0 warnings |
| Embedded tests | `python3 tools/horizon.py test` | `game::naturesaura::tests` 18 passed, 0 warnings |
| Content build | `tools/package.sh` → `cargo horizon-mod build/check` | 21,529 B, 53 entries, `no code: content only`, same SHA-256 across two builds |
| Bundle | `tools/package.py` | 32,749 B, 13 entries, embedded content `cmp`-identical |
| e2e (content dev mod) | `build-slot.sh e2e … -- tools/e2e-all.sh` | `PASS` + `EXIT:0` (`e2e-slice1.log`) |
| e2e (one file installs both) | `EMBEDDED=1 CONTROL=1` | `PASS` + `EXIT:0`; the control shows the ids unknown without the embedded file (`e2e-embedded.log`) |
| e2e (can fail) | `SELFTEST=1` | `FAIL` + `EXIT:1` (`e2e-selftest.log`) |
| Log checker | `check-e2e-logs.py naturesaura --strict` | 3 logs, ok=3, exit 0 |
| Registry gate | `upstream_ready.py naturesaura` | blocked only on `status=pending` and the post-run rebuild, both the orchestrator's; every other gate holds |

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-naturesaura.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** restored recipe index out of bounds panics the altar tick
  **Repair:** Fixed: `runtime::recipe_at` bounds-checks the table, `Altar::step` drops a run whose index is gone, and the restore path (`recipe_at_index`) filters invalid saves. Test: `a_saved_recipe_index_past_the_table_is_dropped_not_indexed`.
- **Reviewer:** description: ',000 container' corruption, relative link, stray bullets
  **Repair:** Rewritten: the altar is stated as **500,000 aura**, the table is a real table, and the link is absolute.
- **Reviewer:** licence notice
  **Repair:** `LICENSE` + `LICENSE-NOTICE.md` in the `.hnmod`, `data/naturesaura/license/` in the carried `.hmod`.
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

### Repair verification (2026-11-11/12)

- Staged `check`: check exit 0 (`ports/_shared/evidence/repair-2026-11-11/naturesaura-check.log`)
- Embedded tests: embedded **19 passed; 0 failed** (`ports/_shared/evidence/repair-2026-11-11/naturesaura-test.log`, the new bounds test included)
- In-game: e2e **PASS** + `EXIT:0` (`ports/_shared/evidence/repair-2026-11-11/naturesaura-e2e.log`): bloom, altar multiblock, aura accounting
- `cargo test` (standalone lib), `cargo clippy --all-targets -- -D warnings` and
  `cargo fmt --check` are clean in the port; the artifacts were built twice and
  hash identically (`registry.toml` carries the sha256 above).
- Skills: superpowers/using-superpowers,systematic-debugging,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices,rust-port


## Migration to the SDK format (2026-11-18)

Migrated from the retired `.hnmod` container (Horizon's mod-API merge #16941, launcher
≥ 0.2.70; PR #17992 cleans up the old files) to the flat SDK package at nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** / mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78. The registry closed this
port's former submission **#837** on 2026-11-18 with the format purge, so 0.2.0 supersedes
it. The reference recipe was `ports/imfast/notes.md` → "Migration to the SDK format", and
`external/botania/notes.md` for a port with block entities and a quantity; this port is the
same shape as botania plus the chunk attachment.

**Done in this order:** the old layout was moved into
`ports/_shared/evidence/naturesaura/retired-hnmod-0.1.0/` **before** the first
`cargo horizon-mod build` (the tool refuses a root holding retired package paths), then
`src/runtime.rs` was copied with `cp` and proved with `cmp` + `sha256sum`, `cargo test`
ran on the new crate **before any glue existed**, and only then was `src/lib.rs` written.

### File map

| Old `.hnmod` layout | New SDK package |
|---|---|
| `horizon-mixin.toml` (`[mod]`, `[compatibility]`, 2 `[[module]]`, 8 `[[injection]]`) | `horizon-mod.toml` (id/name/version/api/side/capabilities + `[native]` game/horizon/mixins) |
| `Cargo.toml` (`[lib] path = "mixins/naturesaura/runtime.rs"`, `[workspace] members = ["tools/audit"]`) | `Cargo.toml` (`[lib] path = "src/lib.rs"`, one dependency: `horizon-mod-api` from `.horizon/sdk/`) |
| `mixins/naturesaura/runtime.rs` (module + crate lib) | `src/runtime.rs` — **byte-identical**, `pub mod runtime;` at the crate root |
| `mixins/naturesaura/support.rs` (`naturesaura_state` module, 752 lines) | `src/lib.rs` — one `impl Hooks`, three Parts, the attachment and the helpers |
| 8 per-hook helper `.rs` files (`block_entity.rs`, `block_entity_ticks.rs`, `block_entity_container.rs`, `block_entity_tick.rs`, `right_click_block.rs`, `register_commands.rs`, `perform_command.rs`, `embedded_content.rs`) | methods of `impl Hooks`, of the Parts, or a `Registrar` call; no per-hook files |
| `content/` (`assets/`, `data/`, its own `horizon-mod.toml`, `naturesaura.hmod`) | package root: `assets/`, `data/` — the content half of the same `.hmod` |
| `mixins/naturesaura/data/naturesaura-content.hmod` + `embedded_content.rs` (the AE2 write-out) | **gone**: code and content are one package, so there is nothing to embed |
| `compatibility.json`, `rust-toolchain.toml`, `horizon-native.json` | **gone**: `[native]` in `horizon-mod.toml` is the pin, and the launcher's toolchain is the one that builds |
| `tools/horizon.py` (stage/check/test/build), `tools/package.py`/`package.sh` (.hnmod), `tools/audit/` | **gone** (moved to the evidence folder); the SDK ladder is `cargo test` + `cargo horizon-mod build/check/fits` |
| `naturesaura.hnmod` (15 entries, 32,749 B) | `naturesaura.hmod` (62 entries, 53,558 B) |

**Kept and updated:** `tools/assemble_content.py` (now writes `assets/` + `data/` at the
package root), `tools/stamp_license.py` (now writes `data/naturesaura/license/` at the
root), `tools/e2e.sh`, `tools/e2e-all.sh`, `FINDINGS.md`, `notes.md`, `VALIDATION.md`,
`LICENSE`, `LICENSE-NOTICE.md`. **Retired into evidence:** `mixins/`, `naturesaura.hnmod`,
`compatibility.json`, `rust-toolchain.toml`, `Cargo.lock`, `PLAN.md`, the old repo
`AGENTS.md` (it tells the reader to open `compatibility.json` and to run `tools/horizon.py`),
`tools/horizon.py`, `tools/package.py`, `tools/package.sh`, `tools/audit/`, the old
`content/horizon-mod.toml` and `content/naturesaura.hmod`.

### Manifest mapping

- identity: `id = "naturesaura"` (also the resource namespace), `name`, `version = "0.2.0"`
  (the port's version, not upstream's 41.10), `license = "MIT"` in `Cargo.toml`,
  `authors = ["Ellpeck", "AdLf0208"]`.
- `api = "0.2"` (exact), `side = "both"` (an integrated server belongs to the client
  installation, so a LAN host and single player need it), `capabilities = ["registries"]`
  (the schema's one value — this package really does declare blocks and block entities).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and the bundle's mixins revision.
- `[mod] priority = 100` and the plan's ordering are gone; nothing in this slice needed
  ordering (`#[listener(priority = …)]` is the new lever).

### Hook → SDK table

| Old `[[injection]]` / helper | New place | Mapping notes |
|---|---|---|
| `block_entity` (`block_entity.rs`, returned the initial compound) | `BlockEntityPart::block_entity` on `BloomPart` / `AltarEntityPart` | a declared type would start with an empty compound; the Parts answer the same initial tags the hook did (`just_generated = 1`; `aura/timer/recipe/complete`) |
| `block_entity_ticks` (`block_entity_ticks.rs`) | `BlockEntityPart::block_entity_ticks` | `event.ticks = true` |
| `block_entity_tick` (`block_entity_tick.rs`, bindings `game`/`pos`/`entity`) | `BlockEntityPart::block_entity_tick` | `event.pos`, `event.entity.nbt`/`.items` (a checked view; `Deref`s to the fields); a Part that changed nothing answers `BlockEntityTickReply::Unchanged` |
| `block_entity_container` (`block_entity_container.rs`, built a `ModContainer`) | `Registrar::block_container(&altar, ModContainer { … })` in `Hooks::register` | a default-capability declaration, not a hook: one slot, every face reaches it, input and output |
| `right_click_block` (`right_click_block.rs`, `game.held()`, `*canceled = true`) | `BlockPart::use_item_on` + `use_without_item` on `AltarClickPart` (attached with `Registrar::block_part`) | the SDK splits the two hands into two methods; an answer replaces the block's behaviour, which is what `canceled` meant. The old two-asks-per-click fix is preserved: only a **finished product** is taken by hand |
| `register_commands` (`register_commands.rs`) | `Hooks::register_commands` (`RegisterCommandsEvent.commands`) | same six usages |
| `perform_command` (`perform_command.rs`, string words) | `Hooks::run_mod_command` (`CommandInvocation`) | typed arguments: `<pos:block_pos>` → `CommandArgumentValue::BlockPos([i32; 3])`, `<amount:integer>` → `Integer`; `Flow::Cancel` = handled |
| `embedded-content` (`content_root` HEAD) | **none** | the SDK `.hmod` is one package: `assets/`+`data/` are loaded because they are in it |
| the chunk attachment (`mod_api::AttachmentType` + `native_attachments`) | the same `AttachmentType` static, read through `host.attachments()` (`Host::attachments`, `#[access(WorldOnly)]`) and `Holder::Chunk([cx, cz])` | `get_existing` for reads that must not create a save entry, `get`/`set` for a write; the codec is unchanged |
| `game.native_block(pos)` | `level.block(host, pos)` → `BlockKey::id(host)` | `LevelRef` handle methods |
| `game.native_dimension()` (the altar's Overworld/Nether rule) | the level in place compared with `host.level("minecraft:overworld")` / `("minecraft:nether")` | a block-entity tick is not told its level; `LevelRef` is `Eq`, so equality answers it without a dimension string |
| `game.world.game_time` | `level.game_time(host)` | `i64` |
| `game.native_block_entity_sync(pos)` | `host.block_entity_sync(pos)` | after a craft |
| `game.native_with_mod_block_entity` (read/write) | `host.block_entity(level, pos)` → `BlockEntityRef::data` / `merge_data` / `item_slot` / `set_item_slot` | the slot writes go through `host.transaction(…)` so the insert and the player's hand move together |
| `game.held()`, `game.native_take_items`, `game.native_give_stack`, `game.creative` | `event.click.stack`, `ItemPlace::PlayerHand{player, hand}.extract(…)` / `.insert(…)` | `extract` already gives without losing any for a creative player, so the separate creative check is gone |
| `game.pack.assets.registry.default_state(id)` | `host.block_state_id(id)` | `/setblock`-style parsing; the harness builds the frame with default states |
| `game.native_set_block(…)` | `level.set_block(host, pos, state, BlockUpdateFlags::NEIGHBORS \| CLIENTS)` | the old `flags::UPDATE_ALL` is those two bits |
| `NativeCommand::new` / `::op` | `Command::new` / `Command::op` | same usage strings |
| `game.native_feedback(Text::literal(…))` | `host.commands().feedback(Text::literal(…))` | `ServicesServerHost::commands` |

### Build and verify (measured 2026-11-18)

Every command ran in a `ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` (the job id
was `sdk-na`; steps `test`, `lint`, `fmt`, `build`, `check`), **directly in the port** —
`build-slot.sh` speaks the old `tools/horizon.py` and does not by itself know the SDK
ladder, so it is used as the slot guard, not as the driver. `CARGO_INCREMENTAL=0`, no
`--target-dir`.

```
cargo horizon-mod build                        # fetches .horizon/sdk/ first
cargo test                                     # 19 passed; 0 failed
cargo clippy --all-targets -- -D warnings      # exit 0, 0 warnings
cargo fmt --check                              # clean
rm -f Cargo.lock && cargo horizon-mod build    # -> target/horizon-mod/naturesaura.hmod
cargo horizon-mod check naturesaura.hmod       # exit 0
cargo horizon-mod fits naturesaura.hmod --build nightly   # exit 0
```

`check` output:

```
naturesaura 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  53558 bytes, SHA-256 a64a5d8ff02b87b02c79787852e6f7c13f5ac91eee04e560ba03c236cf9c4f3c, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`fits` output:

```
Nature's Aura:
0b14f8a  made for
```

The archive is 62 entries: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`,
`LICENSE-NOTICE.md`, `README.md`, `assets/…` (31), `data/…` (23, the two licence
copies included), `src/lib.rs`, `src/runtime.rs`, `uses.json`. **No `Cargo.lock`**
(trap 5). `unzip -p naturesaura.hmod src/runtime.rs | sha256sum` is
`fa7273e5…a393`, the retired file byte for byte.

`uses.json` is the migration's best summary: **358 surface entries** — the seven hooks
(`block_entity`, `block_entity_ticks`, `block_entity_tick`, `use_item_on`,
`use_without_item`, `register_commands`, `run_mod_command`), `Hooks::register` with
`DefaultCapabilitiesRegistrar::block_container` and `BlockEntityTypeBuilder`, the
`AttachmentType` handles (`get`/`get_existing`/`set`/`remove`), the `BlockEntityRef`
handles (`data`, `merge_data`, `item_slot`, `set_item_slot`, `kind`, `position`,
`level`), `HostTransaction::transaction`, `Commands::feedback`, both macros
(`horizon_mod`, `horizon_part`) and the `registries` capability.

### Traps this migration hit (the next port must know)

1. **`tools/audit/` is a retired package path.** The first build failed with
   `error: unexpected package path: tools/audit/src/main.rs` — the old `Cargo.toml`'s
   `[workspace] members = ["tools/audit"]` left a second crate in the tree. Anything
   that is not a package path (`src/`, `assets/`, `data/`, `horizon-mod.toml`,
   `Cargo.toml`, root files) is refused; move it to evidence **before** the first build.
2. **`ItemPlace` carries the item verbs, not `ServerHost`.** `game_insert`/`game_extract`
   live on the *backend* trait (`ItemsBackendMut`); a mod calls
   `ItemPlace::PlayerHand{player, hand}.extract(host, tx, &resource, n)` and `.insert(…)`.
   `ServerHost` offers neither.
3. **`runtime::altar_complete` takes `impl Fn([i32;3]) -> String`.** The old glue passed
   `game.native_block(at)` (a `String`); `level.block(host, at)` gives a `BlockKey`, so the
   closure answers `block_id(host, level, at).unwrap_or_default()`.
4. **A block-entity tick is not told its level.** Find it with
   `host.levels().list_levels().into_iter().find(|level| level.in_place(host))`, and
   answer "which dimension" by equality with `host.level("minecraft:overworld")` /
   `("minecraft:nether")` (`LevelRef` is `Eq`); do not guess the overworld first.
5. **A fresh declared block entity starts with an empty compound.** Without the
   `block_entity` Part method the altar would read `recipe = 0` (a real recipe!) and the
   bloom would never generate, because `read_int` answers 0 for an absent key. The Parts
   answer the old initial tags, and every read of a key whose `0` is meaningful uses the
   sentinel `UNSET` (`-1`).
6. **A tick may answer `Unchanged`.** The bloom does once it has generated, so a spent
   bloom stops dirtying its chunk; the altar writes every tick (as the 0.1.0 subject did)
   and keeps the default `Changed`.
7. **`Compound` is a `BTreeMap<JavaString, Tag>` and `JavaString` is not exported.**
   Write with `nbt.insert(key.into(), …)` and read with the UTF-16 key comparison helper
   (`key.encode_utf16()` against `name.0`); do not try to name the key type.
8. **`ItemStack::new` is fallible and takes an `ItemKey`.** `ItemStack::of(item)` for one,
   `ItemStack::new(key, count) -> Option<_>`; the key comes from
   `Registrar::item_key(id)` at registration (the recipe table names items by id only, so
   the outputs are resolved once into a `OnceLock`), and an id from a stack is
   `stack.item().id(host)`.
9. **`BlockPart` vs `BlockEntityPart`.** `use_item_on`/`use_without_item` are block hooks;
   attaching the click Part with `Registrar::block_part(&altar, …)` keeps the block table
   unambiguous, while the two entity Parts are attached with
   `Registrar::block_entity_part(&type, …)` to types built by
   `block_entity_type("…").blocks(&["naturesaura:…"]).build()`. A block-entity Part also
   needs an empty `impl BlockPart` beside it (`#[horizon_part]` derives subscriptions, it
   does not implement the supertrait).
10. **The content half moves to the package root.** `assets/` and `data/` are no longer
    under `content/`; `tools/assemble_content.py` now writes them there and no longer
    writes a manifest (the root `horizon-mod.toml` is the only one). Re-running it
    reproduced all 52 carried files byte for byte (`sha256sum` diff empty); only
    `data/naturesaura/license/{license,notice}.txt` are added by `tools/stamp_license.py`,
    which is what the 0.1.0 artifact shipped.
11. **`cargo fmt --check`, not `--all`.** `.horizon/sdk` is a path dependency; `--all`
    reports the SDK's own `build.rs` diffs.
12. **`Cargo.lock` changes the artifact.** `cargo test`/`clippy` write one; remove it right
    before `cargo horizon-mod build` (`/Cargo.lock` is in `.gitignore`). With it the
    package would pack a lock the SDK was not built against.
13. **No `rust-toolchain.toml`.** The launcher and the pinned SDK decide the toolchain
    (`sdk.json` declares rust 1.99.1; this crate was compiled here with rustc 1.97.0,
    edition 2024).
14. **`cargo horizon-mod check` with no file checks the wasm target.** Use `check
    <file.hmod>` for the package and `cargo check`/`cargo test` for the native build.
15. **The 0.1.0 e2e cannot run the new package.** `tools/e2e.sh`/`e2e-all.sh` drive a game
    built by `tools/horizon.py` with the bundle compiled in; the SDK package is built and
    run by `horizon-launcher native-build .`, which is not installed here. The scripts are
    kept unchanged (their assertions are the behaviour's proof) and no 0.2.0 in-game leg is
    claimed; the 0.1.0 runs in `ports/_shared/evidence/naturesaura/` stand as the
    behaviour's evidence for a `runtime.rs` that did not change.

### Gaps (unchanged by the migration)

`ported-partial`, as before: the slice boundary above is the port's scope. The migration
adds no new gap and closes none of the recorded ones (`IAuraType` per dimension, the
per-item container rules, `PacketAuraChunk`/client overlay, waterlogging, the altar's own
recipe). One *new* recorded difference: the `/naturesaura altar` readout names the slot's
item by **id** where the 0.1.0 code printed the item field directly; the SDK exposes
`ItemKey::id(host)`, so the string is the same and no API gap is implied.

`## Upstream defects fixed`: none found in this migration. The 0.1.0 repair (a saved
recipe index past the table panicked the tick) was a port defect, already fixed and
covered by `a_saved_recipe_index_past_the_table_is_dropped_not_indexed`.

### Changelog for the resubmission (supersedes #837)

> Supersedes #837, closed on 2026-11-18 because Horizon retired the `.hnmod` container
> (#17941; PR #17992 cleans up the old files). 0.2.0 is the flat SDK package: `api = "0.2"`,
> `[native]` pinned to nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4` and mixins
> `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.78, for 26.3. The eight old
> injections became seven typed hooks (`block_entity`, `block_entity_ticks`,
> `block_entity_tick`, `use_item_on`, `use_without_item`, `register_commands`,
> `run_mod_command`) plus a `Registrar::block_container` declaration and one typed chunk
> attachment; the content half now travels in the same package instead of an embedded
> `.hmod`; `src/runtime.rs` and its 19 tests are unchanged, byte for byte. Package:
> 62 entries, **53,558 B**, sha256
> `a64a5d8ff02b87b02c79787852e6f7c13f5ac91eee04e560ba03c236cf9c4f3c`; `check` and `fits`
> exit 0. No 0.2.0 in-game run: the launcher is not installed here (trap 15).

### Publication route (campaign ruling 2026-11-18)

The submission form is gone; publishing is now `horizon-rust/horizon-mods#1147` — the artifact
is attached to a release of the author's own repository under the entry's exact `file` name, a
PR adds the `[[versions]]` record and links the file, and a **maintainer** runs
`scripts/stage.sh` (a red `check` before that is the expected `fetch.sh` failure). The
orchestrator's `python3 ports/tools/publish-ready.py naturesaura` does the bookkeeping; this
job owns no submission.

### Migration verification

| Check | Command | Result |
|---|---|---|
| Runtime tests in the new crate, before the glue | `cargo test` | **19 passed**, 0 failed (`evidence/naturesaura/sdk-test.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| Package | `cargo horizon-mod build` | `target/horizon-mod/naturesaura.hmod`, exit 0 (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check naturesaura.hmod` | `naturesaura 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)`, **53,558 B**, sha256 `a64a5d8f…5cf3c`, **exit 0** (`sdk-check.log`) |
| Fits the pinned build | `cargo horizon-mod fits naturesaura.hmod --build nightly` | `0b14f8a made for`, exit 0 (`sdk-fits.log`) |
| Same package twice | `cargo horizon-mod build` again | 53,558 B, `a64a5d8f…5cf3c` both times (`sdk-reproducible.txt`, `sdk-build-second.log`) |
| `runtime.rs` untouched | `unzip -p naturesaura.hmod src/runtime.rs \| sha256sum` | `fa7273e5…a393` = the retired file (`sdk-runtime-sha256.txt`) |
| Content carry unchanged | re-ran `assemble_content.py` + `stamp_license.py` | the 52 carried files byte for byte, plus the two licence copies (`sdk-content-carry.log`) |

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
