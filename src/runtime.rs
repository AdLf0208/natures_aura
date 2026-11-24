// Nature's Aura, slice 1: the aura store, the aura bloom's generation, the
// nature's altar's multiblock and its transmutation rules.
//
// This file is both the standalone crate library and the injected
// `crate::game::naturesaura` module, so it holds no game access: the glue in
// `support.rs` reads chunks, block entities and blocks and calls in here.
//
// Ported from Nature's Aura 41.10 (MIT), Minecraft 1.21.1 NeoForge, by
// rewriting the behaviour of `AuraChunk`, `BlockEntityImpl.generateAura`,
// `BlockEntityAuraBloom`, `BlockEntityNatureAltar`, `Multiblocks.ALTAR` and
// `Multiblocks`/`Multiblock`. See `FINDINGS.md`.

use std::collections::BTreeMap;

/// The aura every chunk of an aura-bearing level starts with, before any drain
/// spot deviates from it (`IAuraChunk.DEFAULT_AURA`).
pub const DEFAULT_AURA: i32 = 1_000_000;

/// What an aura bloom stores the one time it generates
/// (`BlockEntityAuraBloom.generateAura(150000)`).
pub const BLOOM_GENERATED_AURA: i32 = 150_000;

/// The radius a generator looks for the emptiest drain spot in
/// (`BlockEntityImpl.generateAura`).
pub const GENERATE_RADIUS: i32 = 35;

/// The radius the altar looks for the fullest drain spot in
/// (`BlockEntityNatureAltar.tick`).
pub const ALTAR_RADIUS: i32 = 20;

/// The altar's own aura container (`BasicAuraContainer(null, 500000)`).
pub const ALTAR_MAX_AURA: i32 = 500_000;

/// How much aura a complete altar moves from the chunk into its container each
/// tick (`container.storeAura(300, true)`).
pub const ALTAR_PULL_PER_TICK: i32 = 300;

/// How often a complete altar re-checks its multiblock
/// (`level.getGameTime() % 40 == 0`).
pub const ALTAR_CHECK_TICKS: i64 = 40;

// ---------------------------------------------------------------------------
// Drain spots
// ---------------------------------------------------------------------------

/// The drain spots of one chunk: `pos -> amount`, the shape
/// `AuraChunk.drainSpots` saves as `{drain_spots: [{pos, amount}]}`.
///
/// A spot at 0 is absent (the Java store removes it), and a spot may be
/// negative — that is aura the area owes, and the next generator fills it first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChunkAura {
    spots: BTreeMap<[i32; 3], i32>,
}

impl ChunkAura {
    /// An empty chunk store: every spot reads 0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A store built from saved pairs; pairs at 0 are dropped, as the Java
    /// store drops them.
    #[must_use]
    pub fn from_spots(spots: impl IntoIterator<Item = ([i32; 3], i32)>) -> Self {
        let spots = spots
            .into_iter()
            .filter(|&(_, amount)| amount != 0)
            .collect();
        Self { spots }
    }

    /// Every non-zero spot, in position order (deterministic for saving).
    pub fn spots(&self) -> impl Iterator<Item = ([i32; 3], i32)> + '_ {
        self.spots.iter().map(|(&pos, &amount)| (pos, amount))
    }

    /// How much aura one spot holds (`getDrainSpot`).
    #[must_use]
    pub fn amount_at(&self, pos: [i32; 3]) -> i32 {
        self.spots.get(&pos).copied().unwrap_or(0)
    }

    /// Stores `amount` at `pos`, answering how much was stored
    /// (`IAuraChunk.storeAura`). With `aim_for_zero` a store that would cross
    /// from a negative spot to a positive one stops at 0 instead.
    pub fn store(&mut self, pos: [i32; 3], amount: i32, aim_for_zero: bool) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let mut amount = amount;
        let current = self.amount_at(pos);
        if aim_for_zero && current < 0 && current.saturating_add(amount) > 0 {
            amount = -current;
        }
        if amount == 0 {
            return 0;
        }
        self.set(pos, current.saturating_add(amount));
        amount
    }

    /// Drains `amount` from `pos`, answering how much was drained
    /// (`IAuraChunk.drainAura`). With `aim_for_zero` a drain that would cross
    /// from a positive spot to a negative one stops at 0 instead; without it a
    /// drain from an empty spot leaves a negative one, as the altar's own
    /// `drainAura(pos, amount)` does.
    pub fn drain(&mut self, pos: [i32; 3], amount: i32, aim_for_zero: bool) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let mut amount = amount;
        let current = self.amount_at(pos);
        if aim_for_zero && current > 0 && current.saturating_sub(amount) < 0 {
            amount = current;
        }
        self.set(pos, current.saturating_sub(amount));
        amount
    }

    /// The spots within `radius` of `center`, as the Java `getSpots(pos,
    /// radius, consumer)` selects them (`distSqr <= radius * radius`).
    pub fn spots_in(
        &self,
        center: [i32; 3],
        radius: i32,
    ) -> impl Iterator<Item = ([i32; 3], i32)> + '_ {
        let limit = i64::from(radius) * i64::from(radius);
        self.spots.iter().filter_map(move |(&pos, &amount)| {
            let d = delta_squared(center, pos);
            (d <= limit).then_some((pos, amount))
        })
    }

    /// The sum of the spots in radius (`AuraChunk.getAuraAndSpotAmount`'s aura
    /// half): the area's aura is [`DEFAULT_AURA`] plus this.
    #[must_use]
    pub fn sum_in(&self, center: [i32; 3], radius: i32) -> i32 {
        self.spots_in(center, radius)
            .fold(0i32, |sum, (_, amount)| sum.saturating_add(amount))
    }

    /// How many spots are in radius.
    #[must_use]
    pub fn count_in(&self, center: [i32; 3], radius: i32) -> usize {
        self.spots_in(center, radius).count()
    }

    /// The most indebted spot in radius, with its amount — `getLowestSpot`
    /// only answers one when it is below 0.
    #[must_use]
    pub fn lowest_in(&self, center: [i32; 3], radius: i32) -> Option<([i32; 3], i32)> {
        self.spots_in(center, radius)
            .filter(|&(_, amount)| amount < 0)
            .min_by_key(|&(pos, amount)| (amount, pos))
    }

    /// The fullest spot in radius, with its amount — `getHighestSpot` only
    /// answers one when it is above 0.
    #[must_use]
    pub fn highest_in(&self, center: [i32; 3], radius: i32) -> Option<([i32; 3], i32)> {
        self.spots_in(center, radius)
            .filter(|&(_, amount)| amount > 0)
            .max_by_key(|&(pos, amount)| (amount, std::cmp::Reverse(pos)))
    }

    fn set(&mut self, pos: [i32; 3], amount: i32) {
        if amount == 0 {
            self.spots.remove(&pos);
        } else {
            self.spots.insert(pos, amount);
        }
    }
}

