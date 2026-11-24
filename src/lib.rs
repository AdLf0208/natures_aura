//! Nature's Aura for Horizon — slice 1: the aura loop as one `impl Hooks` and two Parts.
//!
//! Ported from Nature's Aura 41.10 (Ellpeck/NaturesAura, MIT); see `FINDINGS.md` for the
//! behaviour read from the jar and the upstream repository, and `notes.md` for the hook map,
//! the gaps and the migration to the SDK format.
//!
//! The two blocks are **block entities**, so their moments are Parts attached to the types
//! this mod declares, not global hooks: the aura bloom fills 150,000 aura at the emptiest
//! drain spot in radius 35 once, when it appears; the nature's altar checks its 9x4x9
//! multiblock, pulls 300 aura a tick out of the area it stands in (only in the Overworld and
//! the Nether, as upstream's `IAuraType` says), spends it on one of the three catalyst-free
//! transmutations and puts the result in its one slot. The altar's click is a block Part
//! (`use_item_on`/`use_without_item`), and the `/naturesaura` commands are the port's
//! harness surface. The chunk aura store is a typed attachment on the chunk.
//!
//! The numbers, and every rule that decides aura, live in [`runtime`], which has no
//! `mod_api` type and is unit-tested by `cargo test` alone.

pub mod runtime;

use std::array;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use mod_api::{
    self, AttachmentCodec, AttachmentError, AttachmentType, BlockClick, BlockEntityEvent,
    BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockKey, BlockPart,
    BlockUpdateFlags, Command, CommandArgumentValue, CommandSource, Compound, Flow, Hooks, Host,
    HostTransaction, InteractionResult, ItemKey, ItemPlace, ItemResource, ItemStack, LevelRef,
    LevelSide, ModBlockEntityView, ModContainer, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost, ServerTickEvent, ServerTickPhase, Tag,
    Text, UseItemOnEvent, UseWithoutItemEvent,
};
use runtime::{
    ALTAR_CHECK_TICKS, ALTAR_MAX_AURA, ALTAR_RADIUS, ALTAR_RECIPES, Altar, AltarPart,
    BLOOM_GENERATED_AURA, ChunkAura, DEFAULT_AURA, GENERATE_RADIUS, GRASS_DIE_RADIUS, LEVEL_ADMINS,
    altar_complete, altar_parts, altar_recipe, grass_die, grass_die_replacement, grass_die_targets,
    may_inspect, recipe_at, transfer_is_complete,
};

/// The aura bloom (`BlockAuraBloom("aura_bloom", …)`).
const BLOOM: &str = "naturesaura:aura_bloom";
/// The nature's altar (`BlockNatureAltar`).
const ALTAR: &str = "naturesaura:nature_altar";

/// The block entity types this mod declares, one per block, in its own namespace.
const BLOOM_TYPE: &str = "aura_bloom";
const ALTAR_TYPE: &str = "nature_altar";

/// The altar's one item slot (`ItemStackHandlerNA(1, this, true)`).
const ALTAR_SLOT: u32 = 0;
const SLOT_COUNT: usize = 1;

/// The altar's saved keys (`BlockEntityNatureAltar.writeNBT`, plus the cached multiblock
/// answer, which upstream keeps as `complete`).
const KEY_AURA: &str = "aura";
const KEY_TIMER: &str = "timer";
const KEY_RECIPE: &str = "recipe";
const KEY_COMPLETE: &str = "complete";
/// The bloom's saved flag (`BlockEntityAuraBloom.just_generated`).
const KEY_JUST_GENERATED: &str = "just_generated";
/// `recipe = -1` and `complete = -1` in a fresh block entity: not decided yet.
const UNSET: i32 = -1;

/// The registration keys, so a hot tick compares block identity instead of allocating a
/// string per block read. Registration runs before any hook.
static BLOOM_KEY: OnceLock<BlockKey> = OnceLock::new();
static ALTAR_KEY: OnceLock<BlockKey> = OnceLock::new();
/// The three recipes' output items, resolved once at load: the altar's tick needs an
/// `ItemKey` to put in its slot, while [`runtime`] names items by id only. The assert in
/// `register` keeps this count equal to `ALTAR_RECIPES.len()`.
const RECIPE_COUNT: usize = 3;
static RECIPE_OUTPUTS: OnceLock<[ItemKey; RECIPE_COUNT]> = OnceLock::new();

// ---------------------------------------------------------------------------
// The chunk aura store (a typed attachment)
// ---------------------------------------------------------------------------

