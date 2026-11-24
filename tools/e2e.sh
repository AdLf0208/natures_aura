#!/usr/bin/env bash
# In-game check for Nature's Aura: the slice-1 aura loop **and** the two carried
# crafting recipes.
#
#  A-F  an aura bloom stores 150,000 aura in its chunk's drain spots, and a
#       nature's altar inside its multiblock pulls that aura into its own
#       container and turns an iron ingot into infused iron for exactly the
#       recipe's cost — while an altar outside a multiblock and an altar in an
#       area with no aura do nothing.
#  G    both carried recipes craft in a real crafting table: their declared id
#       and count, the grid emptied, the distinctive ingredient consumed, and two
#       wrong-ingredient controls that craft nothing. The server's load log is
#       asserted clean of `recipes: invalid ingredient`, because a recipe in the
#       1.21.1 `{"item": …}` dialect parses to nothing and crafts nothing.
#  H    every carried item is give-able and reads back, every carried block
#       places and reads back, and an unknown id is refused.
#  I    every carried loot table drops its block when broken, with a vanilla
#       block as the harness's differential.
#  J    a drained spot (negative amount, negative radius-50 area) turns grass
#       into coarse dirt on the next second, while an undrained spot (>= 0)
#       under the same grass leaves it alone. SELFTEST=effect inverts the
#       drained patch's assertion and must FAIL.
#  K    the same effect's leaf branch: `LeavesBlock -> naturesaura:decayed_leaves`
#       at a drained spot, while oak leaves at a spot >= 0 stay oak leaves.
#       SELFTEST=effect-leaf inverts the drained assertion and must FAIL.
#
# The game is a launcher-built client that carries the flat `.hmod` (the SDK
# format, 2026-11-18): no `--dev-mod`, because `assets/` and `data/` travel in
# the same package as the code. The run is `--offscreen` (not `--headless`):
# `click-slot`, which phase G needs, requires a renderer.
#
#   eval "$(ports/tools/launcher-game.sh env naturesaura)"
#   ports/tools/build-slot.sh run --job e2e-naturesaura --port ports/naturesaura \
#       --step e2e -- bash ports/naturesaura/tools/e2e.sh
#
# CONTROL=1 first launches the unmodded client of the same build (CONTROL_GAME)
# and asserts the aura command and the bloom block are unknown there, so the
# content demonstrably comes from this package; the same run then fails in
# phase A (no `/naturesaura` command) and ends EXIT:1. SELFTEST=1 breaks the
# conservation assertion on purpose (it must FAIL), proving the check can fail.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The arena is the Overworld, high above the terrain at x=200, z=0: a dirt floor
# at y=199 with clear air above it. The altar stands at (200, 200, 0), which is
# where the multiblock's pattern puts it; the bloom is planted at (206, 200, -6),
# six blocks away and inside the altar's 20-block aura radius. The crafting
# table stands at (196, 200, 8), on the same floor and outside the frame.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${NA_DEBUG:-}" ] || set -x

CONTROL=${CONTROL:-0}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/naturesaura
NEW_PIN_BIN=$CLONE/ports/_shared/bin/horizon-0a875da
CTL=${CTL:-$NEW_PIN_BIN/horizon-ctl}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
HMOD=${HMOD:-$PORT/naturesaura.hmod}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/naturesaura-e2e.sock}
EVIDENCE=$CLONE/ports/_shared/evidence/naturesaura-e2e
WORLD=NatureAuraE2E
CONTROL_WORLD=NatureAuraControl