/// The squared block distance between two positions, as `BlockPos.distSqr`
/// measures it.
fn delta_squared(a: [i32; 3], b: [i32; 3]) -> i64 {
    let d = |i: usize| i64::from(a[i]) - i64::from(b[i]);
    d(0) * d(0) + d(1) * d(1) + d(2) * d(2)
}

// ---------------------------------------------------------------------------
// Altar recipes
// ---------------------------------------------------------------------------

/// One `naturesaura:altar` recipe without a catalyst: an input item, what it
/// becomes, and the aura and ticks the altar spends on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AltarRecipe {
    /// The item in the altar's slot (`input`).
    pub input: &'static str,
    /// What the slot holds afterwards (`output.id`).
    pub output: &'static str,
    /// The output's count (`output.count`, 1 when absent).
    pub count: u32,
    /// The aura the whole transmutation costs (`aura`).
    pub aura: i32,
    /// The ticks it takes (`time`).
    pub time: i32,
}

/// Every altar recipe of the jar that names no catalyst (the other 31 need a
/// catalyst block this slice does not carry; see `FINDINGS.md` §3.1).
pub const ALTAR_RECIPES: [AltarRecipe; 3] = [
    AltarRecipe {
        input: "minecraft:iron_ingot",
        output: "naturesaura:infused_iron",
        count: 1,
        aura: 15_000,
        time: 80,
    },
    AltarRecipe {
        input: "minecraft:stone",
        output: "naturesaura:infused_stone",
        count: 1,
        aura: 7_500,
        time: 40,
    },
    AltarRecipe {
        input: "minecraft:gold_ingot",
        output: "naturesaura:tainted_gold",
        count: 1,
        aura: 3_000,
        time: 300,
    },
];

/// The recipe an item in the slot starts (`getRecipeForInput`), by index.
#[must_use]
pub fn altar_recipe(input: &str) -> Option<usize> {
    ALTAR_RECIPES
        .iter()
        .position(|recipe| recipe.input == input)
}

/// The recipe a saved `recipe` index names, or `None` when the save holds an
/// index the table does not have (an older or foreign save, or a hand-edited
/// one). [`Altar::step`] indexes the table directly, so a restored index has to
/// be checked here first: an unchecked one panicked the altar's tick.
#[must_use]
pub fn recipe_at(index: usize) -> Option<&'static AltarRecipe> {
    ALTAR_RECIPES.get(index)
}

/// The aura one tick of a recipe costs: `ceil(aura / time)`
/// (`BlockEntityNatureAltar.tick`).
///
/// # Panics
///
/// Panics when a recipe's `time` is not positive; the table above has none.
#[must_use]
pub fn aura_per_tick(recipe: &AltarRecipe) -> i32 {
    assert!(recipe.time > 0, "an altar recipe needs a positive time");
    (recipe.aura + recipe.time - 1) / recipe.time
}

// ---------------------------------------------------------------------------
// The altar's own state
// ---------------------------------------------------------------------------

/// The altar's saved state (`writeNBT`): its aura container, the recipe it is
/// working on and how far it got.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Altar {
    /// Aura in the altar's container (`container.writeNBT`).
    pub aura: i32,
    /// Ticks the current recipe has run (`timer`).
    pub timer: i32,
    /// The recipe being worked on, as an index into [`ALTAR_RECIPES`]
    /// (upstream saves its id as `recipe`).
    pub recipe: Option<usize>,
}

impl Altar {
    /// An empty altar.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How much aura the container can take this tick: at most
    /// [`ALTAR_PULL_PER_TICK`], and nothing when it is full.
    #[must_use]
    pub fn pull_room(&self) -> i32 {
        (ALTAR_MAX_AURA - self.aura).clamp(0, ALTAR_PULL_PER_TICK)
    }