/// The drain spots of a chunk, saved as `AuraChunk.serializeNBT` saves them
/// (`{drain_spots: [{pos, amount}]}`), here as `{spots: [{x, y, z, amount}]}` because the
/// package writes NBT compounds, not packed longs.
///
/// `sync: false`: the aura is a server-side fact; the slice has no client HUD that shows it
/// (upstream sends `PacketAuraChunk`; that is a later slice).
static AURA: AttachmentType<ChunkAura> = AttachmentType {
    namespace: "naturesaura",
    name: "aura",
    default: ChunkAura::new,
    codec: Some(AttachmentCodec {
        encode: encode_aura,
        decode: decode_aura,
    }),
    copy_on_death: false,
    sync: false,
};

fn encode_aura(chunk: &ChunkAura) -> Result<Compound, AttachmentError> {
    let spots = chunk
        .spots()
        .map(|(pos, amount)| {
            let mut spot = Compound::new();
            write_int(&mut spot, "x", pos[0]);
            write_int(&mut spot, "y", pos[1]);
            write_int(&mut spot, "z", pos[2]);
            write_int(&mut spot, "amount", amount);
            Tag::Compound(spot)
        })
        .collect();
    let mut out = Compound::new();
    out.insert("spots".into(), Tag::List(spots));
    Ok(out)
}

fn decode_aura(data: &Compound) -> Result<ChunkAura, AttachmentError> {
    let Some(Tag::List(list)) = tag(data, "spots") else {
        return Ok(ChunkAura::new());
    };
    let mut spots = Vec::with_capacity(list.len());
    for entry in list {
        let Tag::Compound(spot) = entry else {
            return Err(AttachmentError::Decode);
        };
        let (Some(x), Some(y), Some(z), Some(amount)) = (
            int_of(spot, "x"),
            int_of(spot, "y"),
            int_of(spot, "z"),
            int_of(spot, "amount"),
        ) else {
            return Err(AttachmentError::Decode);
        };
        spots.push(([x, y, z], amount));
    }
    Ok(ChunkAura::from_spots(spots))
}

/// The chunks a block position and radius touch, in a stable order.
fn chunks_in_radius(center: [i32; 3], radius: i32) -> Vec<[i32; 2]> {
    let (x0, x1) = (center[0] - radius, center[0] + radius);
    let (z0, z1) = (center[2] - radius, center[2] + radius);
    let mut chunks = Vec::new();
    for cx in (x0 >> 4)..=(x1 >> 4) {
        for cz in (z0 >> 4)..=(z1 >> 4) {
            chunks.push([cx, cz]);
        }
    }
    chunks
}

/// The store of one chunk, if it has one. A chunk nobody has written holds no spots, which
/// is different from an empty store only in that reading it must not create a save entry.
fn store_of(host: &mut dyn ServerHost, chunk: [i32; 2]) -> Option<ChunkAura> {
    AURA.get_existing(host.attachments(), mod_api::Holder::Chunk(chunk))
        .ok()
        .flatten()
}

/// The aura of the area: the base [`DEFAULT_AURA`] plus every drain spot within `radius`
/// (`InternalHooks.getAuraInArea`). Chunks that are not loaded hold no spots, so they add
/// nothing.
fn area_aura(host: &mut dyn ServerHost, center: [i32; 3], radius: i32) -> i32 {
    let mut spots = 0i32;
    for chunk in chunks_in_radius(center, radius) {
        let Some(store) = store_of(host, chunk) else {
            continue;
        };
        spots = spots.saturating_add(store.sum_in(center, radius));
    }
    DEFAULT_AURA.saturating_add(spots)
}

/// The area's spot sum and count in one walk, for the readout.
fn area_spots(host: &mut dyn ServerHost, center: [i32; 3], radius: i32) -> (i32, usize) {
    let (mut sum, mut count) = (0i32, 0usize);
    for chunk in chunks_in_radius(center, radius) {
        let Some(store) = store_of(host, chunk) else {
            continue;
        };
        sum = sum.saturating_add(store.sum_in(center, radius));
        count += store.count_in(center, radius);
    }
    (sum, count)
}

/// The fullest spot in radius (`getHighestAuraDrainSpot`), if any holds aura.
fn highest_spot(host: &mut dyn ServerHost, center: [i32; 3], radius: i32) -> Option<[i32; 3]> {
    let mut best: Option<([i32; 3], i32)> = None;
    for chunk in chunks_in_radius(center, radius) {
        let Some(store) = store_of(host, chunk) else {
            continue;
        };
        if let Some(candidate) = store.highest_in(center, radius)
            && best.is_none_or(|(_, amount)| candidate.1 > amount)
        {
            best = Some(candidate);
        }
    }
    best.map(|(pos, _)| pos)
}