# The altar's cell and the arena around it.
AX=200
AY=200
AZ=0
# The bloom's cell (dirt below it, inside the altar's radius).
BX=206
BY=200
BZ=-6
# An altar with no multiblock around it, still inside the bloom's radius.
CX=210
CY=200
CZ=6
# The crafting table: on the same floor, clear of the altar's 9x4x9 frame.
TX=196
TY=200
TZ=8
# The grass-death patch: 64 blocks from the altar, outside its 50-block area
# radius, so this spot alone decides the effect's threshold. The control patch
# is another 100 blocks away, outside the drained spot's 20-block scatter.
GX=150
GY=200
GZ=40
KX=250
KY=200
KZ=40
# The leaf branch's patch: 90 blocks from the grass spot (outside its radius-50
# area) and within the loaded chunk window around the player. Its control is
# 100 blocks away, outside the drained spot's 20-block scatter.
LX=240
LY=200
LZ=40
MX=240
MY=200
MZ=-60
# The chunk base every area read starts from, and what the bloom adds.
DEFAULT_AURA=1000000
BLOOM_AURA=150000
# `floor(axe)`: ceil(7500/40) = ceil(15000/80) = 188 aura a tick, 80 ticks.
RECIPE_TICKS=80
AURA_PER_TICK=188
RECIPE_SPEND=$((AURA_PER_TICK * RECIPE_TICKS))
TOTAL_AURA=$((DEFAULT_AURA + BLOOM_AURA))
AREA_AFTER=$((TOTAL_AURA - RECIPE_SPEND))

[ -x "$CTL" ] || {
    echo "missing horizon-ctl: $CTL (eval \"\$(ports/tools/launcher-game.sh env naturesaura)\")" >&2
    exit 1
}
[ -n "$GAME" ] || {
    echo "missing GAME: point it at a launcher-built horizon-client that carries naturesaura" >&2
    echo "  ports/tools/launcher-game.sh build naturesaura" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/naturesaura-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() {
    echo "FAIL: $*" >&2
    ctl logs --lines 80 >&2 || true
    # The startup log is where the server says why a recipe did not load; a run
    # that fails keeps it beside the transcript.
    mkdir -p "$EVIDENCE"
    cp "$HORIZON_HOME/logs/latest.log" "$EVIDENCE/game-latest.log" 2>/dev/null || true
    exit 1
}
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-${WORLD_NAME:-$WORLD}}
    local game=${LAUNCH_GAME:-$GAME}
    rm -f "$SOCK"
    cd "$SCRATCH"
    echo "  launch: $(basename "$game"), no --dev-mod: the package carries its content" >&2
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "$world" --seed 20261105 --creative --offscreen --wait \
        --resources "$RES" >/dev/null
    ctl wait-ready 120 >/dev/null
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }

# The arena: the player first (so the chunks load), then the floor, then clear air,
# then no ambient mobs. Standing on the floor keeps the player from falling.
arena() {
    ctl command "execute in minecraft:overworld run tp @s $AX.5 $AY $AZ.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$AX" $((AY - 1)) "$AZ" | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the arena chunk never loaded"
    ctl command "execute in minecraft:overworld run fill $((AX - 10)) $((AY - 1)) $((AZ - 10)) $((AX + 12)) $((AY - 1)) $((AZ + 10)) minecraft:dirt" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:overworld run fill $((AX - 10)) $AY $((AZ - 10)) $((AX + 12)) $((AY + 8)) $((AZ + 10)) minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:overworld run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:overworld run tp @s $AX.5 $AY $AZ.5" >/dev/null
    ctl wait 20
    [ "$(block_name "$AX" $((AY - 1)) "$AZ")" = "minecraft:dirt" ] || fail "the arena floor is not dirt"
    [ "$(block_name "$AX" "$AY" "$AZ")" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

# `/naturesaura aura X Y Z R`: the line, then AREA, BASE, SPOTS, COUNT.
aura() { # aura X Y Z [R]
    local radius=${4:-20}
    say "naturesaura aura $1 $2 $3 $radius"
    ctl wait 8 >/dev/null
    local line
    line=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 "Nature's Aura at" || true)
    [ -n "$line" ] || fail "no /naturesaura aura reply"
    AREA=$(sed -n 's/.* area=\(-\{0,1\}[0-9]\{1,\}\).*/\1/p' <<<"$line")
    BASE=$(sed -n 's/.* base=\(-\{0,1\}[0-9]\{1,\}\).*/\1/p' <<<"$line")
    SPOTS=$(sed -n 's/.* spots=\(-\{0,1\}[0-9]\{1,\}\).*/\1/p' <<<"$line")
    COUNT=$(sed -n 's/.* count=\([0-9]\{1,\}\).*/\1/p' <<<"$line")
    [ -n "$AREA" ] && [ -n "$SPOTS" ] && [ -n "$COUNT" ] || fail "the aura line did not parse: $line"
    echo "  $line" >&2
}

# `/naturesaura spot X Y Z`: SPOT.
spot() { # spot X Y Z
    say "naturesaura spot $1 $2 $3"
    ctl wait 8 >/dev/null
    local line
    line=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 'Aura spot at' || true)
    [ -n "$line" ] || fail "no /naturesaura spot reply"
    SPOT=$(sed -n 's/.*: \(-\{0,1\}[0-9]\{1,\}\)$/\1/p' <<<"$line")
    [ -n "$SPOT" ] || fail "the spot line did not parse: $line"
    echo "  $line" >&2
}