    /// Adds what the chunk gave (`this.container.storeAura(toStore, false)`).
    pub fn take_pulled(&mut self, amount: i32) {
        if amount > 0 {
            self.aura = self.aura.saturating_add(amount).min(ALTAR_MAX_AURA);
        }
    }

    /// One tick of the ritual, with whatever the slot holds
    /// (`BlockEntityNatureAltar.tick`, the non-aura-container branch).
    ///
    /// The first tick an input is present only starts the recipe; each later
    /// tick spends [`aura_per_tick`] while the container can pay it, and after
    /// `time` ticks answers the output item the slot should hold.
    pub fn step(&mut self, slot: Option<&str>) -> Option<&'static AltarRecipe> {
        let Some(index) = self.recipe else {
            self.recipe = slot.and_then(altar_recipe);
            return None;
        };
        let Some(recipe) = recipe_at(index) else {
            // A saved index the table no longer has: drop the run instead of
            // panicking on the table's bounds.
            self.recipe = None;
            self.timer = 0;
            return None;
        };
        if slot != Some(recipe.input) {
            self.recipe = None;
            self.timer = 0;
            return None;
        }
        let required = aura_per_tick(recipe);
        if self.aura < required {
            return None;
        }
        self.aura -= required;
        self.timer += 1;
        if self.timer < recipe.time {
            return None;
        }
        self.recipe = None;
        self.timer = 0;
        Some(recipe)
    }
}

// ---------------------------------------------------------------------------
// The altar multiblock
// ---------------------------------------------------------------------------

/// What one matched position of the altar multiblock accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltarPart {
    /// The altar itself (`0`).
    Altar,
    /// `naturesaura:altar_stone`: `#minecraft:stone_bricks` and nether bricks
    /// (`B`).
    Stone,
    /// `naturesaura:altar_wood`: `#minecraft:planks`, crimson and warped
    /// planks (`W`).
    Wood,
    /// `naturesaura:altar_fancy_brick`: red nether bricks and chiseled stone
    /// bricks (`C`).
    Fancy,
    /// `naturesaura:altar_gold_brick`: the mod's two gold brick blocks (`M`).
    Gold,
}

/// The altar multiblock's pattern, layer by layer from the top: `Multiblocks.
/// ALTAR`, the four `String[]` rows as they are written there.
const ALTAR_PATTERN: [&[&str; 9]; 4] = [
    &[
        "    M    ",
        "         ",
        "         ",
        "         ",
        "M       M",
        "         ",
        "         ",
        "         ",
        "    M    ",
    ],
    &[
        "    B    ",
        "         ",
        "         ",
        "         ",
        "B       B",
        "         ",
        "         ",
        "         ",
        "    B    ",
    ],
    &[
        "    B    ",
        "         ",
        "  M   M  ",
        "         ",
        "B   0   B",
        "         ",
        "  M   M  ",
        "         ",
        "    B    ",
    ],
    &[
        "         ",
        "   WBW   ",
        "   WBW   ",
        " WWCWCWW ",
        " BBW WBB ",
        " WWCWCWW ",
        "   WBW   ",
        "   WBW   ",
        "         ",
    ],
];

/// Where the pattern's `0` sits inside it (`Multiblock.xOffset/yOffset/zOffset`):
/// the altar is at the raw offset `(4, 1, 4)` of a 9 x 4 x 9 box.
pub const ALTAR_OFFSET: [i32; 3] = [4, 1, 4];

/// Every matched position of the altar multiblock, relative to the altar's own
/// position, with what it accepts. Blank positions match anything and are not
/// listed, as `Multiblock`'s matcher map leaves them out.
#[must_use]
pub fn altar_parts() -> Vec<([i32; 3], AltarPart)> {
    let mut parts = Vec::new();
    for (layer, rows) in ALTAR_PATTERN.iter().enumerate() {
        let y = 3 - layer as i32;
        for (z, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let part = match c {
                    '0' => AltarPart::Altar,
                    'B' => AltarPart::Stone,
                    'W' => AltarPart::Wood,
                    'C' => AltarPart::Fancy,
                    'M' => AltarPart::Gold,
                    _ => continue,
                };
                let pos = [
                    x as i32 - ALTAR_OFFSET[0],
                    y - ALTAR_OFFSET[1],
                    z as i32 - ALTAR_OFFSET[2],
                ];
                parts.push((pos, part));
            }
        }
    }
    parts
}

/// Whether a block id satisfies one part, by the ids the jar's `altar_*` block
/// tags hold.
#[must_use]
pub fn part_accepts(part: AltarPart, block: &str) -> bool {
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    match part {
        AltarPart::Altar => block == "naturesaura:nature_altar",
        AltarPart::Stone => matches!(
            name,
            "stone_bricks"
                | "mossy_stone_bricks"
                | "cracked_stone_bricks"
                | "chiseled_stone_bricks"
                | "nether_bricks"
        ),
        AltarPart::Wood => {
            name.ends_with("_planks") && is_vanilla_wood(name.trim_end_matches("_planks"))
        }
        AltarPart::Fancy => matches!(name, "red_nether_bricks" | "chiseled_stone_bricks"),
        AltarPart::Gold => {
            block == "naturesaura:gold_brick" || block == "naturesaura:gold_nether_brick"
        }
    }
}

