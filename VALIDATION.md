# Validation ladder — Nature's Aura slice 1

Pin: Horizon `1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`, horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6). Minecraft 26.3.
Every build/test step ran inside a slot lease (`ports/tools/build-slot.sh`,
which forwarded the compile steps to the second machine); `CARGO_INCREMENTAL=0`;
no `--target-dir` of my own.

## 1. The pure runtime (standalone crate)

```
cargo test --locked --offline          -> 18 passed; 0 failed
cargo clippy --all-targets --locked --offline -- -D warnings -> clean
cargo fmt                              -> applied
```

**Red → green:** the first run failed 3 of 19 tests — the guessed multiblock
counts (12 stone / 12 wood against the artefact's 16 / 20), a generation-cap case
that exceeded the cap, and a ritual test that handed the altar exactly 15,000
aura and could not pay the recipe's `ceil(15000/80) = 188` per tick over 80 ticks
(15,040). Expectations #1 and #3 were corrected **from the artefact** (the
`Multiblocks.ALTAR` rows and the over-spend arithmetic), then all 19 passed; the
cap test and its helper were removed as unreached in this slice (recorded in
`notes.md`).

## 2. The module inside the game

```
python3 tools/horizon.py stage   -> stage_changed: true
python3 tools/horizon.py check   -> cargo_exit 0, 0 warnings
python3 tools/horizon.py test    -> cargo_exit 0, `game::naturesaura::tests`
                                    18 passed; 0 failed; 0 warnings
```

`check` first failed with three real errors (an `i32`/`usize` mix-up in the
readout and two double borrows of `game`), fixed before it passed. A second round
of `check`/`test` after the e2e's click fix stayed at exit 0 / 0 warnings.

## 3. Packages (`tools/package.sh`)

```
content/naturesaura.hmod  21,529 B
                          sha256 07576a9ec727f18297c660e6cf41c4bf39a0553f9598ce91ede3db5b23193781
                          53 entries, `no code: content only`
naturesaura.hnmod         32,749 B
                          sha256 d7237dd6aa551d5ae61674f607ff1c8c2b72d7adaf79f58c26674a26f2ae627b
                          13 entries
```

The `.hnmod` carries `mixins/naturesaura/data/naturesaura-content.hmod`, the
byte-identical copy of the built `.hmod` (`cmp` clean; `tools/package.py` refuses
a stale copy). The `.hmod` rebuilt to the same SHA-256 across two runs.

**Reproducibility caveat, measured:** the `.hnmod`'s *entries* are byte-identical
to the bundle the e2e ran, but the zip container depends on the packager's zlib:
packed on the second (test) machine it is 32,749 B / `d7237dd6…`, packed on this
one 32,785 B / `bb7fa88d…`, with the same 13 names and the same 85,009
uncompressed bytes. The delivered file is the one packed on the machine that ran
the e2e; rebuild on that machine to reproduce its SHA-256. Its mtime was set after
the last source touch because `cargo fmt` rewrote `runtime.rs` *after* that
package run, and the packed `runtime.rs` (27,987 B) was then checked byte for byte
against the formatted source — so the artifact corresponds to the delivered
sources. The `.hmod` (built by `cargo-horizon-mod`, which writes its own
deterministic zip) hashes the same on both machines.

## 4. In-game e2e (three legs, one lease)

`tools/e2e-all.sh` builds once and launches three times; the logs live in
`ports/_shared/evidence/naturesaura/`:

| Log | Leg | Verdict |
|---|---|---|
| `e2e-slice1.log` | `EMBEDDED=0`, `content/` as a dev mod | `PASS` + `EXIT:0` |
| `e2e-embedded.log` | `EMBEDDED=1 CONTROL=1`, only the `.hnmod` | `PASS` + `EXIT:0` |
| `e2e-selftest.log` | `SELFTEST=1`, the conservation assertion broken on purpose | `FAIL` + `EXIT:1` |