# `/naturesaura altar X Y Z`: COMPLETE, ALTAR_AURA, TIMER, RECIPE, SLOT, AREA.
altar() { # altar X Y Z
    say "naturesaura altar $1 $2 $3"
    ctl wait 8 >/dev/null
    local line
    line=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 "Nature's Altar at" || true)
    [ -n "$line" ] || fail "no /naturesaura altar reply"
    COMPLETE=$(sed -n 's/.* complete=\([0-9]\{1,\}\).*/\1/p' <<<"$line")
    ALTAR_AURA=$(sed -n 's/.* aura=\([0-9]\{1,\}\)\/[0-9]\{1,\}.*/\1/p' <<<"$line")
    TIMER=$(sed -n 's/.* timer=\([0-9]\{1,\}\).*/\1/p' <<<"$line")
    RECIPE=$(sed -n 's/.* recipe=\([^ ]*\).*/\1/p' <<<"$line")
    SLOT=$(sed -n 's/.* slot=\(.*\) area=.*/\1/p' <<<"$line")
    AREA=$(sed -n 's/.* area=\(-\{0,1\}[0-9]\{1,\}\).*/\1/p' <<<"$line")
    [ -n "$COMPLETE" ] && [ -n "$ALTAR_AURA" ] && [ -n "$SLOT" ] && [ -n "$AREA" ] \
        || fail "the altar line did not parse: $line"
    echo "  $line" >&2
}

# Hold `count` of an item in hotbar slot 1 and select it.
hold() { # hold ITEM COUNT
    ctl command "item replace entity @s hotbar.0 with $1 $2" >/dev/null
    ctl wait 4 >/dev/null
    ctl select 1 >/dev/null
    ctl wait 2 >/dev/null
}

# Right-click a block with what is held (or with an empty hand). The click aims
# at the block itself: the altar is 12/16 high, so aiming at its "up" face sends
# the ray over it, and `ctl state`'s `target` stays null until the click's own
# aim runs, so the click is what proves the aim.
click() { # click X Y Z
    local x=$1 y=$2 z=$3
    ctl command "execute in minecraft:overworld run tp @s $((x - 2)).5 $y $z.5" >/dev/null
    ctl wait 6 >/dev/null
    ctl look-at-block "$x" "$y" "$z" >/dev/null
    ctl wait 3 >/dev/null
    ctl interact "$x" "$y" "$z" >/dev/null || fail "ctl interact at $x,$y,$z failed"
    ctl wait 6 >/dev/null
}

expect() { # expect NAME GOT WANT
    [ "$2" = "$3" ] || fail "$1 is $2, expected $3"
}

# Break one block while standing two blocks away, and return the reply. The
# caller asserts the drop; a block whose loot table is dead reports `drops: {}`.
break_block() { # break_block X Y Z
    local x=$1 y=$2 z=$3
    ctl command "execute in minecraft:overworld run tp @s $((x - 2)).5 $y $((z)).5" >/dev/null
    ctl wait 6 >/dev/null
    ctl look-at-block "$x" "$y" "$z" >/dev/null 2>&1 || true
    ctl wait 3 >/dev/null
    ctl break "$x" "$y" "$z" || fail "cannot break the block at $x,$y,$z"
}