/// The eleven wood types whose planks `#minecraft:planks` holds.
fn is_vanilla_wood(wood: &str) -> bool {
    matches!(
        wood,
        "oak"
            | "spruce"
            | "birch"
            | "jungle"
            | "acacia"
            | "dark_oak"
            | "mangrove"
            | "cherry"
            | "bamboo"
            | "crimson"
            | "warped"
    )
}

/// Whether the multiblock around the altar at `altar` is complete
/// (`Multiblocks.ALTAR.isComplete`), reading each matched position through
/// `block_at`.
#[must_use]
pub fn altar_complete(altar: [i32; 3], block_at: impl Fn([i32; 3]) -> String) -> bool {
    altar_parts().into_iter().all(|(offset, part)| {
        let pos = [
            altar[0] + offset[0],
            altar[1] + offset[1],
            altar[2] + offset[2],
        ];
        part_accepts(part, &block_at(pos))
    })
}

/// How far a player without operator permission may be from an altar to read it.
pub const INSPECT_RANGE: f64 = 8.0;

/// `Commands.LEVEL_ADMINS`: the permission level that bypasses the reach check.
pub const LEVEL_ADMINS: u8 = 2;

/// Whether a caller may read an altar's contents: an operator, or a player in
/// the same level within [`INSPECT_RANGE`] of the block.
pub fn may_inspect(op: bool, same_level: bool, distance: f64) -> bool {
    op || (same_level && distance <= INSPECT_RANGE)
}

/// Whether taking a finished output may clear the altar's slot: only a full
/// transfer commits. An incompatible hand inserts nothing, and the old code
/// still cleared the slot (the review finding).
pub fn transfer_is_complete(count: u32, inserted: u32) -> bool {
    count > 0 && count == inserted
}

// ---------------------------------------------------------------------------
// Drain-spot passive effect: grass death (`GrassDieEffect`)
// ---------------------------------------------------------------------------

/// The radius the drain-spot effect measures the area over
/// (`getAuraAndSpotAmountInArea(level, pos, 50)`).
pub const GRASS_DIE_RADIUS: i32 = 50;

/// What one grass-die pass derives from a spot (`GrassDieEffect.calcValues`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrassDie {
    /// How many blocks the pass tries to change (`amount`).
    pub amount: i32,
    /// The scatter radius, clamped to 5..=75 (`dist`).
    pub dist: i32,
}

/// Whether a drained spot's grass-die effect fires, and with what reach.
///
/// Upstream fires only at a *negative* spot whose radius-50 area aura
/// (`1_000_000 + the spot sum`) is also negative, and only when the debt is
/// concentrated enough to make `amount > 1`.
pub fn grass_die(spot: i32, area_aura: i32, spot_count: i32) -> Option<GrassDie> {
    if spot >= 0 || spot_count <= 0 || area_aura >= 0 {
        return None;
    }
    let magnitude = i64::from(area_aura.unsigned_abs());
    // `Mth.ceil(abs(aura) / 100000F / spotCount)` — one division, so the rounding
    // is on the whole quotient, not on `abs(aura) / 100000` first.
    let denominator = 100_000_i64 * i64::from(spot_count);
    let amount = ((magnitude + denominator - 1) / denominator).min(300);
    if amount <= 1 {
        return None;
    }
    // `Mth.clamp(abs(aura) / 75000, 5, 75)` — Java's integer division.
    let dist = (magnitude / 75_000).clamp(5, 75);
    Some(GrassDie {
        amount: amount as i32,
        dist: dist as i32,
    })
}

/// The next value of the port's small deterministic generator. Upstream samples
/// `level.random`; the port owns its sequence so a drained spot's scatter is
/// reproducible from the spot alone (recorded as a deviation in `notes.md`).
fn next_offset(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 33
}

/// Where one grass-die pass looks for grass: the spot itself first, then
/// `amount - 1` positions scattered within `dist` (halved on Y, as upstream's
/// `dist / 2` does). Starting at the spot is a deliberate deviation: upstream's
/// pure gaussian scatter can miss the very spot that drives the effect, which
/// makes it unobservable in a small scenario.
#[must_use]
pub fn grass_die_targets(pos: [i32; 3], params: &GrassDie) -> Vec<[i32; 3]> {
    let mut targets = Vec::with_capacity(params.amount.max(1) as usize);
    targets.push(pos);
    let mut seed = (pos[0] as u64 & 0xffff_ffff)
        ^ ((pos[1] as u64 & 0xffff) << 32)
        ^ ((pos[2] as u64 & 0xffff) << 48)
        ^ 0x9e37_79b9_7f4a_7c15;
    let (span, yspan) = (params.dist.max(1), (params.dist / 2).max(1));
    for _ in 1..params.amount.max(1) {
        let x = pos[0] + (next_offset(&mut seed) % (2 * span as u64 + 1)) as i32 - span;
        let y = pos[1] + (next_offset(&mut seed) % (2 * yspan as u64 + 1)) as i32 - yspan;
        let z = pos[2] + (next_offset(&mut seed) % (2 * span as u64 + 1)) as i32 - span;
        targets.push([x, y, z]);
    }
    targets
}