```
$ python3 ports/tools/check-e2e-logs.py naturesaura --strict
ok       naturesaura            e2e-embedded.log           PASS + EXIT:0
ok       naturesaura            e2e-selftest.log           FAIL + EXIT:1
ok       naturesaura            e2e-slice1.log             PASS + EXIT:0
logs=3  ok=3  missing-exit=0  bad-exit=0        (exit 0)
```

The default leg's transcript (abridged; the log keeps every line, and the
`[naturesaura] altar click` lines show the hook's view of each click):

```
== A: an untouched chunk reads the base aura ==
  Nature's Aura at 200, 200, 0 (radius 20): area=1000000 base=1000000 spots=0 count=0
  Aura spot at 206, 200, -6: 0
== B: an aura bloom stores exactly 150,000 aura, once ==
  Aura spot at 206, 200, -6: 150000
  Nature's Aura at 200, 200, 0 (radius 20): area=1150000 base=1000000 spots=150000 count=1
  Aura spot at 206, 200, -6: 150000          (60 ticks later: it generates once)
== C: an altar outside a multiblock does nothing ==
  Nature's Altar at 210, 200, 6: complete=0 aura=0/500000 timer=0 recipe=none slot=minecraft:iron_ingot x1 area=1150000
  Nature's Aura at 200, 200, 0 (radius 20): area=1150000 base=1000000 spots=150000 count=1
== D: the complete altar turns an iron ingot into infused iron ==
  Nature's Altar frame around 200, 200, 0: placed 48 blocks; area=1150000
  Nature's Altar at 200, 200, 0: complete=1 aura=6900/500000 timer=0 recipe=none slot=empty area=1143100
  Nature's Altar at 200, 200, 0: complete=1 aura=19008/500000 timer=9 recipe=minecraft:iron_ingot slot=minecraft:iron_ingot x1 area=1129300
  Nature's Altar at 200, 200, 0: complete=1 aura=87560/500000 timer=0 recipe=none slot=naturesaura:infused_iron x1 area=1047400
  area 1047400 + container 87560 = 1134960; the recipe spent 15040 of 1150000
== E: an empty hand takes the infused iron off the altar ==
  Nature's Altar at 200, 200, 0: complete=1 aura=99860/500000 timer=0 recipe=none slot=empty area=1035100
  the infused iron is in the player's inventory
== F: an altar in an area with no aura transmutes nothing ==
  Nature's Aura at 200, 200, 0 (radius 20): area=-168500 base=1000000 spots=-1168500 count=2
  Nature's Altar at 200, 200, 0: complete=1 aura=0/500000 timer=0 recipe=minecraft:iron_ingot slot=minecraft:iron_ingot x1 area=-168500
PASS: … the complete altar turns an iron ingot into infused iron while the area and its container account for exactly the recipe's 15,040 aura
EXIT:0
```

The embedded leg also proves the one-file install: its control run (the same
binary, `.horizon-cache` made unusable) has **no bloom item** and the block stays
`minecraft:air`, while the real run writes
`.horizon-cache/naturesaura-embedded-content.hmod` (**21,529 B**, `cmp`-identical
to `content/naturesaura.hmod`) and every id resolves.

The selftest leg ends `FAIL: area + container (selftest, must fail) is 1134960,
expected 1134961` and `EXIT:1` — the scenario can fail.

## 5. The SDK migration (0.2.0, 2026-11-18)

The retired `.hnmod` ladder above (§1–§4) is history: `tools/horizon.py`, `tools/package.py`
and the staged tree are gone, and the in-game legs need a launcher-built game that is not
installed here (`notes.md`, migration trap 15). What replaced them, measured in this tree:

```
# in a build-slot lease (BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-na …);
# the commands run directly in the port, CARGO_INCREMENTAL=0, no --target-dir
cargo test                                          -> 19 passed; 0 failed
cargo clippy --all-targets -- -D warnings            -> exit 0, 0 warnings
cargo fmt --check                                    -> clean
rm -f Cargo.lock && cargo horizon-mod build          -> target/horizon-mod/naturesaura.hmod
cargo horizon-mod check naturesaura.hmod             -> exit 0
cargo horizon-mod fits naturesaura.hmod --build nightly -> exit 0
```