# The negative control: an unmodded client of the same build. The aura command
# and the bloom block must be unknown there, so the content demonstrably comes
# from this package and not from anything else on disk. The run then continues
# into phase A, which fails without the command, so the leg ends EXIT:1.
control_leg() {
    if [ -z "$CONTROL_GAME" ]; then
        echo "== control: skipped (set CONTROL_GAME=<unmodded client> to run it) =="
        return
    fi
    [ -x "$CONTROL_GAME" ] || fail "missing CONTROL_GAME: $CONTROL_GAME"
    echo "== control: the same build with no naturesaura =="
    LAUNCH_GAME=$CONTROL_GAME WORLD_NAME=$CONTROL_WORLD launch
    arena
    local reply chat
    # A player-chat command's answer arrives as a server message a tick later,
    # not in the `ctl chat` reply (that only says the chat was sent).
    ctl chat "/naturesaura aura $AX $AY $AZ 20" >/dev/null
    ctl wait 8 >/dev/null
    chat=$(ctl messages 8 | jq -r '.chat[]' | tr '\n' ' ')
    echo "  /naturesaura aura chat: $chat"
    case "$chat" in
        *"Unknown"*|*"unknown"*|*"incomplete"*) ;;
        *) fail "the control game answered a /naturesaura command: $chat" ;;
    esac
    case "$chat" in
        *"Nature's Aura at"*) fail "the control game computed aura: $chat" ;;
    esac
    reply=$(ctl command "execute in minecraft:overworld run setblock $BX $BY $BZ naturesaura:aura_bloom")
    echo "  setblock naturesaura:aura_bloom reply: $reply"
    case "$reply" in
        *"Unknown"*|*"unknown"*|*"error"*) ;;
        *) fail "the control game knew naturesaura:aura_bloom: $reply" ;;
    esac
    [ "$(block_name "$BX" "$BY" "$BZ")" = "minecraft:air" ] \
        || fail "the control game holds a bloom, so the block id was known"
    echo "  without the package: no /naturesaura command, the bloom block stayed minecraft:air"
    ctl quit >/dev/null 2>&1 || true
}

if [ "$CONTROL" = 1 ]; then control_leg; fi
echo "== launch (Overworld arena; one package, code and content together) =="
launch
arena

echo "== A: an untouched chunk reads the base aura =="
aura "$AX" "$AY" "$AZ" 20
expect "the area's base aura" "$BASE" "$DEFAULT_AURA"
expect "the area's spot sum" "$SPOTS" 0
expect "the area's spot count" "$COUNT" 0
spot "$BX" "$BY" "$BZ"
expect "the bloom's spot before the bloom" "$SPOT" 0

echo "== B: an aura bloom stores exactly 150,000 aura, once =="
ctl command "execute in minecraft:overworld run setblock $BX $BY $BZ naturesaura:aura_bloom" >/dev/null
ctl wait 20 >/dev/null
[ "$(block_name "$BX" "$BY" "$BZ")" = "naturesaura:aura_bloom" ] || fail "the bloom did not place"
spot "$BX" "$BY" "$BZ"
expect "the bloom's spot" "$SPOT" "$BLOOM_AURA"
aura "$AX" "$AY" "$AZ" 20
expect "the area's aura with a bloom" "$AREA" "$TOTAL_AURA"
expect "the area's spot sum with a bloom" "$SPOTS" "$BLOOM_AURA"
expect "the area's spot count with a bloom" "$COUNT" 1
ctl wait 60 >/dev/null
spot "$BX" "$BY" "$BZ"
expect "the bloom's spot after 60 more ticks (it generates once)" "$SPOT" "$BLOOM_AURA"

echo "== C: an altar outside a multiblock does nothing =="
ctl command "execute in minecraft:overworld run setblock $CX $CY $CZ naturesaura:nature_altar" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$CX" "$CY" "$CZ")" = "naturesaura:nature_altar" ] || fail "the lone altar did not place"
hold minecraft:iron_ingot 2
click "$CX" "$CY" "$CZ"
altar "$CX" "$CY" "$CZ"
expect "the lone altar's completeness" "$COMPLETE" 0
expect "the lone altar's slot" "$SLOT" "minecraft:iron_ingot x1"
ctl wait 200 >/dev/null
altar "$CX" "$CY" "$CZ"
expect "the lone altar's completeness after 200 ticks" "$COMPLETE" 0
expect "the lone altar's container after 200 ticks" "$ALTAR_AURA" 0
expect "the lone altar's slot after 200 ticks" "$SLOT" "minecraft:iron_ingot x1"
aura "$AX" "$AY" "$AZ" 20
expect "the area's aura with an incomplete altar" "$AREA" "$TOTAL_AURA"
echo "  the altar outside its multiblock took no aura and transmuted nothing"