/// The vanilla `LeavesBlock` ids at this pin, i.e. the artefact's
/// `block instanceof LeavesBlock` target set (`GrassDieEffect.update`). Read from
/// the prepared 26.3 resources' `assets/minecraft/blockstates/*_leaves.json`.
/// The mod's own leaves are not carried, and decayed leaves extend `BlockImpl`,
/// not `LeavesBlock`, so neither is a target.
pub const LEAVES_BLOCKS: [&str; 14] = [
    "minecraft:acacia_leaves",
    "minecraft:azalea_leaves",
    "minecraft:birch_leaves",
    "minecraft:cherry_leaves",
    "minecraft:dark_oak_leaves",
    "minecraft:flowering_azalea_leaves",
    "minecraft:jungle_leaves",
    "minecraft:mangrove_leaves",
    "minecraft:oak_leaves",
    "minecraft:orange_poplar_leaves",
    "minecraft:pale_oak_leaves",
    "minecraft:red_poplar_leaves",
    "minecraft:spruce_leaves",
    "minecraft:yellow_poplar_leaves",
];

/// What a grass-die pass replaces a block with (`GrassDieEffect.update`).
///
/// The artefact checks `instanceof LeavesBlock` first, then `GrassBlock`, then
/// `BushBlock` and its own nether grass. The port matches ids (the pin has no
/// class/tag test in a tick), carrying the leaf and grass branches; the bush and
/// nether-grass branches remain gaps recorded in `notes.md`.
pub fn grass_die_replacement(block: &str) -> Option<&'static str> {
    if LEAVES_BLOCKS.contains(&block) {
        return Some("naturesaura:decayed_leaves");
    }
    (block == "minecraft:grass_block").then_some("minecraft:coarse_dirt")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLOOM: [i32; 3] = [206, 200, -6];
    const ALTAR: [i32; 3] = [200, 200, 0];

    #[test]
    fn a_bloom_adds_exactly_its_aura_to_the_area() {
        let mut chunk = ChunkAura::new();
        assert_eq!(chunk.sum_in(ALTAR, ALTAR_RADIUS), 0);
        assert_eq!(
            chunk.store(BLOOM, BLOOM_GENERATED_AURA, true),
            BLOOM_GENERATED_AURA
        );
        assert_eq!(chunk.amount_at(BLOOM), BLOOM_GENERATED_AURA);
        assert_eq!(
            DEFAULT_AURA + chunk.sum_in(ALTAR, ALTAR_RADIUS),
            DEFAULT_AURA + BLOOM_GENERATED_AURA
        );
        assert_eq!(chunk.count_in(ALTAR, ALTAR_RADIUS), 1);
    }

    #[test]
    fn a_store_that_reaches_zero_from_below_stops_there() {
        let mut chunk = ChunkAura::from_spots([([1, 2, 3], -100)]);
        assert_eq!(chunk.store([1, 2, 3], 300, true), 100);
        assert_eq!(chunk.amount_at([1, 2, 3]), 0, "the spot is gone at 0");
        assert_eq!(chunk.spots().count(), 0, "the spot is gone at 0");
    }

    #[test]
    fn a_store_without_aiming_for_zero_pays_the_whole_debt() {
        let mut chunk = ChunkAura::from_spots([([1, 2, 3], -100)]);
        assert_eq!(chunk.store([1, 2, 3], 300, false), 300);
        assert_eq!(chunk.amount_at([1, 2, 3]), 200);
    }

    #[test]
    fn a_drain_that_would_cross_zero_stops_there_when_asked() {
        let mut chunk = ChunkAura::from_spots([([4, 5, 6], 250)]);
        assert_eq!(chunk.drain([4, 5, 6], 300, true), 250);
        assert_eq!(chunk.amount_at([4, 5, 6]), 0);
    }

    #[test]
    fn the_altars_drain_without_aiming_leaves_a_debt() {
        let mut chunk = ChunkAura::new();
        assert_eq!(chunk.drain([4, 5, 6], 300, false), 300);
        assert_eq!(chunk.amount_at([4, 5, 6]), -300);
        assert_eq!(
            DEFAULT_AURA + chunk.sum_in([4, 5, 6], 0),
            DEFAULT_AURA - 300,
            "a drained chunk reads below the base aura"
        );
    }

    #[test]
    fn the_emptiest_spot_is_answered_only_when_it_is_in_debt() {
        let chunk = ChunkAura::from_spots([([1, 0, 0], 500), ([2, 0, 0], -40), ([3, 0, 0], -900)]);
        assert_eq!(
            chunk.lowest_in([0, 0, 0], GENERATE_RADIUS),
            Some(([3, 0, 0], -900))
        );
        let paid = ChunkAura::from_spots([([1, 0, 0], 500)]);
        assert_eq!(paid.lowest_in([0, 0, 0], GENERATE_RADIUS), None);
    }

    #[test]
    fn the_fullest_spot_is_answered_only_when_it_holds_aura() {
        let chunk = ChunkAura::from_spots([([1, 0, 0], 500), ([2, 0, 0], 900)]);
        assert_eq!(
            chunk.highest_in([0, 0, 0], ALTAR_RADIUS),
            Some(([2, 0, 0], 900))
        );
        let debt = ChunkAura::from_spots([([1, 0, 0], -500)]);
        assert_eq!(debt.highest_in([0, 0, 0], ALTAR_RADIUS), None);
    }

    #[test]
    fn spots_outside_the_radius_are_ignored() {
        let chunk = ChunkAura::from_spots([([0, 0, 0], 100), ([0, 21, 0], 900)]);
        assert_eq!(chunk.sum_in([0, 0, 0], ALTAR_RADIUS), 100);
        assert_eq!(chunk.count_in([0, 0, 0], ALTAR_RADIUS), 1);
    }

    #[test]
    fn only_the_catalyst_free_altar_recipes_are_known() {
        assert_eq!(altar_recipe("minecraft:iron_ingot"), Some(0));
        assert_eq!(altar_recipe("minecraft:stone"), Some(1));
        assert_eq!(altar_recipe("minecraft:gold_ingot"), Some(2));
        assert_eq!(
            altar_recipe("minecraft:charcoal"),
            None,
            "charcoal needs a catalyst"
        );
    }

    #[test]
    fn a_recipe_costs_its_aura_rounded_up_per_tick() {
        assert_eq!(aura_per_tick(&ALTAR_RECIPES[0]), 188, "ceil(15000 / 80)");
        assert_eq!(aura_per_tick(&ALTAR_RECIPES[1]), 188, "ceil(7500 / 40)");
        assert_eq!(aura_per_tick(&ALTAR_RECIPES[2]), 10, "ceil(3000 / 300)");
    }

    #[test]
    fn the_altar_turns_an_ingot_into_infused_iron_after_eighty_ticks() {
        let mut altar = Altar::new();
        assert_eq!(
            altar.step(Some("minecraft:iron_ingot")),
            None,
            "the first tick only starts"
        );
        assert_eq!(altar.recipe, Some(0));
        // One tick as `BlockEntityNatureAltar.tick` runs it: make room, take
        // what the chunk gives, then run the recipe.
        let mut pulled = 0;
        let mut ticks = 0;
        let output = loop {
            ticks += 1;
            let room = altar.pull_room();
            altar.take_pulled(room);
            pulled += room;
            if let Some(recipe) = altar.step(Some("minecraft:iron_ingot")) {
                break recipe;
            }
            assert!(ticks < 200, "the recipe never finished");
        };
        assert_eq!(ticks, 80);
        assert_eq!(output.output, "naturesaura:infused_iron");
        assert_eq!(pulled, 300 * 80, "a full pull every tick");
        assert_eq!(
            altar.aura,
            pulled - 188 * 80,
            "the recipe over-spends by its rounding"
        );
        assert_eq!(altar.recipe, None);
        assert_eq!(altar.timer, 0);
    }

    #[test]
    fn a_dry_altar_does_not_progress() {
        let mut altar = Altar::new();
        altar.step(Some("minecraft:iron_ingot"));
        for _ in 0..200 {
            assert_eq!(altar.step(Some("minecraft:iron_ingot")), None);
        }
        assert_eq!(altar.timer, 0, "no tick ran without aura");
        assert_eq!(altar.aura, 0);
    }

    #[test]
    fn an_altar_holds_at_most_188_less_than_a_tick_of_aura() {
        let mut altar = Altar::new();
        altar.aura = 187;
        altar.step(Some("minecraft:stone"));
        assert_eq!(altar.timer, 0);
        altar.aura = 188;
        altar.step(Some("minecraft:stone"));
        assert_eq!(altar.timer, 1);
        assert_eq!(altar.aura, 0);
    }

    #[test]
    fn a_saved_recipe_index_past_the_table_is_dropped_not_indexed() {
        // `support.rs` restores the saved `recipe` index into `Altar.recipe`;
        // an index the three-recipe table does not have used to panic the tick.
        assert!(recipe_at(0).is_some());
        assert!(recipe_at(ALTAR_RECIPES.len() - 1).is_some());
        assert!(recipe_at(ALTAR_RECIPES.len()).is_none());
        assert!(recipe_at(usize::MAX).is_none());
        let mut altar = Altar::new();
        altar.aura = ALTAR_MAX_AURA;
        altar.recipe = Some(ALTAR_RECIPES.len() + 7);
        assert_eq!(altar.step(Some("minecraft:iron_ingot")), None);
        assert_eq!(altar.recipe, None, "the broken run is reset");
        assert_eq!(altar.timer, 0);
        // The next tick starts the item's own recipe again.
        assert_eq!(altar.step(Some("minecraft:iron_ingot")), None);
        assert_eq!(altar.recipe, Some(0));
    }

    #[test]
    fn a_swapped_input_restarts_the_ritual() {
        let mut altar = Altar::new();
        altar.aura = 10_000;
        altar.step(Some("minecraft:iron_ingot"));
        for _ in 0..10 {
            altar.step(Some("minecraft:iron_ingot"));
        }
        assert_eq!(altar.timer, 10);
        assert_eq!(altar.step(Some("minecraft:stone")), None);
        assert_eq!(altar.recipe, None);
        assert_eq!(altar.timer, 0);
        altar.step(Some("minecraft:stone"));
        assert_eq!(altar.recipe, Some(1));
    }

    #[test]
    fn the_container_takes_at_most_three_hundred_a_tick() {
        let mut altar = Altar::new();
        assert_eq!(altar.pull_room(), ALTAR_PULL_PER_TICK);
        altar.take_pulled(300);
        assert_eq!(altar.aura, 300);
        altar.aura = ALTAR_MAX_AURA - 10;
        assert_eq!(
            altar.pull_room(),
            10,
            "a nearly full container takes only its room"
        );
        altar.take_pulled(10);
        assert_eq!(altar.pull_room(), 0);
        assert_eq!(altar.aura, ALTAR_MAX_AURA);
    }

    #[test]
    fn the_altar_pattern_is_the_jars_forty_eight_positions_plus_the_altar() {
        let parts = altar_parts();
        assert_eq!(parts.len(), 49, "48 matched blocks and the altar itself");
        let count = |kind: AltarPart| parts.iter().filter(|(_, p)| *p == kind).count();
        // Counted from the four `String[]` rows of `Multiblocks.ALTAR`:
        // 20 W + 16 B + 8 M + 4 C + the altar.
        assert_eq!(count(AltarPart::Altar), 1);
        assert_eq!(count(AltarPart::Gold), 8);
        assert_eq!(count(AltarPart::Stone), 16);
        assert_eq!(count(AltarPart::Wood), 20);
        assert_eq!(count(AltarPart::Fancy), 4);
        // The gold bricks: the top layer's four corners and the four mid ones.
        for offset in [[0, 2, -4], [-4, 2, 0], [4, 2, 0], [0, 2, 4]] {
            assert!(
                parts.contains(&(offset, AltarPart::Gold)),
                "top gold brick at {offset:?}"
            );
        }
        for offset in [[-2, 0, -2], [2, 0, -2], [-2, 0, 2], [2, 0, 2]] {
            assert!(
                parts.contains(&(offset, AltarPart::Gold)),
                "mid gold brick at {offset:?}"
            );
        }
        // The stone bricks under the top corners, and the bottom ring.
        assert!(parts.contains(&([0, 1, -4], AltarPart::Stone)));
        assert!(parts.contains(&([0, 0, -4], AltarPart::Stone)));
        assert!(parts.contains(&([-3, -1, 0], AltarPart::Stone)));
        // The bottom ring's planks and fancy bricks, from the pattern's rows.
        assert!(parts.contains(&([-1, -1, -3], AltarPart::Wood)));
        assert!(parts.contains(&([-3, -1, -1], AltarPart::Wood)));
        assert!(parts.contains(&([-1, -1, -1], AltarPart::Fancy)));
        assert!(
            !parts.iter().any(|(pos, _)| *pos == [0, -1, 0]),
            "the bottom centre under the altar matches anything"
        );
        // The altar itself is the only Altar part, at the origin.
        assert!(parts.contains(&([0, 0, 0], AltarPart::Altar)));
    }

    #[test]
    fn the_parts_accept_exactly_the_jars_tags() {
        assert!(part_accepts(AltarPart::Stone, "minecraft:stone_bricks"));
        assert!(part_accepts(AltarPart::Stone, "minecraft:nether_bricks"));
        assert!(!part_accepts(AltarPart::Stone, "minecraft:stone"));
        assert!(part_accepts(AltarPart::Wood, "minecraft:oak_planks"));
        assert!(part_accepts(AltarPart::Wood, "minecraft:crimson_planks"));
        assert!(part_accepts(AltarPart::Wood, "minecraft:bamboo_planks"));
        assert!(!part_accepts(AltarPart::Wood, "minecraft:oak_log"));
        assert!(!part_accepts(AltarPart::Wood, "mymod:planks"));
        assert!(part_accepts(
            AltarPart::Fancy,
            "minecraft:red_nether_bricks"
        ));
        assert!(part_accepts(
            AltarPart::Fancy,
            "minecraft:chiseled_stone_bricks"
        ));
        assert!(!part_accepts(AltarPart::Fancy, "minecraft:stone_bricks"));
        assert!(part_accepts(AltarPart::Gold, "naturesaura:gold_brick"));
        assert!(part_accepts(
            AltarPart::Gold,
            "naturesaura:gold_nether_brick"
        ));
        assert!(!part_accepts(AltarPart::Gold, "minecraft:gold_block"));
        assert!(part_accepts(AltarPart::Altar, "naturesaura:nature_altar"));
        assert!(!part_accepts(AltarPart::Altar, "naturesaura:gold_brick"));
    }

    #[test]
    fn a_complete_structure_is_complete_and_one_missing_block_is_not() {
        let build = |skip: Option<[i32; 3]>| {
            let parts = altar_parts();
            move |pos: [i32; 3]| {
                let offset = [pos[0] - ALTAR[0], pos[1] - ALTAR[1], pos[2] - ALTAR[2]];
                if Some(offset) == skip {
                    return "minecraft:air".to_owned();
                }
                match parts.iter().find(|(p, _)| *p == offset) {
                    Some((_, AltarPart::Stone)) => "minecraft:nether_bricks",
                    Some((_, AltarPart::Wood)) => "minecraft:oak_planks",
                    Some((_, AltarPart::Fancy)) => "minecraft:red_nether_bricks",
                    Some((_, AltarPart::Gold)) => "naturesaura:gold_brick",
                    Some((_, AltarPart::Altar)) => "naturesaura:nature_altar",
                    None => "minecraft:air",
                }
                .to_owned()
            }
        };
        assert!(altar_complete(ALTAR, build(None)));
        assert!(
            !altar_complete(ALTAR, build(Some([-4, 2, 0]))),
            "one top gold brick short"
        );
        let wrong = |pos: [i32; 3]| {
            let offset = [pos[0] - ALTAR[0], pos[1] - ALTAR[1], pos[2] - ALTAR[2]];
            if offset == [-1, -1, -3] {
                "minecraft:stone".to_owned()
            } else {
                build(None)(pos)
            }
        };
        assert!(
            !altar_complete(ALTAR, wrong),
            "stone is not in the altar stone tag"
        );
    }

    #[test]
    fn an_altar_readout_needs_an_operator_reach_or_the_same_level() {
        assert!(may_inspect(true, false, f64::MAX), "an operator");
        assert!(may_inspect(false, true, 2.0), "a player in reach");
        assert!(!may_inspect(false, true, 9.0), "out of reach");
        assert!(!may_inspect(false, false, 1.0), "another level");
    }

    #[test]
    fn only_a_full_transfer_clears_the_output() {
        assert!(transfer_is_complete(3, 3));
        assert!(!transfer_is_complete(3, 0), "an incompatible hand");
        assert!(!transfer_is_complete(3, 2), "a partial insert");
        assert!(!transfer_is_complete(0, 0), "nothing to take");
    }

    #[test]
    fn grass_die_fires_only_at_a_debt_whose_area_is_negative() {
        // One spot of -2,500,000: the area is 1,000,000 - 2,500,000 = -1,500,000.
        // amount = ceil(1,500,000 / 100000 / 1) = 15; dist = clamp(1,500,000 /
        // 75,000 = 20, 5, 75) = 20.
        assert_eq!(
            grass_die(-2_500_000, -1_500_000, 1),
            Some(GrassDie {
                amount: 15,
                dist: 20
            })
        );
        // A positive spot never kills grass, even with a negative area (the
        // area's debt belongs to some other spot).
        assert_eq!(grass_die(1, -1_500_000, 1), None);
        // A negative spot in a still-positive area is inert.
        assert_eq!(grass_die(-100, DEFAULT_AURA - 100, 1), None);
        // A zero spot is no spot at all.
        assert_eq!(grass_die(0, DEFAULT_AURA, 0), None);
    }

    #[test]
    fn grass_die_concentrates_the_debt_over_the_spot_count() {
        // The same -1,500,000 area debt split over 20 spots: amount =
        // ceil(1,500,000 / (100000 * 20)) = 1, which upstream rejects.
        assert_eq!(grass_die(-75_000, -1_500_000, 20), None);
        // Over 5 spots it is ceil(1,500,000 / 500,000) = 3 and dist stays 20.
        assert_eq!(
            grass_die(-300_000, -1_500_000, 5),
            Some(GrassDie {
                amount: 3,
                dist: 20
            })
        );
        // amount is capped at 300 no matter how deep the debt.
        let huge = grass_die(-100_000_000, -99_999_000, 1).unwrap();
        assert_eq!(huge.amount, 300);
        assert_eq!(huge.dist, 75, "dist is capped at 75");
    }

    #[test]
    fn grass_die_targets_start_at_the_spot_and_stay_within_dist() {
        let params = GrassDie {
            amount: 15,
            dist: 20,
        };
        let targets = grass_die_targets([200, 200, 0], &params);
        assert_eq!(targets.len(), 15);
        assert_eq!(
            targets[0],
            [200, 200, 0],
            "the spot itself is the first target"
        );
        assert!(
            targets.iter().all(|&[x, y, z]| {
                (x - 200).abs() <= 20 && z.abs() <= 20 && (y - 200).abs() <= 10
            })
        );
        // Position-seeded, so two passes over the same spot agree.
        assert_eq!(targets, grass_die_targets([200, 200, 0], &params));
        assert_ne!(targets, grass_die_targets([201, 200, 0], &params));
    }

    #[test]
    fn grass_die_turns_grass_into_coarse_dirt() {
        assert_eq!(
            grass_die_replacement("minecraft:grass_block"),
            Some("minecraft:coarse_dirt")
        );
        // The bush -> air and nether-grass -> netherrack branches still need
        // blocks/behaviour this port does not carry; they remain gaps in notes.md.
        assert_eq!(grass_die_replacement("naturesaura:aura_bloom"), None);
        assert_eq!(grass_die_replacement("minecraft:dirt"), None);
    }

    #[test]
    fn grass_die_turns_leaves_into_decayed_leaves() {
        // `LeavesBlock` in 26.3 (the artefact's target set), read from the
        // prepared resources' blockstate ids. The list is written out here so
        // the assertion does not borrow the implementation's own set.
        for leaves in [
            "minecraft:acacia_leaves",
            "minecraft:azalea_leaves",
            "minecraft:birch_leaves",
            "minecraft:cherry_leaves",
            "minecraft:dark_oak_leaves",
            "minecraft:flowering_azalea_leaves",
            "minecraft:jungle_leaves",
            "minecraft:mangrove_leaves",
            "minecraft:oak_leaves",
            "minecraft:orange_poplar_leaves",
            "minecraft:pale_oak_leaves",
            "minecraft:red_poplar_leaves",
            "minecraft:spruce_leaves",
            "minecraft:yellow_poplar_leaves",
        ] {
            assert_eq!(
                grass_die_replacement(leaves),
                Some("naturesaura:decayed_leaves"),
                "leaves id {leaves}"
            );
        }
        // Decayed leaves extend BlockImpl, not LeavesBlock: a second pass must
        // leave them alone.
        assert_eq!(grass_die_replacement("naturesaura:decayed_leaves"), None);
        assert_eq!(
            grass_die_replacement("minecraft:grass_block"),
            Some("minecraft:coarse_dirt")
        );
    }
}