/// The most indebted spot in radius (`getLowestAuraDrainSpot`), if any is in debt.
fn lowest_spot(host: &mut dyn ServerHost, center: [i32; 3], radius: i32) -> Option<[i32; 3]> {
    let mut best: Option<([i32; 3], i32)> = None;
    for chunk in chunks_in_radius(center, radius) {
        let Some(store) = store_of(host, chunk) else {
            continue;
        };
        if let Some(candidate) = store.lowest_in(center, radius)
            && best.is_none_or(|(_, amount)| candidate.1 < amount)
        {
            best = Some(candidate);
        }
    }
    best.map(|(pos, _)| pos)
}

/// Drains one spot (`IAuraChunk.drainAura(pos, amount)`), answering what moved.
fn drain_at(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> i32 {
    let chunk = [pos[0] >> 4, pos[2] >> 4];
    let mut store = AURA
        .get(host.attachments(), mod_api::Holder::Chunk(chunk))
        .unwrap_or_default();
    let drained = store.drain(pos, amount, false);
    if drained > 0 {
        let _ = AURA.set(host.attachments(), mod_api::Holder::Chunk(chunk), store);
    }
    drained
}

/// Stores into one spot (`IAuraChunk.storeAura(pos, amount)`), answering what went in.
fn store_at(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> i32 {
    let chunk = [pos[0] >> 4, pos[2] >> 4];
    let mut store = AURA
        .get(host.attachments(), mod_api::Holder::Chunk(chunk))
        .unwrap_or_default();
    let stored = store.store(pos, amount, true);
    if stored > 0 {
        let _ = AURA.set(host.attachments(), mod_api::Holder::Chunk(chunk), store);
    }
    stored
}

/// What one spot holds, for the readout and the harness.
fn amount_at(host: &mut dyn ServerHost, pos: [i32; 3]) -> i32 {
    store_of(host, [pos[0] >> 4, pos[2] >> 4]).map_or(0, |store| store.amount_at(pos))
}

/// `BlockEntityImpl.generateAura`: fill the emptiest spot in radius 35, then the next one,
/// until the aura is placed (a bounded number of spots, as Java's `while` also ends because
/// every store takes what it is given).
fn generate_aura(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) {
    let mut left = amount;
    for _ in 0..8 {
        if left <= 0 {
            break;
        }
        let spot = lowest_spot(host, pos, GENERATE_RADIUS).unwrap_or(pos);
        let stored = store_at(host, spot, left);
        if stored <= 0 {
            break;
        }
        left -= stored;
    }
}

/// Whether the altar may pull aura here: upstream pulls only when the level's aura type is
/// the overworld's or the nether's (`IAuraType.forLevel`).
fn altar_pulls_here(host: &dyn Host, level: LevelRef) -> bool {
    host.level("minecraft:overworld") == Some(level)
        || host.level("minecraft:nether") == Some(level)
}

// ---------------------------------------------------------------------------
// The bloom: a block entity Part
// ---------------------------------------------------------------------------

/// `BlockEntityAuraBloom`: it generates once, the first tick after it appears, and then only
/// sleeps.
struct BloomPart;

#[mod_api::horizon_part]
impl BlockPart for BloomPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for BloomPart {
    /// A fresh bloom has not generated yet (`BlockEntityAuraBloom.just_generated = true`).
    fn block_entity(&self, _: LevelSide, _: &mut BlockEntityEvent<'_>) -> Option<Compound> {
        let mut nbt = Compound::new();
        write_int(&mut nbt, KEY_JUST_GENERATED, 1);
        Some(nbt)
    }

    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if read_int(&event.entity.nbt, KEY_JUST_GENERATED) == 0 {
            // Nothing of this Part's payload changed: do not dirty the chunk every tick.
            *event.reply = mod_api::BlockEntityTickReply::Unchanged;
            return;
        }
        let pos = event.pos;
        write_int(&mut event.entity.nbt, KEY_JUST_GENERATED, 0);
        generate_aura(host, pos, BLOOM_GENERATED_AURA);
    }
}

// ---------------------------------------------------------------------------
// The altar: a block entity Part and a click Part
// ---------------------------------------------------------------------------

/// `BlockEntityNatureAltar`: the multiblock check and the ritual.
struct AltarEntityPart;

#[mod_api::horizon_part]
impl BlockPart for AltarEntityPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for AltarEntityPart {
    /// A fresh altar holds no aura, runs no recipe, and has not decided its multiblock.
    fn block_entity(&self, _: LevelSide, _: &mut BlockEntityEvent<'_>) -> Option<Compound> {
        let mut nbt = Compound::new();
        write_int(&mut nbt, KEY_AURA, 0);
        write_int(&mut nbt, KEY_TIMER, 0);
        write_int(&mut nbt, KEY_RECIPE, UNSET);
        write_int(&mut nbt, KEY_COMPLETE, UNSET);
        Some(nbt)
    }

    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = level_in_place(host) else {
            *event.reply = mod_api::BlockEntityTickReply::Unchanged;
            return;
        };
        let pos = event.pos;
        altar_tick(host, level, pos, &mut event.entity);
    }
}