echo "== D: the complete altar turns an iron ingot into infused iron =="
say "naturesaura altar-build $AX $AY $AZ"
ctl wait 12 >/dev/null
BUILT=$(ctl messages 12 | jq -r '.chat[]' | grep -m1 "frame around" || true)
[ -n "$BUILT" ] || fail "no /naturesaura altar-build reply"
echo "  $BUILT"
case "$BUILT" in
    *"placed 48 blocks"*) ;;
    *) fail "the frame was not built completely: $BUILT" ;;
esac
# Four positions of the jar's pattern, named absolutely: the top gold corners
# (offset ±4, +2, 0), the stone bricks under one (0, +1, -4) and the bottom ring
# (offset -1, -1, -3 planks; -1, -1, -1 fancy bricks).
[ "$(block_name $((AX + 4)) $((AY + 2)) "$AZ")" = "naturesaura:gold_brick" ] || fail "no gold brick at the top corner"
[ "$(block_name $((AX - 4)) $((AY + 2)) "$AZ")" = "naturesaura:gold_brick" ] || fail "no gold brick at the other top corner"
[ "$(block_name "$AX" $((AY + 1)) $((AZ - 4)) )" = "minecraft:stone_bricks" ] || fail "no stone bricks above the bottom layer"
[ "$(block_name $((AX - 1)) $((AY - 1)) $((AZ - 3)) )" = "minecraft:oak_planks" ] || fail "no planks in the bottom ring"
[ "$(block_name $((AX - 1)) $((AY - 1)) $((AZ - 1)) )" = "minecraft:red_nether_bricks" ] || fail "no fancy bricks in the bottom ring"
ctl command "execute in minecraft:overworld run setblock $AX $AY $AZ naturesaura:nature_altar" >/dev/null
ctl wait 20 >/dev/null
altar "$AX" "$AY" "$AZ"
expect "the altar's completeness" "$COMPLETE" 1
# A complete altar pulls from its first tick, so its container is already
# filling; what must be empty is the slot.
expect "the altar's slot before the ritual" "$SLOT" empty
hold minecraft:iron_ingot 2
click "$AX" "$AY" "$AZ"
altar "$AX" "$AY" "$AZ"
expect "the altar's slot" "$SLOT" "minecraft:iron_ingot x1"
echo "  the altar holds $SLOT with $ALTAR_AURA aura, area $AREA"
ctl wait 260 >/dev/null
altar "$AX" "$AY" "$AZ"
expect "the altar's slot after the ritual" "$SLOT" "naturesaura:infused_iron x1"
expect "the altar's recipe after the ritual" "$RECIPE" none
expect "the altar's timer after the ritual" "$TIMER" 0
# Conservation over the whole window: the area lost exactly the recipe's aura,
# and everything else it lost is in the altar's own container.
if [ "$SELFTEST" = 1 ]; then
    echo "(selftest: asserting the area and container add up to $((AREA_AFTER + 1)))"
    expect "area + container (selftest, must fail)" $((AREA + ALTAR_AURA)) $((AREA_AFTER + 1))
else
    expect "area + container" $((AREA + ALTAR_AURA)) "$AREA_AFTER"
fi
echo "  area $AREA + container $ALTAR_AURA = $((AREA + ALTAR_AURA)); the recipe spent $RECIPE_SPEND of $TOTAL_AURA"

echo "== E: an empty hand takes the infused iron off the altar =="
# Slot 2 is empty, so the main hand is empty after selecting it.
ctl select 2 >/dev/null
ctl wait 4 >/dev/null
click "$AX" "$AY" "$AZ"
altar "$AX" "$AY" "$AZ"
expect "the altar's slot after taking" "$SLOT" empty
ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "naturesaura:infused_iron")] | length >= 1' >/dev/null \
    || fail "the infused iron did not reach the player's inventory"
echo "  the infused iron is in the player's inventory"