```
naturesaura 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  53558 bytes, SHA-256 a64a5d8ff02b87b02c79787852e6f7c13f5ac91eee04e560ba03c236cf9c4f3c, unsigned
```

- **One package, 62 entries**, no `Cargo.lock`; `LICENSE` and `LICENSE-NOTICE.md` inside,
  plus the two copies the content half carries under `data/naturesaura/license/`.
- **`runtime.rs` byte-identical**: `unzip -p naturesaura.hmod src/runtime.rs | sha256sum` is
  `fa7273e5d8a238447f93fb43f5ef7df094941e44a78d1dc1c3412d0f6029a393`, the retired
  `mixins/naturesaura/runtime.rs` (`cmp` clean); its 19 tests ran **before** `src/lib.rs`
  existed.
- **Reproducible**: a second `cargo horizon-mod build` produced the same 53,558 B and the
  same SHA-256.
- **The content carry is unchanged**: re-running `tools/assemble_content.py` from the pack
  jar (SHA-1 checked) rewrote the 52 carried files byte for byte, and
  `tools/stamp_license.py` rewrote the two licence copies; `sdk-content-carry.log` is the
  54-line `sha256sum` manifest.
- **Not re-run**: the in-game e2e. `tools/e2e.sh`/`tools/e2e-all.sh` are kept unchanged; the
  0.1.0 logs (§4) remain the behaviour's evidence for a runtime this migration did not
  change.

Skills: superpowers/using-superpowers,systematic-debugging,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## 6. Slice 2 (0.2.2, 2026-11-21): the carried recipes and their dialect

Pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** / mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher 0.2.92. Every compile step ran in a
`ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1`, `CARGO_INCREMENTAL=0`, no
`--target-dir`; the in-game legs ran through `launcher-game.sh build` + a leased
`build-slot.sh run --step e2e`.

**Red first.** `tools/data_shape.py` on the delivered 0.2.1 package exits 1 on
`data/naturesaura/recipe/gold_brick.json` (`{"item": …}` ingredients), and a
launcher-built game carrying the pre-fix tree logs
`recipes: invalid ingredient (unresolved tag or malformed input): {"item":"minecraft:stone_bricks"}`
and crafts no `naturesaura:gold_brick` (`evidence/naturesaura-e2e/e2e-red.log`, EXIT:1).
The shaped `gold_fiber` recipe already crafted, which is why slice 1 missed it.

**Fix.** `tools/assemble_content.py` now upgrades `{"item": …}` and `{"tag": …}` for a
shapeless recipe's `ingredients` list as well as a shaped `key`; the data was
regenerated from the kept jar. `diff -rq` over the before/after carry trees shows
exactly one changed file, `recipe/gold_brick.json` — the other 51 are byte-for-byte
the same. `tools/data_shape.py` and `tools/craft_check.py` are new; `tools/e2e.sh`
gained phases G (the two recipes), H (items/blocks/unknown ids) and I (loot drops
with a vanilla differential).

```
cargo test                                          -> 21 passed; 0 failed
cargo clippy --all-targets -- -D warnings            -> exit 0, 0 warnings
cargo fmt --check                                    -> clean
rm -f Cargo.lock && cargo horizon-mod build          -> target/horizon-mod/naturesaura.hmod
rm -f Cargo.lock && cargo horizon-mod build          -> cmp-identical
cargo horizon-mod check naturesaura.hmod             -> exit 0
cargo horizon-mod fits naturesaura.hmod --build 0a875da -> 0a875da  made for
cargo horizon-mod fits naturesaura.hmod --build 0b14f8a -> 0b14f8a  fits
python3 ports/tools/verify-sdk-package.py naturesaura -> 1/1
```

```
naturesaura 0.2.2 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  54908 bytes, SHA-256 be31234c0485c8873071b0a4ff2f1568dc23bbc3a35a96a1140fe61d8d978143, unsigned
```

- **62 entries**, no `Cargo.lock`; `src/runtime.rs` `0a11dfcc…` in tree and package
  (unchanged this slice).