/// `Helper.putStackOnTile` and the altar's own handler rules: a right click puts one recipe
/// input on the altar, or takes a finished product off it.
struct AltarClickPart;

#[mod_api::horizon_part]
impl BlockPart for AltarClickPart {
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        altar_click(host, event.click, event.click.stack)
    }

    fn use_without_item(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseWithoutItemEvent<'_>,
    ) -> Option<InteractionResult> {
        altar_click(host, event.click, None)
    }
}

/// One server tick of the altar (`BlockEntityNatureAltar.tick`, minus the aura-container
/// item branch, which needs items this slice does not carry).
fn altar_tick(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    entity: &mut ModBlockEntityView<'_>,
) {
    if entity.items.len() < SLOT_COUNT {
        entity.items.resize(SLOT_COUNT, None);
    }
    let mut altar = Altar::new();
    altar.aura = read_int(&entity.nbt, KEY_AURA).clamp(0, ALTAR_MAX_AURA);
    altar.timer = read_int(&entity.nbt, KEY_TIMER).max(0);
    // A saved index the table does not have is dropped here: `Altar::step` indexes
    // `ALTAR_RECIPES` directly (`recipe_at` checks the bounds).
    altar.recipe = match read_int(&entity.nbt, KEY_RECIPE) {
        UNSET => None,
        index if index >= 0 => recipe_at(index as usize).map(|_| index as usize),
        _ => None,
    };
    let cached = read_int(&entity.nbt, KEY_COMPLETE);
    let complete = if cached == UNSET || level.game_time(host).unwrap_or(0) % ALTAR_CHECK_TICKS == 0
    {
        let answer = altar_complete(pos, |at| block_id(host, level, at).unwrap_or_default());
        write_int(&mut entity.nbt, KEY_COMPLETE, i32::from(answer));
        answer
    } else {
        cached == 1
    };

    let mut crafted = false;
    if complete {
        let room = altar.pull_room();
        if room > 0 && altar_pulls_here(host, level) {
            // `toStore = min(area aura, room)`: a drained area can answer a negative amount,
            // and then nothing is pulled.
            let area = area_aura(host, pos, ALTAR_RADIUS);
            let to_store = area.min(room);
            if to_store > 0 {
                let spot = highest_spot(host, pos, ALTAR_RADIUS).unwrap_or(pos);
                altar.take_pulled(drain_at(host, spot, to_store));
            }
        }
        let slot = entity.items[ALTAR_SLOT as usize]
            .as_ref()
            .map(|stack| stack.item().id(host));
        if let Some(recipe) = altar.step(slot.as_deref())
            && let Some(index) = altar_recipe(recipe.input)
            && let Some(stack) =
                recipe_output(index).and_then(|key| ItemStack::new(key, recipe.count))
        {
            entity.items[ALTAR_SLOT as usize] = Some(stack);
            crafted = true;
        }
    }

    write_int(&mut entity.nbt, KEY_AURA, altar.aura);
    write_int(&mut entity.nbt, KEY_TIMER, altar.timer);
    write_int(
        &mut entity.nbt,
        KEY_RECIPE,
        altar.recipe.map_or(UNSET, |index| index as i32),
    );
    if crafted {
        host.block_entity_sync(pos);
    }
}

/// The item a recipe's output slot should hold, resolved at registration.
fn recipe_output(index: usize) -> Option<ItemKey> {
    RECIPE_OUTPUTS
        .get()
        .and_then(|outputs| outputs.get(index))
        .copied()
}