echo "== F: an altar in an area with no aura transmutes nothing =="
say "naturesaura drain $AX $AY $AZ 1200000"
ctl wait 10 >/dev/null
aura "$AX" "$AY" "$AZ" 20
[ "$AREA" -lt 0 ] || fail "the area still holds $AREA aura, expected a deficit"
say "naturesaura altar-aura $AX $AY $AZ 0"
ctl wait 10 >/dev/null
hold minecraft:iron_ingot 2
click "$AX" "$AY" "$AZ"
altar "$AX" "$AY" "$AZ"
expect "the altar's slot in an empty area" "$SLOT" "minecraft:iron_ingot x1"
container_before=$ALTAR_AURA
ctl wait 200 >/dev/null
altar "$AX" "$AY" "$AZ"
expect "the altar's container in an area with no aura" "$ALTAR_AURA" "$container_before"
expect "the altar's slot in an area with no aura" "$SLOT" "minecraft:iron_ingot x1"
echo "  with area $AREA the altar took no aura and transmuted nothing"

echo "== G: the two carried recipes craft in a real crafting table =="
# A carried recipe whose ingredient is in the 1.21.1 object form
# (`{"item": "…"}`) is rejected by the 26.3 parser at load, so the recipe is dead
# data. The server names it once, at startup: print those lines as the
# explanation, then let the crafting table be the assertion (and check the log
# again afterwards, so a rejected recipe is a failure even if no craft below
# happens to cover it).
rejected=$(grep -a "recipes: invalid ingredient" "$HORIZON_HOME/logs/latest.log" || true)
if [ -n "$rejected" ]; then
    echo "  the server rejected these ingredients at load:" >&2
    echo "$rejected" | sed 's/^/  /' >&2
fi
# Survival, so a craft consumes its ingredients; the player stands on the floor
# the arena laid and the table is a fresh block entity every time.
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" "$TX" "$TY" "$TZ" \
    || fail "the crafting-table recipe check failed"
ctl close >/dev/null 2>&1 || true
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3
[ -z "$rejected" ] || fail "the server rejected a carried recipe ingredient (see the lines above)"

echo "== H: every carried item is give-able, every block places and drops, unknown ids are refused =="
for id in naturesaura:aura_bloom naturesaura:nature_altar naturesaura:gold_brick \
          naturesaura:infused_stone naturesaura:decayed_leaves naturesaura:infused_iron \
          naturesaura:tainted_gold naturesaura:gold_fiber; do
    ctl command "clear @s" >/dev/null
    ctl give "$id" 1 >/dev/null || fail "give $id failed"
    ctl wait 3
    held=$(ctl inventory | jq -r --arg i "$id" '.counts[$i] // 0')
    [ "$held" -ge 1 ] || fail "$id is not in the inventory after give ($held)"
done
echo "  all eight carried items read back from the inventory"
# Five blocks on the arena floor, each read back after placement.
ctl command "execute in minecraft:overworld run setblock $((AX - 6)) $AY $((AZ + 8)) naturesaura:aura_bloom" >/dev/null
ctl command "execute in minecraft:overworld run setblock $((AX - 4)) $AY $((AZ + 8)) naturesaura:infused_stone" >/dev/null
ctl command "execute in minecraft:overworld run setblock $((AX - 2)) $AY $((AZ + 8)) naturesaura:gold_brick" >/dev/null
ctl command "execute in minecraft:overworld run setblock $AX $AY $((AZ + 8)) naturesaura:nature_altar" >/dev/null
ctl command "execute in minecraft:overworld run setblock $((AX + 2)) $AY $((AZ + 8)) naturesaura:decayed_leaves" >/dev/null
ctl wait 6
for pair in "naturesaura:aura_bloom $((AX - 6))" "naturesaura:infused_stone $((AX - 4))" \
            "naturesaura:gold_brick $((AX - 2))" "naturesaura:nature_altar $AX" \
            "naturesaura:decayed_leaves $((AX + 2))"; do
    set -- $pair
    [ "$(block_name "$2" "$AY" $((AZ + 8)))" = "$1" ] || fail "$1 did not place"
done
echo "  aura_bloom, infused_stone, gold_brick, nature_altar and decayed_leaves each placed and read back"
unknown=$(ctl command "give @s naturesaura:not_an_item 1" | jq -r '.chat[]?' | tr '\n' ' ')
echo "  give naturesaura:not_an_item: $unknown"
case "$unknown" in
    *"Unknown item"*|*"unknown item"*|*"not found"*) ;;
    *) fail "an unknown naturesaura item was accepted: $unknown" ;;