- **In-game:** `PASS` + `EXIT:0` (`evidence/naturesaura-e2e/e2e-launcher.log`), the
  control is `FAIL` + `EXIT:1` (`e2e-launcher-control.log`, unknown command and
  unknown block without the package), the selftest is `FAIL` + `EXIT:1`
  (`e2e-selftest.log`), and `check-e2e-logs.py naturesaura-e2e --strict` is 4/4.
- **Dialect:** `data_shape.py` is green over `data/` and over the built package
  (`evidence/naturesaura/data-shape-green.log`).

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## 7. Reviewer follow-up #1530 (0.2.3, 2026-11-24): copyright line + attribution

The reviewer refused the first credit line: upstream's `LICENSE` at tag `41.10` (commit
`70b93d3…`) reads, verbatim, `Copyright (c) 2021 Ellpeck`. That exact line is now `LICENSE`
line 3, with `Copyright (c) 2026 AdLf0208 (Horizon port)` on line 4 (industrialforegoing 0.2.13
precedent); `LICENSE-NOTICE.md` quotes the same line and puts the port credit in a separate
block below. `stamp_license.py` regenerated the two packed copies. The published 0.2.2 record
is untouched — the fix is a new patch version.

Pin `0a875da…` / mixins `e59da90…` unchanged; `src/runtime.rs` unchanged (`0a11dfcc…`).
Every compile step ran in a `build-slot.sh` lease **forwarded to AdLf-PC** (no `BUILD_LOCAL=1`);
the in-game legs ran locally through `launcher-game.sh` + a leased `--step e2e`.

```
rm -f Cargo.lock && cargo horizon-mod build     # twice, cmp-identical, forwarded
cargo horizon-mod check naturesaura.hmod        # 0.2.3, exit 0
cargo horizon-mod fits … --build 0a875da        # 0a875da  made for
cargo horizon-mod fits … --build 0b14f8a        # 0b14f8a  fits
cargo test                                      # 21 passed; 0 failed (after the build)
cargo clippy --all-targets -- -D warnings         # exit 0, 0 warnings
cargo fmt --check                                # clean
python3 ports/tools/verify-sdk-package.py naturesaura  # 1/1, "package matches the tree"
python3 ports/tools/check-e2e-logs.py naturesaura-e2e --strict  # logs=4 ok=4
```

```
naturesaura 0.2.3 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  55145 bytes, SHA-256 4687233c2c7d2af296a0cfd775300262421ff2a2e9165d6526af43de81529c7f, unsigned
```

- **62 entries**, no packed `Cargo.lock`; both licence texts and both content copies are the
  corrected bytes, and `verify-sdk-package.py` confirms the whole archive matches the tree.
- **Fits:** `0a875da made for`, `0b14f8a fits` — no local `Block`/`Item` type or variant
  introduced, so no false failure from that trap.
- **In-game:** the launcher's new native tag `native-0a875da-9a0ab846863a` was exported and its
  `LICENSE` and manifest verified before the legs. Default `PASS` + `EXIT:0`; control and
  `SELFTEST=1` both `FAIL` + `EXIT:1` on the expected assertions; strict log gate 4/4. All of
  slice 1's aura/bloom/altar/recipes/items/blocks/loot assertions stayed green.
- **Traps observed:** no stale `native-*` tag carrying naturesaura was found; game root used
  `E2E_TMP`/`SOCK` from `launcher-game.sh env`, not `/tmp`.

Skills: superpowers/using-superpowers,writing-plans,verification-before-completion; BeautifulRust/rust-best-practices

## 8. Slice 3 (0.2.4, 2026-11-24): the grass-death drain-spot effect

Pin `0a875da…` / mixins `e59da90…` unchanged. Every Rust step ran in a
`build-slot.sh` lease **forwarded to `AdLf-PC`** (no `BUILD_LOCAL=1`); the game
legs stay local through `launcher-game.sh build` + a leased `--step e2e`.

**Red first** (`ports/_shared/evidence/naturesaura/effect-red.log`): with the
rule stubbed to `None`, three new tests failed. Verbatim:

```
---- runtime::tests::grass_die_fires_only_at_a_debt_whose_area_is_negative stdout ----
thread 'runtime::tests::grass_die_fires_only_at_a_debt_whose_area_is_negative' (863269) panicked at src/runtime.rs:893:9:
assertion `left == right` failed
  left: None
 right: Some(GrassDie { amount: 15, dist: 20 })

test result: FAILED. 21 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
```

Then the rule, the deterministic targets and the glue were written:

```
cargo test                                          -> 25 passed; 0 failed
cargo clippy --all-targets -- -D warnings            -> exit 0, 0 warnings
cargo fmt --check                                    -> clean (applied, re-checked)
```

Clippy first refused two things, both fixed rather than silenced: a manual
`tick % 20 == 0` (now `tick.is_multiple_of(EFFECT_TICKS)`) and a `(z - 0).abs()`
identity in a test. No `Block`/`Item` local type or variant was introduced, so
`fits` cannot false-fail on R9. The exact upstream copyright line
`Copyright (c) 2021 Ellpeck` was verified first in `LICENSE`,
`LICENSE-NOTICE.md` and both packed content copies, and is unchanged.

Final build and gates (`sdk-*.log` regenerated for 0.2.4):

```
rm -f Cargo.lock && cargo horizon-mod build     # twice, cmp-identical; forwarded
cargo horizon-mod check naturesaura.hmod        # 0.2.4, exit 0
cargo horizon-mod fits … --build 0a875da        # 0a875da  made for
cargo horizon-mod fits … --build 0b14f8a        # 0b14f8a  fits
python3 ports/tools/verify-sdk-package.py naturesaura  # 1/1, "package matches the tree"
cargo test                                      # 25 passed, 0 failed (AFTER the final build)
cargo clippy --all-targets -- -D warnings        # exit 0, 0 warnings
cargo fmt --check                                # clean
```

```
naturesaura 0.2.4 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  59131 bytes, SHA-256 4e70a92adf6932e72f0de90393dd9fedd2019b663fa4d0e91140090c7522b5c9, unsigned
```

- **62 entries**, no packed `Cargo.lock`; `LICENSE`, `LICENSE-NOTICE.md` and both
  content copies carry the exact upstream line first. `verify-sdk-package.py`
  confirms every packed file is the tree's bytes.
- **Manifest bump proven:** the warmed slot logged `Compiling naturesaura v0.2.4`
  and the launcher build logged `Compiling naturesaura (1 of 1)`.
- **Fits:** `0a875da made for`, `0b14f8a fits`. No local `Block`/`Item` type or
  variant introduced, so no false failure from that trap.
- **Stale tag:** the old `native-0a875da-9a0ab846863a` carried 0.2.3; it was
  removed before the build. The new tag is `native-0a875da-0fb4e08164b6`,
  exported under `E2E_TMP` (never `/tmp` for the game root).
- **In-game:** default `PASS` + `EXIT:0`, control `FAIL` + `EXIT:1`, `SELFTEST=1`
  `FAIL` + `EXIT:1`, `SELFTEST=effect` `FAIL` + `EXIT:1`. Phase J's transcript:

```
== J: a drained spot kills grass; an undrained spot under grass does not ==
  Aura spot at 150, 200, 40: 0
  Aura spot at 150, 200, 40: -2500000
  Nature's Aura at 150, 200, 40 (radius 50): area=-1500000 base=1000000 spots=-2500000 count=1
  Aura spot at 150, 200, 40: -2500000
  spot -2500000, area -1500000: the drained patch became coarse dirt while the undrained patch stayed grass
```

  The inverted leg fails with
  `FAIL: the drained patch (selftest: must fail) is minecraft:coarse_dirt, expected minecraft:grass_block`.
  All of slice 1's aura/bloom/altar, slice 2's recipes/items/blocks/loot, and the
  three existing control/selftest legs stayed green.
- **Log gate:** `check-e2e-logs.py naturesaura-e2e --strict` → logs=5, ok=5,
  missing-exit=0, bad-exit=0.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