/// The altar's slot at `pos`, if that block entity is one.
fn altar_slot(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<ItemStack> {
    if !is_block(host, level, pos, &ALTAR_KEY) {
        return None;
    }
    host.block_entity(level, pos)?
        .item_slot(host, ALTAR_SLOT)
        .ok()
        .flatten()
}

/// A right click on the altar: insert one input, or take a finished product.
///
/// Upstream's handler refuses an input through `canExtract`, and the game asks a click for
/// each hand: taking only a **finished product** by hand keeps a second ask from undoing the
/// insert. Automation may still take an input; that is a recorded gap in `notes.md`.
fn altar_click(
    host: &mut dyn ServerHost,
    click: &BlockClick<'_>,
    held: Option<&ItemStack>,
) -> Option<InteractionResult> {
    let level = host.level(click.dimension.key().id())?;
    let pos = click.pos;
    if !is_block(host, level, pos, &ALTAR_KEY) {
        return None;
    }
    let hand = ItemPlace::PlayerHand {
        player: click.player,
        hand: click.hand,
    };
    if let Some(stack) = held
        && stack.count() > 0
        && altar_recipe(&stack.item().id(host)).is_some()
        && altar_slot(host, level, pos).is_none()
    {
        let one = ItemStack::of(stack.item());
        let resource = ItemResource::of(stack);
        let placed = host
            .transaction(|host, tx| {
                let entity = host.block_entity(level, pos).ok_or(())?;
                entity
                    .set_item_slot(host, tx, ALTAR_SLOT, Some(one))
                    .map_err(|_| ())?;
                // A creative player's hand gives without losing any.
                let _ = hand.extract(host, tx, &resource, 1);
                Ok::<(), ()>(())
            })
            .is_ok();
        if placed {
            return Some(InteractionResult::Success);
        }
    }
    let stack = altar_slot(host, level, pos);
    let Some(stack) = stack else {
        // The altar consumes the click whether or not it had something to give, as
        // 0.1.0's unconditional cancel did.
        return Some(InteractionResult::Success);
    };
    if altar_recipe(&stack.item().id(host)).is_some() {
        return Some(InteractionResult::Success);
    }
    let resource = ItemResource::of(&stack);
    let count = stack.count();
    // Fill the hand first and clear the slot only after a full transfer, in one
    // transaction: an incompatible occupied hand inserts nothing, and a partial
    // insert aborts the transaction (the journal rolls it back), so the finished
    // output is never destroyed. The review finding: the old order cleared the
    // slot and then ignored how much the hand took.
    let _ = host.transaction(|host, tx| {
        let inserted = hand.insert(host, tx, &resource, count);
        if !transfer_is_complete(count, inserted) {
            return Err(());
        }
        let entity = host.block_entity(level, pos).ok_or(())?;
        entity
            .set_item_slot(host, tx, ALTAR_SLOT, None)
            .map_err(|_| ())?;
        Ok::<(), ()>(())
    });
    Some(InteractionResult::Success)
}

// ---------------------------------------------------------------------------
// The drained-spot passive effect: grass death (`AuraChunk.update`)
// ---------------------------------------------------------------------------

/// `AuraChunk.update()` runs once a second at 20 tps.
const EFFECT_TICKS: u64 = 20;

/// How far around an online player the port looks for loaded chunks with spots.
/// Upstream walks `LevelData.auraChunksWithSpots` (every loaded chunk that holds
/// a spot); the public SDK exposes loaded chunks via `Host::chunk`, so the port
/// scans the chunk window a player keeps loaded. Eight chunks is 128 blocks,
/// wider than the effect's own 75-block scatter and 50-block area radius.
const EFFECT_CHUNKS: i32 = 8;

/// One passing second: apply the grass-die effect at every drained spot the port
/// can see. Upstream walks `LevelData.auraChunksWithSpots` and calls each
/// effect's `update` per spot; here the spots live in chunk attachments and a
/// loaded chunk is one a player keeps in memory.
fn grass_die_second(host: &mut dyn ServerHost) {
    let players = host.players().server_players();
    for level in host.levels().list_levels() {
        // `GrassDieEffect.appliesHere`: overworld or nether only.
        if host.level("minecraft:overworld") != Some(level)
            && host.level("minecraft:nether") != Some(level)
        {
            continue;
        }
        let centers = players
            .iter()
            .filter(|player| player.level(host) == Some(level))
            .filter_map(|player| player.entity().position(host))
            .map(|at| [at[0] as i32 >> 4, at[2] as i32 >> 4])
            .collect::<Vec<_>>();
        for [px, pz] in centers {
            for cx in (px - EFFECT_CHUNKS)..=(px + EFFECT_CHUNKS) {
                for cz in (pz - EFFECT_CHUNKS)..=(pz + EFFECT_CHUNKS) {
                    let chunk = [cx, cz];
                    if host.chunk(level, chunk).is_none() {
                        continue;
                    }
                    let Some(store) = store_of(host, chunk) else {
                        continue;
                    };
                    for (pos, amount) in store.spots().collect::<Vec<_>>() {
                        grass_die_at(host, level, pos, amount);
                    }
                }
            }
        }
    }
}

/// One spot's grass-die pass. The threshold, the target choice and the block
/// mapping live in [`runtime`]; this function only reads and writes blocks.
fn grass_die_at(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], amount: i32) {
    let (sum, count) = area_spots(host, pos, GRASS_DIE_RADIUS);
    let area = DEFAULT_AURA.saturating_add(sum);
    let Some(params) = grass_die(amount, area, count as i32) else {
        return;
    };
    for target in grass_die_targets(pos, &params) {
        let Some(id) = block_id(host, level, target) else {
            continue;
        };
        let Some(replacement) = grass_die_replacement(&id) else {
            continue;
        };
        let Some(state) = host.block_state_id(replacement) else {
            continue;
        };
        let _ = level.set_block(
            host,
            target,
            state,
            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
        );
    }
}