esac
unknown=$(ctl command "execute in minecraft:overworld run setblock $((AX - 6)) $AY $((AZ + 9)) naturesaura:not_a_block" | jq -r '.chat[]?' | tr '\n' ' ')
echo "  setblock naturesaura:not_a_block: $unknown"
case "$unknown" in
    *"Unknown"*|*"unknown"*|*"error"*) ;;
    *) fail "an unknown naturesaura block was accepted: $unknown" ;;
esac

echo "== I: every carried loot table drops its block, with a vanilla block as the differential =="
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe 1" >/dev/null
ctl wait 3
ctl select 1 >/dev/null
# The vanilla block first: a break that reports no drop at all would make the
# mod blocks' assertions meaningless, so the harness proves itself on a block
# whose loot table ships with the game.
ctl command "execute in minecraft:overworld run setblock $((AX - 8)) $AY $((AZ + 8)) minecraft:stone" >/dev/null
ctl wait 4
reply=$(break_block $((AX - 8)) "$AY" $((AZ + 8)))
echo "  vanilla stone: $reply"
# Vanilla stone's loot table yields cobblestone, not stone: the assertion is that
# the break path reports *a* drop at all, so the differential proves the harness.
got=$(jq '[.drops[]] | add // 0' <<<"$reply")
[ "$got" -ge 1 ] || fail "the vanilla control block dropped nothing: $reply"
# The four carried blocks; the bloom is hand-breakable, the other three need the
# iron pickaxe held above.
for pair in "naturesaura:aura_bloom $((AX - 6))" "naturesaura:infused_stone $((AX - 4))" \
            "naturesaura:gold_brick $((AX - 2))" "naturesaura:nature_altar $AX"; do
    set -- $pair
    reply=$(break_block "$2" "$AY" $((AZ + 8)))
    echo "  $1: $reply"
    got=$(jq -r --arg d "$1" '.drops[$d] // 0' <<<"$reply")
    [ "$got" -ge 1 ] || fail "$1 dropped $got of itself: $reply"
    [ "$(block_name "$2" "$AY" $((AZ + 8)))" = "minecraft:air" ] \
        || fail "the broken $1 is still there"