// ---------------------------------------------------------------------------
// The one impl Hooks
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Mod {
    /// Running count of authoritative ticks, advanced once per running tick.
    tick: AtomicU64,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// `AuraChunk.update()`: once a running second, apply the drained spots' passive
    /// effect. A frozen step does not advance the world and does nothing.
    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        let ServerTickPhase::ServerTickStart {
            world_running: true,
        } = event.phase
        else {
            return;
        };
        let tick = self.tick.fetch_add(1, Ordering::Relaxed) + 1;
        if tick.is_multiple_of(EFFECT_TICKS) {
            grass_die_second(host);
        }
    }

    /// Declare the two block entity types (each held by its own block), their Parts, the
    /// altar's one item slot, the click Part and the recipe outputs.
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        assert_eq!(
            ALTAR_RECIPES.len(),
            RECIPE_COUNT,
            "the recipe table changed: RECIPE_COUNT must follow it"
        );
        let bloom = r.block_key(BLOOM);
        let altar = r.block_key(ALTAR);
        let _ = BLOOM_KEY.set(bloom);
        let _ = ALTAR_KEY.set(altar);
        let _ = RECIPE_OUTPUTS.set(array::from_fn(|index| {
            r.item_key(ALTAR_RECIPES[index].output)
        }));

        let bloom_type = r.block_entity_type(BLOOM_TYPE).blocks(&[BLOOM]).build();
        let _ = r.block_entity_part(&bloom_type, BloomPart);

        let altar_type = r.block_entity_type(ALTAR_TYPE).blocks(&[ALTAR]).build();
        let _ = r.block_entity_part(&altar_type, AltarEntityPart);

        r.block_container(
            &altar,
            ModContainer {
                slots: SLOT_COUNT,
                faces: array::from_fn(|_| vec![ALTAR_SLOT as usize]),
                inputs: vec![ALTAR_SLOT as usize],
                outputs: vec![ALTAR_SLOT as usize],
                menu: None,
            },
        );
        r.block_part(altar, AltarClickPart);
    }

    /// `/naturesaura aura|spot|altar|altar-build|altar-aura|drain` (upstream's own command
    /// manages aura; these read the slice's state and give the harness the same reach).
    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::new(
            "naturesaura aura <pos:block_pos> [radius:integer]",
        ));
        event
            .commands
            .push(Command::new("naturesaura spot <pos:block_pos>"));
        event
            .commands
            .push(Command::new("naturesaura altar <pos:block_pos>"));
        event
            .commands
            .push(Command::op("naturesaura altar-build <pos:block_pos>", 2));
        event.commands.push(Command::op(
            "naturesaura altar-aura <pos:block_pos> <amount:integer>",
            2,
        ));
        event.commands.push(Command::op(
            "naturesaura drain <pos:block_pos> <amount:integer>",
            2,
        ));
    }

    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let invocation = event.command;
        let Some(CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos") else {
            feedback(
                host,
                "usage: /naturesaura aura|spot|altar|altar-build|drain ...",
            );
            return Flow::Cancel;
        };
        let pos = *pos;
        match invocation.literals.get(1).map(String::as_str) {
            Some("aura") => {
                let radius = match invocation.argument("radius") {
                    Some(CommandArgumentValue::Integer(radius)) => (*radius).clamp(0, 128),
                    _ => ALTAR_RADIUS,
                };
                let (spots, count) = area_spots(host, pos, radius);
                let area = DEFAULT_AURA.saturating_add(spots);
                feedback(
                    host,
                    format!(
                        "Nature's Aura at {}, {}, {} (radius {radius}): area={area} base={DEFAULT_AURA} spots={spots} count={count}",
                        pos[0], pos[1], pos[2]
                    ),
                );
                Flow::Cancel
            }
            Some("spot") => {
                let amount = amount_at(host, pos);
                feedback(
                    host,
                    format!("Aura spot at {}, {}, {}: {amount}", pos[0], pos[1], pos[2]),
                );
                Flow::Cancel
            }
            Some("altar") => {
                // The readout names the altar's item and count: gate it like a
                // reach, so another player's altar is not read from afar.
                let Some(level) = level_of(host, invocation) else {
                    feedback(host, format!("{pos:?} is not a mod block entity."));
                    return Flow::Cancel;
                };
                if !may_read(&*host, event.source, level, pos) {
                    feedback(
                        host,
                        format!(
                            "You must be an operator or within {:.0} blocks of the altar.",
                            runtime::INSPECT_RANGE
                        ),
                    );
                    return Flow::Cancel;
                }
                let line = altar_readout(host, invocation, pos);
                feedback(host, line);
                Flow::Cancel
            }
            Some("altar-build") => {
                let placed = build_altar(host, invocation, pos);
                let (spots, _) = area_spots(host, pos, ALTAR_RADIUS);
                feedback(
                    host,
                    format!(
                        "Nature's Altar frame around {}, {}, {}: placed {placed} blocks; area={}",
                        pos[0],
                        pos[1],
                        pos[2],
                        DEFAULT_AURA.saturating_add(spots)
                    ),
                );
                Flow::Cancel
            }
            Some("altar-aura") => {
                let Some(CommandArgumentValue::Integer(amount)) = invocation.argument("amount")
                else {
                    feedback(
                        host,
                        "usage: /naturesaura altar-aura <x> <y> <z> <amount>".to_owned(),
                    );
                    return Flow::Cancel;
                };
                let amount = (*amount).clamp(0, ALTAR_MAX_AURA);
                let level = level_of(host, invocation)
                    .filter(|level| altar_block_there(&*host, *level, pos));
                let line = match level {
                    Some(level) => {
                        let mut data = Compound::new();
                        write_int(&mut data, KEY_AURA, amount);
                        if merge_data(host, level, pos, data) {
                            format!(
                                "Nature's Altar at {}, {}, {}: the container holds {amount}/{ALTAR_MAX_AURA} aura.",
                                pos[0], pos[1], pos[2]
                            )
                        } else {
                            format!("{pos:?} is not a nature's altar.")
                        }
                    }
                    None => format!("{pos:?} is not a nature's altar."),
                };
                feedback(host, line);
                Flow::Cancel
            }
            Some("drain") => {
                let Some(CommandArgumentValue::Integer(amount)) = invocation.argument("amount")
                else {
                    feedback(
                        host,
                        "usage: /naturesaura drain <x> <y> <z> <amount>".to_owned(),
                    );
                    return Flow::Cancel;
                };
                let drained = drain_at(host, pos, (*amount).max(0));
                let left = amount_at(host, pos);
                let (spots, _) = area_spots(host, pos, ALTAR_RADIUS);
                feedback(
                    host,
                    format!(
                        "Aura spot at {}, {}, {}: drained {drained}, {left} left; area={}",
                        pos[0],
                        pos[1],
                        pos[2],
                        DEFAULT_AURA.saturating_add(spots)
                    ),
                );
                Flow::Cancel
            }
            _ => {
                feedback(
                    host,
                    "usage: /naturesaura aura|spot|altar|altar-build|drain ...",
                );
                Flow::Cancel
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The altar's readout and the harness's build helper
// ---------------------------------------------------------------------------

/// Whether this command source may read the altar at `pos`: an operator, or a
/// player in the same level within [`runtime::INSPECT_RANGE`] of the block.
fn may_read(host: &dyn ServerHost, source: &CommandSource, level: LevelRef, pos: [i32; 3]) -> bool {
    let op = source.permission().level() >= LEVEL_ADMINS;
    match source {
        CommandSource::Player { player, .. } => {
            let same_level = player.level(host) == Some(level);
            let distance = player
                .entity()
                .position(host)
                .map(|at| {
                    let block = [
                        f64::from(pos[0]) + 0.5,
                        f64::from(pos[1]) + 0.5,
                        f64::from(pos[2]) + 0.5,
                    ];
                    (0..3)
                        .map(|axis| (at[axis] - block[axis]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                })
                .unwrap_or(f64::MAX);
            may_inspect(op, same_level, distance)
        }
        _ => op,
    }
}

/// The altar's state as `/naturesaura altar` prints it: whether its multiblock is complete,
/// its container, its recipe, its slot and the aura of the area.
fn altar_readout(
    host: &mut dyn ServerHost,
    invocation: &mod_api::CommandInvocation,
    pos: [i32; 3],
) -> String {
    let Some(level) = level_of(host, invocation) else {
        return format!("{pos:?} is not a mod block entity.");
    };
    if host.block_entity(level, pos).is_none() {
        return format!("{pos:?} is not a mod block entity.");
    }
    if !is_block(host, level, pos, &ALTAR_KEY) {
        let found = block_id(host, level, pos).unwrap_or_default();
        return format!("{pos:?} is a {found} not a nature's altar.");
    }
    let Some(nbt) = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(host))
    else {
        return format!("{pos:?} is not a mod block entity.");
    };
    let aura = read_int(&nbt, KEY_AURA).clamp(0, ALTAR_MAX_AURA);
    let timer = read_int(&nbt, KEY_TIMER).max(0);
    let recipe = match read_int(&nbt, KEY_RECIPE) {
        UNSET => "none".to_owned(),
        index if index >= 0 => ALTAR_RECIPES
            .get(index as usize)
            .map_or_else(|| "none".to_owned(), |recipe| recipe.input.to_owned()),
        _ => "none".to_owned(),
    };
    let slot = match altar_slot(host, level, pos) {
        Some(stack) => format!("{} x{}", stack.item().id(host), stack.count()),
        None => "empty".to_owned(),
    };
    let complete = read_int(&nbt, KEY_COMPLETE) == 1;
    let (spots, _) = area_spots(host, pos, ALTAR_RADIUS);
    format!(
        "Nature's Altar at {}, {}, {}: complete={} aura={aura}/{ALTAR_MAX_AURA} timer={timer} recipe={recipe} slot={slot} area={}",
        pos[0],
        pos[1],
        pos[2],
        i32::from(complete),
        DEFAULT_AURA.saturating_add(spots)
    )
}

/// Whether `pos` holds a nature's altar whose block entity exists (the command's guard).
fn altar_block_there(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    is_block(host, level, pos, &ALTAR_KEY) && host.block_entity(level, pos).is_some()
}

/// `/naturesaura altar-build` (harness only): place every matched block of the altar
/// multiblock around `pos`, but not the altar itself, so a scenario can build the structure
/// the way a player would without 48 typed setblocks.
fn build_altar(
    host: &mut dyn ServerHost,
    invocation: &mod_api::CommandInvocation,
    pos: [i32; 3],
) -> usize {
    let Some(level) = level_of(host, invocation) else {
        return 0;
    };
    let mut placed = 0;
    for (offset, part) in altar_parts() {
        let Some(id) = multiblock_block(part) else {
            continue;
        };
        let at = [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]];
        let Some(state) = host.block_state_id(id) else {
            continue;
        };
        if level.set_block(
            host,
            at,
            state,
            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
        ) == Ok(true)
        {
            placed += 1;
        }
    }
    placed
}

/// The block a part of the altar is built from: the first id each jar tag lists.
fn multiblock_block(part: AltarPart) -> Option<&'static str> {
    match part {
        AltarPart::Altar => None,
        AltarPart::Stone => Some("minecraft:stone_bricks"),
        AltarPart::Wood => Some("minecraft:oak_planks"),
        AltarPart::Fancy => Some("minecraft:red_nether_bricks"),
        AltarPart::Gold => Some("naturesaura:gold_brick"),
    }
}

// ---------------------------------------------------------------------------
// Level and command helpers
// ---------------------------------------------------------------------------

/// The level a command executes in, from the dimension its invocation named.
fn level_of(host: &dyn ServerHost, invocation: &mod_api::CommandInvocation) -> Option<LevelRef> {
    invocation.dimension.id(host).and_then(|id| host.level(&id))
}

/// The level whose block entities the server ticks now. A block-entity tick is not told its
/// level; the level in place is the one being ticked.
fn level_in_place(host: &dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

/// Whether `pos` holds the block registered under `key`.
fn is_block(host: &dyn Host, level: LevelRef, pos: [i32; 3], key: &OnceLock<BlockKey>) -> bool {
    let Some(found) = level.block(host, pos) else {
        return false;
    };
    key.get().is_some_and(|wanted| *wanted == found)
}

fn block_id(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    level.block(host, pos).map(|key| key.id(host))
}

/// Merge `data` into the block entity at `pos` (`BlockDataAccessor.setData`: load,
/// `setChanged`, and a client update). Answers whether it changed.
fn merge_data(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], data: Compound) -> bool {
    host.block_entity(level, pos)
        .is_some_and(|entity| entity.merge_data(host, data).unwrap_or(false))
}

fn feedback(host: &mut dyn ServerHost, line: impl Into<String>) {
    let _ = host.commands().feedback(Text::literal(line.into()));
}

// ---------------------------------------------------------------------------
// Saved-compound reads
// ---------------------------------------------------------------------------

/// `key`'s tag, with Java's UTF-16 key spelling.
fn tag<'a>(nbt: &'a Compound, key: &str) -> Option<&'a Tag> {
    let units: Vec<u16> = key.encode_utf16().collect();
    nbt.iter()
        .find(|(name, _)| name.0 == units)
        .map(|(_, tag)| tag)
}

fn int_of(compound: &Compound, key: &str) -> Option<i32> {
    match tag(compound, key) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    int_of(nbt, key).unwrap_or(0)
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}