done
# Decayed leaves use the jar's silk-touch-only loot table. First the differential:
# a plain pickaxe gets no drop, so the silk-touch result below cannot be vacuous.
ctl command "execute in minecraft:overworld run setblock $((AX + 4)) $AY $((AZ + 8)) naturesaura:decayed_leaves" >/dev/null
ctl wait 4
reply=$(break_block $((AX + 4)) "$AY" $((AZ + 8)))
echo "  decayed leaves, plain pickaxe: $reply"
got=$(jq -r '.drops["naturesaura:decayed_leaves"] // 0' <<<"$reply")
[ "$got" -eq 0 ] || fail "decayed leaves dropped themselves without silk touch ($got): $reply"
ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe[minecraft:enchantments={\"minecraft:silk_touch\":1}]" >/dev/null
ctl wait 4
ctl select minecraft:iron_pickaxe >/dev/null
held=$(ctl inventory | jq -c '.held')
grep -q "silk_touch" <<<"$held" || fail "the silk-touch pickaxe has no enchantment: $held"
ctl command "execute in minecraft:overworld run setblock $((AX + 6)) $AY $((AZ + 8)) naturesaura:decayed_leaves" >/dev/null
ctl wait 4
reply=$(break_block $((AX + 6)) "$AY" $((AZ + 8)))
echo "  decayed leaves, silk touch: $reply"
got=$(jq -r '.drops["naturesaura:decayed_leaves"] // 0' <<<"$reply")
[ "$got" -ge 1 ] || fail "silk touch did not drop decayed leaves: $reply"
[ "$(block_name $((AX + 6)) "$AY" $((AZ + 8)))" = "minecraft:air" ] \
    || fail "the silk-touch-broken decayed leaves are still there"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
echo "  all five carried loot tables dropped their block (decayed leaves only with silk touch)"

echo "== J: a drained spot kills grass; an undrained spot under grass does not =="
# Two identical grass blocks 100 blocks apart, outside each other's influence.
ctl command "execute in minecraft:overworld run setblock $GX $GY $GZ minecraft:grass_block" >/dev/null
ctl command "execute in minecraft:overworld run setblock $KX $KY $KZ minecraft:grass_block" >/dev/null
ctl wait 6 >/dev/null
[ "$(block_name "$GX" "$GY" "$GZ")" = "minecraft:grass_block" ] || fail "the drained patch was not grass"
[ "$(block_name "$KX" "$KY" "$KZ")" = "minecraft:grass_block" ] || fail "the control patch was not grass"
spot "$GX" "$GY" "$GZ"
expect "the drained patch's spot before draining" "$SPOT" 0
say "naturesaura drain $GX $GY $GZ 2500000"
ctl wait 10 >/dev/null
spot "$GX" "$GY" "$GZ"
expect "the drained patch's spot" "$SPOT" -2500000
aura "$GX" "$GY" "$GZ" 50
expect "the drained area crossed zero" "$AREA" -1500000
# One effect second is 20 ticks; wait more than two to be independent of phase.
ctl wait 45 >/dev/null
spot "$GX" "$GY" "$GZ"
expect "the drained patch's spot after the effect" "$SPOT" -2500000
if [ "$SELFTEST" = effect ]; then
    expect "the drained patch (selftest: must fail)" "$(block_name "$GX" "$GY" "$GZ")" "minecraft:grass_block"
else
    expect "the drained patch became coarse dirt" "$(block_name "$GX" "$GY" "$GZ")" "minecraft:coarse_dirt"
fi
expect "the undrained control patch stays grass" "$(block_name "$KX" "$KY" "$KZ")" "minecraft:grass_block"
echo "  spot $SPOT, area $AREA: the drained patch became coarse dirt while the undrained patch stayed grass"

echo "== K: a drained spot turns leaves into decayed leaves; undrained leaves stay =="
# `persistent=true` keeps the placed leaves from decaying on a random tick before
# the effect's second; the branch matches the block id, not its state.
ctl command "execute in minecraft:overworld run setblock $LX $LY $LZ minecraft:oak_leaves[persistent=true]" >/dev/null
ctl command "execute in minecraft:overworld run setblock $MX $MY $MZ minecraft:oak_leaves[persistent=true]" >/dev/null
ctl wait 6
expect "the drained leaves patch was oak leaves" "$(block_name "$LX" "$LY" "$LZ")" "minecraft:oak_leaves"
expect "the control leaves patch was oak leaves" "$(block_name "$MX" "$MY" "$MZ")" "minecraft:oak_leaves"
spot "$LX" "$LY" "$LZ"
expect "the leaves patch's spot before draining" "$SPOT" 0
say "naturesaura drain $LX $LY $LZ 2500000"
ctl wait 10
spot "$LX" "$LY" "$LZ"
expect "the leaves patch's spot" "$SPOT" -2500000
aura "$LX" "$LY" "$LZ" 50
expect "the leaves area crossed zero" "$AREA" -1500000
# One effect second is 20 ticks; wait more than two to be independent of phase.
ctl wait 45 >/dev/null
spot "$LX" "$LY" "$LZ"
expect "the leaves patch's spot after the effect" "$SPOT" -2500000
if [ "$SELFTEST" = effect-leaf ]; then
    expect "the drained leaves patch (selftest: must fail)" "$(block_name "$LX" "$LY" "$LZ")" "minecraft:oak_leaves"
else
    expect "the drained leaves patch became decayed leaves" "$(block_name "$LX" "$LY" "$LZ")" "naturesaura:decayed_leaves"
fi
expect "the undrained control leaves patch stays oak leaves" "$(block_name "$MX" "$MY" "$MZ")" "minecraft:oak_leaves"
echo "  spot $SPOT, area $AREA: the drained leaves became decayed leaves while the undrained leaves stayed oak"

echo "PASS: a bloom stores exactly 150,000 aura once, an altar outside its multiblock and an altar in an area with no aura transmute nothing, the complete altar turns an iron ingot into infused iron while the area and its container account for exactly the recipe's 15,040 aura, both carried recipes craft their declared id and count in a real table with the grid emptied and the distinctive ingredient consumed while two wrong-ingredient controls craft nothing, the load log has no rejected ingredient, all eight carried items read back, all five carried blocks place and drop themselves with a vanilla stone as the drop differential, unknown ids are refused, a drained spot turns grass into coarse dirt while an undrained spot leaves it grass, and the same spot turns vanilla leaves into naturesaura:decayed_leaves while undrained leaves stay vanilla"
