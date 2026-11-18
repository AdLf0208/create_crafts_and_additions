//! Game glue for the FE half: the block entities, the wire links, the per-level network pass,
//! the spool click and the `/createaddition` commands.
//!
//! Upstream (`FINDINGS.md`) keeps one `EnergyNetwork` per wire graph in memory and rebuilds it
//! on load; this port books the same double buffer in the saved compound of the network's
//! lowest-positioned connector, and every move goes through the engine's `ENERGY` capability, so
//! no energy is created. The rules are [`crate::runtime`]'s, byte-identical to the retired
//! bundle's `runtime.rs`; this file only reads the world, calls them and writes the result back.
//!
//! A block entity is out of the world during its own tick, so its tick only **queues** its
//! position: the whole pass runs once at `level_tick_post`, over the blocks whose tick ran in
//! that level this tick. The SDK cannot enumerate a level's block entities, which is why the
//! queue exists at all.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use mod_api::{
  self, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockKey, BlockPart,
  CommandArgumentValue, CommandInvocation, Compound, Direction, ENERGY, Host, HostTransaction,
  InteractionResult, ItemKey, ItemPlace, ItemResource, ItemStack, LevelRef, ModEnergyBuffer,
  ServerHost, StateForPlacementEvent, Tag, Text, UseItemOnEvent, move_energy,
};

use crate::runtime::{
  ACCUMULATOR_CAPACITY, ACCUMULATOR_TRANSFER, Connector, End, Link, Mode, Pool, Refusal, Wire,
  buffer_capacity, link as plan_link, placed_mode, pull_amount, push_amount,
};

/// The small connector block.
pub(crate) const CONNECTOR: &str = "createaddition:connector";
/// The large connector block.
pub(crate) const LARGE_CONNECTOR: &str = "createaddition:large_connector";
/// The modular accumulator block.
pub(crate) const ACCUMULATOR: &str = "createaddition:modular_accumulator";
/// The empty spool: right-clicking a linked connector with it takes the wire out.
const SPOOL: &str = "createaddition:spool";
/// The three wire spools, each naming the wire it lays.
const COPPER_SPOOL: &str = "createaddition:copper_spool";
const GOLD_SPOOL: &str = "createaddition:gold_spool";
const ELECTRUM_SPOOL: &str = "createaddition:electrum_spool";
/// The item component a spool's pending selection is saved in, and the key inside it.
const PENDING: &str = "minecraft:custom_data";
const NODE: &str = "createaddition";
/// The block entity's saved charge: the game's own buffer keeps its amount here
/// (`ModEnergyBuffer`'s doc), so the readout and the retired bundle's saves agree.
const CHARGE: &str = "energy";
/// The block property a connector's mounting face is saved in.
const FACING: &str = "facing";
/// The block property its mode is shown in (`none`/`pull`/`push`).
const MODE: &str = "mode";
/// One network's saved buffer, in its controller's compound (0.1.0's keys, kept).
const NET_READY: &str = "netReady";
const NET_NEXT: &str = "netNext";
const NET_DEMAND: &str = "netDemand";
/// The saved wire list of a connector.
const LINKS: &str = "links";

/// A block position.
pub(crate) type Pos = [i32; 3];

/// The three blocks this port owns, each with the energy buffer it declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
  /// `createaddition:connector`: 1,000 FE/t either way, a 1,000 FE window.
  Small,
  /// `createaddition:large_connector`: 5,000 FE/t either way.
  Large,
  /// `createaddition:modular_accumulator`: 2,000,000 FE, 5,000 FE/t either way.
  Accumulator,
}

impl Kind {
  /// The connector this block is, if it is one.
  fn connector(self) -> Option<Connector> {
    match self {
      Self::Small => Some(Connector::Small),
      Self::Large => Some(Connector::Large),
      Self::Accumulator => None,
    }
  }

  /// How the readout names the block.
  fn label(self) -> &'static str {
    match self {
      Self::Small => "Connector",
      Self::Large => "Large Connector",
      Self::Accumulator => "Modular Accumulator",
    }
  }

  /// The energy buffer the block declares (`Registrar::block_energy`): upstream's window for
  /// the two connectors and the accumulator's own capacity, reachable from every side.
  pub(crate) fn buffer(self) -> ModEnergyBuffer {
    let (capacity, max_insert, max_extract) = match self.connector() {
      Some(connector) => (
        connector.max_in().min(connector.max_out()),
        connector.max_in(),
        connector.max_out(),
      ),
      None => (
        ACCUMULATOR_CAPACITY,
        ACCUMULATOR_TRANSFER,
        ACCUMULATOR_TRANSFER,
      ),
    };
    ModEnergyBuffer {
      capacity,
      max_insert,
      max_extract,
      insert_faces: [true; 6],
      extract_faces: [true; 6],
    }
  }

  /// How much FE the block holds when full.
  fn capacity(self) -> i32 {
    self.buffer().capacity
  }
}

/// The registry keys this port resolves once per pass: the three blocks and the four spools.
#[derive(Clone, Copy)]
struct Keys {
  small: BlockKey,
  large: BlockKey,
  accumulator: BlockKey,
  spool: ItemKey,
  copper: ItemKey,
  gold: ItemKey,
  electrum: ItemKey,
}

impl Keys {
  /// Resolve every key, or `None` while the registry lacks one of them.
  fn resolve(host: &dyn Host) -> Option<Keys> {
    let item = |id: &str| {
      let identifier = mod_api::Identifier::parse(id).ok()?;
      host.item_key(&identifier)
    };
    Some(Keys {
      small: host.block_key(CONNECTOR)?,
      large: host.block_key(LARGE_CONNECTOR)?,
      accumulator: host.block_key(ACCUMULATOR)?,
      spool: item(SPOOL)?,
      copper: item(COPPER_SPOOL)?,
      gold: item(GOLD_SPOOL)?,
      electrum: item(ELECTRUM_SPOOL)?,
    })
  }

  /// Which of the three blocks `block` is.
  fn kind(self, block: BlockKey) -> Option<Kind> {
    if block == self.small {
      Some(Kind::Small)
    } else if block == self.large {
      Some(Kind::Large)
    } else if block == self.accumulator {
      Some(Kind::Accumulator)
    } else {
      None
    }
  }

  /// The wire a held spool lays, or `None` for another item.
  fn wire(self, item: ItemKey) -> Option<Wire> {
    if item == self.copper {
      Some(Wire::Copper)
    } else if item == self.gold {
      Some(Wire::Gold)
    } else if item == self.electrum {
      Some(Wire::Electrum)
    } else {
      None
    }
  }

  /// Whether a held stack is the empty spool.
  fn is_empty_spool(self, item: ItemKey) -> bool {
    item == self.spool
  }
}

// --- NBT ------------------------------------------------------------------

fn int_at(nbt: &Compound, name: &str) -> i32 {
  match nbt.get(&name.into()) {
    Some(Tag::Int(value)) => *value,
    _ => 0,
  }
}

fn put_int(nbt: &mut Compound, name: &str, value: i32) {
  nbt.insert(name.into(), Tag::Int(value));
}

fn name_at(nbt: &Compound, name: &str) -> Option<String> {
  match nbt.get(&name.into()) {
    Some(Tag::String(value)) => Some(value.to_string_lossy()),
    _ => None,
  }
}

fn put_name(nbt: &mut Compound, name: &str, value: &str) {
  nbt.insert(name.into(), Tag::String(value.into()));
}

/// A connector's saved mode, when it has one yet.
fn mode_saved(nbt: &Compound) -> Option<Mode> {
  name_at(nbt, MODE).and_then(|name| Mode::of_name(&name))
}

// --- the blocks -----------------------------------------------------------

/// One saved wire: the other connector, the node it sits in, and the wire kind.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SavedWire {
  pos: Pos,
  node: i32,
  wire: Wire,
}

fn saved_wires(nbt: &Compound) -> Vec<SavedWire> {
  let Some(Tag::List(entries)) = nbt.get(&LINKS.into()) else {
    return Vec::new();
  };
  entries
    .iter()
    .filter_map(|tag| {
      let Tag::Compound(entry) = tag else {
        return None;
      };
      let pos = [int_at(entry, "x"), int_at(entry, "y"), int_at(entry, "z")];
      let wire = name_at(entry, "type").and_then(|name| Wire::of_name(&name))?;
      Some(SavedWire {
        pos,
        node: int_at(entry, "node"),
        wire,
      })
    })
    .collect()
}

fn saved_wire_tag(wire: &SavedWire) -> Tag {
  let mut entry = Compound::new();
  put_int(&mut entry, "x", wire.pos[0]);
  put_int(&mut entry, "y", wire.pos[1]);
  put_int(&mut entry, "z", wire.pos[2]);
  put_int(&mut entry, "node", wire.node);
  put_name(&mut entry, "type", wire.wire.name());
  Tag::Compound(entry)
}

fn store_wires(nbt: &mut Compound, wires: &[SavedWire]) {
  nbt.insert(
    LINKS.into(),
    Tag::List(wires.iter().map(saved_wire_tag).collect()),
  );
}

/// The first node of a connector that has no wire in it.
fn free_node(nbt: &Compound, connector: Connector) -> Option<i32> {
  let used: BTreeSet<i32> = saved_wires(nbt).iter().map(|wire| wire.node).collect();
  (0..i32::try_from(connector.nodes()).ok()?).find(|node| !used.contains(node))
}

/// What a block of ours holds at `pos`: its kind and its saved compound.
#[derive(Clone, Debug)]
struct Saved {
  kind: Kind,
  nbt: Compound,
}

/// One of our blocks at `pos`, or `None` for another block, an unloaded position or a block
/// without its entity.
fn saved_at(host: &dyn Host, level: LevelRef, keys: Keys, pos: Pos) -> Option<Saved> {
  let kind = keys.kind(level.block(host, pos)?)?;
  let nbt = host.block_entity(level, pos)?.data(host)?;
  Some(Saved { kind, nbt })
}

/// What a connector knows at the start of a pass.
#[derive(Clone, Debug)]
struct Node {
  pos: Pos,
  connector: Connector,
  mode: Mode,
  wires: Vec<SavedWire>,
}

fn node_at(host: &dyn Host, level: LevelRef, keys: Keys, pos: Pos) -> Option<Node> {
  let saved = saved_at(host, level, keys, pos)?;
  Some(Node {
    pos,
    connector: saved.kind.connector()?,
    mode: mode_saved(&saved.nbt).unwrap_or(Mode::None),
    wires: saved_wires(&saved.nbt),
  })
}

// --- geometry -------------------------------------------------------------

fn offset(pos: Pos, side: Direction) -> Pos {
  let [dx, dy, dz] = side.normal();
  [pos[0] + dx, pos[1] + dy, pos[2] + dz]
}

/// The face a block property value names.
fn face_named(name: &str) -> Option<Direction> {
  match name {
    "down" => Some(Direction::Down),
    "up" => Some(Direction::Up),
    "north" => Some(Direction::North),
    "south" => Some(Direction::South),
    "west" => Some(Direction::West),
    "east" => Some(Direction::East),
    _ => None,
  }
}

/// The block property value a face is saved as.
fn face_name(face: Direction) -> &'static str {
  match face {
    Direction::Down => "down",
    Direction::Up => "up",
    Direction::North => "north",
    Direction::South => "south",
    Direction::West => "west",
    Direction::East => "east",
  }
}

/// The block a connector is mounted on, and the side of that block which faces the connector.
fn mounted(host: &dyn Host, level: LevelRef, pos: Pos) -> Option<(Pos, Direction)> {
  let facing = level.block_property(host, pos, FACING)?;
  let facing = face_named(&facing)?;
  Some((offset(pos, facing), facing.opposite()))
}

/// The level the server ticks now: the one a block entity's tick runs in.
fn level_in_place(host: &dyn ServerHost) -> Option<LevelRef> {
  host
    .levels()
    .list_levels()
    .into_iter()
    .find(|&level| level.in_place(host))
}

// --- the Parts ------------------------------------------------------------

/// Positions whose block entity ticked this tick, per level, drained at that level's tick end.
#[derive(Default)]
pub(crate) struct Queue {
  pending: HashMap<LevelRef, BTreeSet<Pos>>,
}

impl Queue {
  /// Queue `pos` for the level the tick ran in.
  fn push(&mut self, level: LevelRef, pos: Pos) {
    self.pending.entry(level).or_default().insert(pos);
  }

  /// Everything queued for `level`, in position order; empty afterwards.
  pub(crate) fn take(&mut self, level: LevelRef) -> BTreeSet<Pos> {
    self.pending.remove(&level).unwrap_or_default()
  }
}

/// The one block entity type of the three blocks: it makes them tick, and its tick only queues
/// the position for the level's pass.
pub(crate) struct Ticker {
  queue: Arc<Mutex<Queue>>,
}

impl Ticker {
  pub(crate) fn new(queue: Arc<Mutex<Queue>>) -> Self {
    Self { queue }
  }
}

#[mod_api::horizon_part]
impl BlockPart for Ticker {}

#[mod_api::horizon_part]
impl BlockEntityPart for Ticker {
  /// Our block entities tick on the server.
  fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
    event.ticks = true;
  }

  /// One tick: queue the position for the level's pass. A block entity is out of the world
  /// during its own tick, so nothing is read or moved here.
  fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let Some(level) = level_in_place(host) else {
      return;
    };
    self
      .queue
      .lock()
      .expect("the createaddition tick queue lock is held for one insert")
      .push(level, event.pos);
  }
}

/// The part both connectors carry: where a placed one is mounted, and what a spool does with it.
pub(crate) struct ConnectorShape;

#[mod_api::horizon_part]
impl BlockPart for ConnectorShape {
  /// A placed connector is mounted on the face it was clicked against, as upstream's
  /// `getClickedFace().getOpposite()` has it.
  fn state_for_placement(
    &self,
    level: &dyn mod_api::LevelHost,
    event: &mut StateForPlacementEvent<'_>,
  ) -> Option<Option<mod_api::BlockState>> {
    let facing = face_name(event.context.face.opposite());
    let placed = level.state_with_property(event.state, FACING, facing)?;
    Some(Some(placed))
  }

  /// A spool's first click selects a node, its second lays the wire, and the empty spool takes
  /// a wire out (upstream's `WireSpool.useOn`). The click is ours only when it holds a spool.
  fn use_item_on(
    &self,
    host: &mut dyn ServerHost,
    event: &mut UseItemOnEvent<'_>,
  ) -> Option<InteractionResult> {
    let click = event.click;
    let stack = click.stack?;
    let keys = Keys::resolve(host)?;
    let player = host.players().acting_player()?;
    let level = player.level(host)?;
    let place = ItemPlace::PlayerHand {
      player: click.player,
      hand: click.hand,
    };
    if keys.is_empty_spool(stack.item()) {
      remove_wires(host, level, keys, click.pos);
      return Some(InteractionResult::Success);
    }
    let wire = keys.wire(stack.item())?;
    let target = saved_at(host, level, keys, click.pos)?;
    let target_connector = target.kind.connector()?;
    let Some(first) = pending_of(stack) else {
      // The first click: remember the connector and the node it offers.
      let Some(node) = free_node(&target.nbt, target_connector) else {
        feedback(host, refusal_text(Refusal::NodesFull));
        return Some(InteractionResult::Success);
      };
      offer(host, &place, Some((click.pos, node)));
      feedback(host, "Wire connector selected.");
      return Some(InteractionResult::Success);
    };
    let (from, selected_node) = first;
    offer(host, &place, None);
    if from == click.pos {
      return Some(InteractionResult::Success);
    }
    // The first connector may have gone since the first click: the click is still ours (0.1.0
    // claimed it before reading anything), but there is nothing left to link to.
    let Some(from_saved) = saved_at(host, level, keys, from) else {
      return Some(InteractionResult::Success);
    };
    let Some(from_connector) = from_saved.kind.connector() else {
      return Some(InteractionResult::Success);
    };
    // The node the first click selected, unless another wire took it since.
    let from_node = free_node(&from_saved.nbt, from_connector).unwrap_or(selected_node);
    let proposal = Proposal {
      a: from,
      from: from_connector,
      b: click.pos,
      to: target_connector,
      wire,
    };
    match proposal.check(host, level, keys) {
      Link::Refused(reason) => feedback(host, refusal_text(reason)),
      Link::Linked => {
        if !proposal.store(host, level, keys, from_node) {
          feedback(host, "The wire could not be saved.");
          return Some(InteractionResult::Success);
        }
        let use_up = spool_after_link(place.amount(host), player.is_creative(host) == Some(true));
        spend_spool(host, keys, &place, use_up);
        feedback(host, "Nodes linked.");
      }
    }
    Some(InteractionResult::Success)
  }
}

/// What one laid wire does to the held spool stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SpoolUse {
  /// What the hand keeps.
  hand: HandAfter,
  /// How many empty spools the player is handed as well.
  returns: u32,
}

/// What a hand holds once one wire spool is used up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HandAfter {
  /// Nothing was used (a creative player's hand).
  Untouched,
  /// One wire spool fewer, with room for the empty spool elsewhere in the inventory.
  OneFewer,
  /// The last wire spool, replaced by the empty spool in the hand itself.
  EmptySpool,
}

/// Upstream consumes one spool and gives the empty spool back. The hand keeps what is left of
/// the wire spools, and the last one becomes the empty spool — which is **not** handed over a
/// second time (0.1.0 did, so one wire spool returned two empty ones).
fn spool_after_link(count: u32, creative: bool) -> SpoolUse {
  if creative || count == 0 {
    return SpoolUse {
      hand: HandAfter::Untouched,
      returns: 0,
    };
  }
  if count > 1 {
    SpoolUse {
      hand: HandAfter::OneFewer,
      returns: 1,
    }
  } else {
    SpoolUse {
      hand: HandAfter::EmptySpool,
      returns: 0,
    }
  }
}

/// Record the connector a spool's first click selected, or clear it.
fn offer(host: &mut dyn ServerHost, place: &ItemPlace, at: Option<(Pos, i32)>) {
  let _ = host.transaction(|host, tx| {
    Ok::<_, ()>(place.edit(host, tx, |stack| {
      set_pending(stack, at);
    }))
  });
}

/// Use one wire spool up as [`spool_after_link`] decided, in one transaction.
fn spend_spool(host: &mut dyn ServerHost, keys: Keys, place: &ItemPlace, use_up: SpoolUse) {
  let Some(empty) = ItemStack::new(keys.spool, 1) else {
    return;
  };
  let resource = ItemResource::of(&empty);
  let Some(player) = host.players().acting_player() else {
    return;
  };
  let _ = host.transaction(|host, tx| {
    match use_up.hand {
      HandAfter::Untouched => {}
      HandAfter::OneFewer => {
        if let Some(wire) = place.resource(host) {
          place.extract(host, tx, &wire, 1);
        }
      }
      HandAfter::EmptySpool => {
        place.exchange(host, tx, &resource, 1);
      }
    }
    for _ in 0..use_up.returns {
      let _ = player.give_stack(host, empty.clone());
    }
    Ok::<_, ()>(())
  });
}

/// The custom data a stack carries, or an empty compound.
fn custom_data(stack: &ItemStack) -> Compound {
  match stack
    .components()
    .and_then(|components| components.as_nbt().get(&PENDING.into()))
  {
    Some(Tag::Compound(data)) => data.clone(),
    _ => Compound::new(),
  }
}

/// A spool's remembered first click: the connector and its node.
fn pending_of(stack: &ItemStack) -> Option<(Pos, i32)> {
  let data = custom_data(stack);
  let Tag::Compound(entry) = data.get(&NODE.into())? else {
    return None;
  };
  Some((
    [int_at(entry, "x"), int_at(entry, "y"), int_at(entry, "z")],
    int_at(entry, "node"),
  ))
}

/// Write (or clear) the pending selection inside the stack's custom data.
fn set_pending(stack: &mut ItemStack, at: Option<(Pos, i32)>) {
  let mut data = custom_data(stack);
  match at {
    Some((pos, node)) => {
      let mut entry = Compound::new();
      put_int(&mut entry, "x", pos[0]);
      put_int(&mut entry, "y", pos[1]);
      put_int(&mut entry, "z", pos[2]);
      put_int(&mut entry, "node", node);
      data.insert(NODE.into(), Tag::Compound(entry));
    }
    None => {
      data.remove(&NODE.into());
    }
  }
  let mut patch = Compound::new();
  patch.insert(PENDING.into(), Tag::Compound(data));
  let _ = stack.set_components(patch);
}

fn refusal_text(refusal: Refusal) -> &'static str {
  match refusal {
    Refusal::Long => "The connectors are too far apart for this wire.",
    Refusal::NodesFull => "Every node of this connector has a wire.",
    Refusal::AlreadyLinked => "These connectors are already linked.",
    Refusal::SameBlock => "A connector cannot be linked to itself.",
    Refusal::RequiresHighCurrent => "Two large connectors need a gold or electrum wire.",
  }
}

/// A wire a click or a command proposes between two connectors.
#[derive(Clone, Copy, Debug)]
struct Proposal {
  a: Pos,
  from: Connector,
  b: Pos,
  to: Connector,
  wire: Wire,
}

impl Proposal {
  /// The rule check the spool and the command share: do both ends accept this wire?
  fn check(self, host: &dyn Host, level: LevelRef, keys: Keys) -> Link {
    let at = |pos: Pos| saved_at(host, level, keys, pos);
    let from_wires = at(self.a).map_or_else(Vec::new, |saved| saved_wires(&saved.nbt));
    let to_wires = at(self.b).map_or_else(Vec::new, |saved| saved_wires(&saved.nbt));
    let free_from = at(self.a).and_then(|saved| free_node(&saved.nbt, self.from));
    let free_to = at(self.b).and_then(|saved| free_node(&saved.nbt, self.to));
    plan_link(
      End {
        pos: self.a,
        connector: self.from,
        has_free_node: free_from.is_some(),
        linked_to_other: from_wires.iter().any(|saved| saved.pos == self.b),
      },
      End {
        pos: self.b,
        connector: self.to,
        has_free_node: free_to.is_some(),
        linked_to_other: to_wires.iter().any(|saved| saved.pos == self.a),
      },
      self.wire,
    )
  }

  /// Lay the wire in both connectors' saved compounds; answers whether both ends took it.
  /// `node` is the node `a`'s end goes in.
  fn store(self, host: &mut dyn ServerHost, level: LevelRef, keys: Keys, node: i32) -> bool {
    let Some(to_node) =
      saved_at(host, level, keys, self.b).and_then(|saved| free_node(&saved.nbt, self.to))
    else {
      return false;
    };
    let mut left =
      saved_at(host, level, keys, self.a).map_or_else(Vec::new, |saved| saved_wires(&saved.nbt));
    left.push(SavedWire {
      pos: self.b,
      node,
      wire: self.wire,
    });
    let mut right =
      saved_at(host, level, keys, self.b).map_or_else(Vec::new, |saved| saved_wires(&saved.nbt));
    right.push(SavedWire {
      pos: self.a,
      node: to_node,
      wire: self.wire,
    });
    let mut left_data = Compound::new();
    store_wires(&mut left_data, &left);
    let mut right_data = Compound::new();
    store_wires(&mut right_data, &right);
    let stored_a = host
      .block_entity(level, self.a)
      .is_some_and(|entity| entity.merge_data(host, left_data).is_ok());
    let stored_b = host
      .block_entity(level, self.b)
      .is_some_and(|entity| entity.merge_data(host, right_data).is_ok());
    stored_a && stored_b
  }
}

/// What an empty spool hands back when it takes a connector's wires out: one spool of each
/// wire's own kind, per wire removed (upstream coils each wire back onto its own spool).
fn spools_back(wires: &[SavedWire]) -> [(Wire, u32); 3] {
  let mut counts = [(Wire::Copper, 0), (Wire::Gold, 0), (Wire::Electrum, 0)];
  for saved in wires {
    if let Some(slot) = counts.iter_mut().find(|(wire, _)| *wire == saved.wire) {
      slot.1 += 1;
    }
  }
  counts
}

/// The empty spool takes every wire out of the clicked connector, on both ends.
fn remove_wires(host: &mut dyn ServerHost, level: LevelRef, keys: Keys, pos: Pos) {
  let Some(saved) = saved_at(host, level, keys, pos) else {
    return;
  };
  if saved.kind.connector().is_none() {
    return;
  }
  let wires = saved_wires(&saved.nbt);
  if wires.is_empty() {
    feedback(host, "This connector has no wire.");
    return;
  }
  for wire in &wires {
    let Some(other) = saved_at(host, level, keys, wire.pos) else {
      continue;
    };
    let kept: Vec<SavedWire> = saved_wires(&other.nbt)
      .into_iter()
      .filter(|saved| saved.pos != pos)
      .collect();
    let mut data = Compound::new();
    store_wires(&mut data, &kept);
    if let Some(entity) = host.block_entity(level, wire.pos) {
      let _ = entity.merge_data(host, data);
    }
  }
  let mut data = Compound::new();
  store_wires(&mut data, &[]);
  if let Some(entity) = host.block_entity(level, pos) {
    let _ = entity.merge_data(host, data);
  }
  if let Some(player) = host.players().acting_player()
    && player.is_creative(host) != Some(true)
  {
    for (wire, count) in spools_back(&wires) {
      let key = match wire {
        Wire::Copper => keys.copper,
        Wire::Gold => keys.gold,
        Wire::Electrum => keys.electrum,
      };
      if let Some(spools) = ItemStack::new(key, count) {
        let _ = player.give_stack(host, spools);
      }
    }
  }
  feedback(host, "Wire removed.");
}

// --- the level pass -------------------------------------------------------

/// Settle each queued connector's placement mode, let the queued accumulators feed the blocks
/// above and below, then run every wire network among the queued connectors.
pub(crate) fn level_ticked(host: &mut dyn ServerHost, level: LevelRef, queued: &BTreeSet<Pos>) {
  if queued.is_empty() {
    return;
  }
  let Some(keys) = Keys::resolve(host) else {
    return;
  };
  let blocks: Vec<(Pos, Saved)> = queued
    .iter()
    .filter_map(|&pos| Some((pos, saved_at(host, level, keys, pos)?)))
    .collect();
  if blocks.is_empty() {
    return;
  }
  for (pos, saved) in &blocks {
    if saved.kind.connector().is_some() {
      settle(host, level, keys, *pos);
    }
  }
  for (pos, saved) in &blocks {
    if saved.kind == Kind::Accumulator {
      accumulator_output(host, level, *pos);
    }
  }
  let nodes: Vec<Node> = blocks
    .iter()
    .filter_map(|(pos, _)| node_at(host, level, keys, *pos))
    .collect();
  if nodes.len() < 2 {
    return;
  }
  let positions: Vec<Pos> = nodes.iter().map(|node| node.pos).collect();
  let wires: Vec<Vec<SavedWire>> = nodes.iter().map(|node| node.wires.clone()).collect();
  for members in groups(&positions, &wires) {
    if members.len() >= 2 {
      run_network(host, level, keys, &members);
    }
  }
}

/// Give a connector its placement mode on the first pass after it is placed, as upstream's
/// `ConnectorMode.test` does: a mounted block that can give is pulled from, one that can only
/// take is pushed into, and one that does neither leaves the connector idle.
fn settle(host: &mut dyn ServerHost, level: LevelRef, keys: Keys, pos: Pos) {
  let Some(saved) = saved_at(host, level, keys, pos) else {
    return;
  };
  if saved.kind.connector().is_none() || mode_saved(&saved.nbt).is_some() {
    return;
  }
  let mode = match mounted(host, level, pos) {
    Some((at, side)) => {
      let ours = saved_at(host, level, keys, at).map(|mounted| mounted.kind);
      // A foreign block's `EnergyHandler` answers whether the side reaches a fill, a drain
      // or both, and nothing says which: one that answers at all is taken as a source.
      let foreign = ours.is_none() && level.capability(host, &ENERGY, at, Some(side)).is_some();
      mounted_mode(ours, foreign, side)
    }
    None => Mode::None,
  };
  let mut data = Compound::new();
  put_name(&mut data, MODE, mode.name());
  let Some(entity) = host.block_entity(level, pos) else {
    return;
  };
  if entity.merge_data(host, data).is_ok() {
    let _ = host.set_block_property(pos, MODE, mode.name());
  }
}

/// Whether a connector mounted on `ours` (or on a foreign block answering the energy capability)
/// settles to pull, push or neither.
fn mounted_mode(ours: Option<Kind>, foreign: bool, side: Direction) -> Mode {
  match ours {
    Some(kind) => {
      let buffer = kind.buffer();
      let face = side as usize;
      let can_extract = buffer.max_extract > 0 && buffer.extract_faces[face];
      let can_insert = buffer.max_insert > 0 && buffer.insert_faces[face];
      placed_mode(can_extract, can_insert)
    }
    None if foreign => Mode::Pull,
    None => Mode::None,
  }
}

/// An accumulator feeds the blocks on its UP and DOWN faces
/// (`ModularAccumulatorBlockEntity.tickOutput`); a single block has both of its `top`/`bottom`
/// state bits set, which this slice always keeps.
fn accumulator_output(host: &mut dyn ServerHost, level: LevelRef, pos: Pos) {
  for side in [Direction::Up, Direction::Down] {
    let at = offset(pos, side);
    let Some(source) = level.capability(host, &ENERGY, pos, Some(side)) else {
      continue;
    };
    let Some(sink) = level.capability(host, &ENERGY, at, Some(side.opposite())) else {
      continue;
    };
    let max = u32::try_from(ACCUMULATOR_TRANSFER).unwrap_or(0);
    let _ = host.transaction(|host, tx| move_energy(host, &source, &sink, max, tx));
  }
}

/// The connector groups of one pass, each a list of positions in the order they were given. Two
/// connectors share a group when a wire joins them, directly or through others.
fn groups(positions: &[Pos], wires: &[Vec<SavedWire>]) -> Vec<Vec<Pos>> {
  let index: BTreeMap<Pos, usize> = positions
    .iter()
    .enumerate()
    .map(|(at, pos)| (*pos, at))
    .collect();
  let mut parents: Vec<usize> = (0..positions.len()).collect();
  fn root(parents: &mut [usize], at: usize) -> usize {
    let mut current = at;
    while parents[current] != current {
      current = parents[current];
    }
    let found = current;
    let mut walk = at;
    while parents[walk] != found {
      let next = parents[walk];
      parents[walk] = found;
      walk = next;
    }
    found
  }
  for (at, saved) in wires.iter().enumerate() {
    for wire in saved {
      if let Some(&other) = index.get(&wire.pos) {
        let (a, b) = (root(&mut parents, at), root(&mut parents, other));
        if a != b {
          parents[a.max(b)] = a.min(b);
        }
      }
    }
  }
  let mut found: BTreeMap<usize, Vec<Pos>> = BTreeMap::new();
  for (at, pos) in positions.iter().enumerate() {
    let group = root(&mut parents, at);
    found.entry(group).or_default().push(*pos);
  }
  found.into_values().collect()
}

/// One network's tick: swap the buffer, feed every push connector from it, then draw every pull
/// connector into it. The buffer lives in the saved compound of the network's lowest-positioned
/// connector, so a reload cannot lose it.
fn run_network(host: &mut dyn ServerHost, level: LevelRef, keys: Keys, members: &[Pos]) {
  let nodes: Vec<Node> = members
    .iter()
    .filter_map(|&pos| node_at(host, level, keys, pos))
    .collect();
  if nodes.len() < 2 {
    return;
  }
  let Some(controller) = nodes.iter().map(|node| node.pos).min() else {
    return;
  };
  let Some(nbt) = host
    .block_entity(level, controller)
    .and_then(|entity| entity.data(host))
  else {
    return;
  };
  let demand = int_at(&nbt, NET_DEMAND);
  let capacity = buffer_capacity(nodes.len(), demand);
  let saved = Pool {
    ready: int_at(&nbt, NET_READY),
    next: int_at(&nbt, NET_NEXT),
  };
  let outcome = host.transaction(|host, tx| {
    let mut pool = saved;
    pool.swap();
    let mut wanted = 0;
    for node in nodes.iter().filter(|node| node.mode == Mode::Push) {
      let Some((at, side)) = mounted(host, level, node.pos) else {
        continue;
      };
      let Some(sink) = level.capability(host, &ENERGY, at, Some(side)) else {
        continue;
      };
      let room = sink.capacity(host).saturating_sub(sink.amount(host));
      let amount = push_amount(
        node.connector.max_out(),
        i32::try_from(room).unwrap_or(i32::MAX),
        pool.ready,
      );
      if amount <= 0 {
        continue;
      }
      wanted += amount;
      let asked = u32::try_from(amount).unwrap_or(0);
      let moved = sink.insert(host, tx, asked).min(asked);
      pool.take(i32::try_from(moved).unwrap_or(i32::MAX));
    }
    for node in nodes.iter().filter(|node| node.mode == Mode::Pull) {
      let Some((at, side)) = mounted(host, level, node.pos) else {
        continue;
      };
      let Some(source) = level.capability(host, &ENERGY, at, Some(side)) else {
        continue;
      };
      let amount = pull_amount(
        node.connector.max_in(),
        i32::try_from(source.amount(host)).unwrap_or(i32::MAX),
        pool.room(capacity),
      );
      if amount <= 0 {
        continue;
      }
      let asked = u32::try_from(amount).unwrap_or(0);
      let moved = source.extract(host, tx, asked).min(asked);
      pool.put(i32::try_from(moved).unwrap_or(i32::MAX), capacity);
    }
    Ok::<_, ()>((pool, wanted))
  });
  let Ok((pool, wanted)) = outcome else {
    return;
  };
  let unchanged = int_at(&nbt, NET_READY) == pool.ready
    && int_at(&nbt, NET_NEXT) == pool.next
    && int_at(&nbt, NET_DEMAND) == wanted;
  if unchanged {
    return;
  }
  let mut data = Compound::new();
  put_int(&mut data, NET_READY, pool.ready);
  put_int(&mut data, NET_NEXT, pool.next);
  put_int(&mut data, NET_DEMAND, wanted);
  if let Some(entity) = host.block_entity(level, controller) {
    let _ = entity.merge_data(host, data);
  }
}

// --- the commands ---------------------------------------------------------

/// The five usages `/createaddition` declares. The optional `tag:word` is the harness's marker:
/// the reply names it, so a line from an earlier command cannot be read as this one's answer.
pub(crate) fn command_usages() -> [&'static str; 5] {
  [
    "createaddition info <pos:block_pos> [tag:word]",
    "createaddition energy <pos:block_pos> <amount:integer> [tag:word]",
    "createaddition link <pos:block_pos> <pos2:block_pos> <wire:word> [tag:word]",
    "createaddition unlink <pos:block_pos> <pos2:block_pos> [tag:word]",
    "createaddition mode <pos:block_pos> <mode:word> [tag:word]",
  ]
}

/// A trailing `#tag` word, without its marker: the harness marks each command so its reply can be
/// told apart from an older, identical one (`reply` appends it).
fn tag_of(command: &CommandInvocation) -> Option<&str> {
  match command.argument("tag")? {
    CommandArgumentValue::String(word) => word.strip_prefix('#').filter(|tag| !tag.is_empty()),
    _ => None,
  }
}

/// `text` with the harness's tag appended.
fn reply(tag: Option<&str>, text: String) -> String {
  match tag {
    Some(tag) => format!("{text} [#{tag}]"),
    None => text,
  }
}

/// The `pos` argument of a command as a block position.
fn command_pos(command: &CommandInvocation) -> Option<Pos> {
  match command.argument("pos")? {
    CommandArgumentValue::BlockPos(pos) => Some(*pos),
    _ => None,
  }
}

/// A `word` argument of a command.
fn command_word<'a>(command: &'a CommandInvocation, name: &str) -> Option<&'a str> {
  match command.argument(name)? {
    CommandArgumentValue::String(word) => Some(word),
    _ => None,
  }
}

/// An `integer` argument of a command.
fn command_integer(command: &CommandInvocation, name: &str) -> Option<i32> {
  match command.argument(name)? {
    CommandArgumentValue::Integer(value) => Some(*value),
    _ => None,
  }
}

/// The level a command acts in: the acting player's own.
fn command_level(host: &dyn ServerHost) -> Option<LevelRef> {
  host.players().acting_player()?.level(host)
}

/// A command's answer, through the source's own feedback channel.
fn feedback(host: &mut dyn ServerHost, line: &str) {
  let _ = host.commands().feedback(Text::literal(line));
}

/// `/createaddition info <pos>`: a connector's mode, buffer, links and network buffer, or an
/// accumulator's charge.
pub(crate) fn command_info(host: &mut dyn ServerHost, command: &CommandInvocation) {
  let tag = tag_of(command);
  let Some(pos) = command_pos(command) else {
    feedback(host, "Usage: /createaddition info <x> <y> <z>");
    return;
  };
  let Some(level) = command_level(host) else {
    feedback(host, "No level to read.");
    return;
  };
  let Some(keys) = Keys::resolve(host) else {
    return;
  };
  let answer = match saved_at(host, level, keys, pos) {
    Some(saved) => reply(tag, info_line(host, level, pos, &saved)),
    None => reply(tag, "No Create Crafts & Additions block there.".to_owned()),
  };
  feedback(host, &answer);
}

/// A block's line of text: an accumulator's charge, or a connector's mode, buffer, links and
/// network buffer. Byte-identical to 0.1.0's readout.
fn info_line(host: &dyn Host, level: LevelRef, pos: Pos, saved: &Saved) -> String {
  let time = level.game_time(host).unwrap_or(0);
  let charge = int_at(&saved.nbt, CHARGE).max(0);
  let Some(connector) = saved.kind.connector() else {
    return format!(
      "{} at {},{},{}: {}/{} FE (t={})",
      Kind::Accumulator.label(),
      pos[0],
      pos[1],
      pos[2],
      charge,
      ACCUMULATOR_CAPACITY,
      time
    );
  };
  let links = saved_wires(&saved.nbt)
    .iter()
    .map(|wire| {
      format!(
        "({},{},{})/{}/{}",
        wire.pos[0],
        wire.pos[1],
        wire.pos[2],
        wire.node,
        wire.wire.name()
      )
    })
    .collect::<Vec<_>>()
    .join(" ");
  format!(
    "{} at {},{},{}: mode {}, buffer {}/{} FE, links [{}], pool {}/{}, demand {}, t={}",
    saved.kind.label(),
    pos[0],
    pos[1],
    pos[2],
    mode_saved(&saved.nbt).unwrap_or(Mode::None).name(),
    charge,
    connector.max_in().min(connector.max_out()),
    links,
    int_at(&saved.nbt, NET_READY),
    int_at(&saved.nbt, NET_NEXT),
    int_at(&saved.nbt, NET_DEMAND),
    time
  )
}

/// A requested amount of FE, no less than an empty buffer and no more than its capacity.
fn clamped(amount: i32, capacity: i32) -> i32 {
  amount.clamp(0, capacity.max(0))
}

/// `/createaddition energy <pos> <amount>`: seed or empty one of our blocks. The charge lives in
/// the block entity's saved compound (`ModEnergyBuffer`'s own key), so one write sets it.
pub(crate) fn command_energy(host: &mut dyn ServerHost, command: &CommandInvocation) {
  let tag = tag_of(command);
  let Some(pos) = command_pos(command) else {
    feedback(host, "Usage: /createaddition energy <x> <y> <z> <amount>");
    return;
  };
  let Some(amount) = command_integer(command, "amount") else {
    feedback(host, "The amount must be a whole number of FE.");
    return;
  };
  let Some(level) = command_level(host) else {
    feedback(host, "No level to write.");
    return;
  };
  let Some(keys) = Keys::resolve(host) else {
    return;
  };
  let Some(saved) = saved_at(host, level, keys, pos) else {
    feedback(host, "No Create Crafts & Additions block there.");
    return;
  };
  let after = clamped(amount, saved.kind.capacity());
  let mut data = Compound::new();
  put_int(&mut data, CHARGE, after);
  let Some(entity) = host.block_entity(level, pos) else {
    feedback(host, "No Create Crafts & Additions block there.");
    return;
  };
  match entity.merge_data(host, data) {
    Ok(_) => feedback(host, &reply(tag, format!("Energy set to {after} FE."))),
    Err(_) => feedback(host, "No Create Crafts & Additions block there."),
  }
}

/// `/createaddition mode <pos> <mode>`: set a connector's mode by command, because Create's
/// wrench is not installed.
pub(crate) fn command_mode(host: &mut dyn ServerHost, command: &CommandInvocation) {
  let tag = tag_of(command);
  let Some(pos) = command_pos(command) else {
    feedback(host, "Usage: /createaddition mode <x> <y> <z> <mode>");
    return;
  };
  let Some(level) = command_level(host) else {
    feedback(host, "No level to write.");
    return;
  };
  let Some(keys) = Keys::resolve(host) else {
    return;
  };
  let Some(saved) = saved_at(host, level, keys, pos) else {
    feedback(host, "No connector there.");
    return;
  };
  if saved.kind.connector().is_none() {
    feedback(host, "No connector there.");
    return;
  }
  let Some(name) = command_word(command, "mode") else {
    feedback(host, "Usage: /createaddition mode <x> <y> <z> <mode>");
    return;
  };
  let Some(mode) = Mode::of_name(name) else {
    feedback(
      host,
      &reply(tag, "The mode is none, pull or push.".to_owned()),
    );
    return;
  };
  let mut data = Compound::new();
  put_name(&mut data, MODE, mode.name());
  if let Some(entity) = host.block_entity(level, pos) {
    let _ = entity.merge_data(host, data);
  }
  if level.in_place(host) {
    let _ = host.set_block_property(pos, MODE, mode.name());
  }
  feedback(host, &reply(tag, format!("Mode set to {}.", mode.name())));
}

/// `/createaddition link <a> <b> <wire>`: lay a wire, as a spool would.
pub(crate) fn command_link(host: &mut dyn ServerHost, command: &CommandInvocation) {
  let tag = tag_of(command);
  let Some(a) = command_pos(command) else {
    feedback(
      host,
      "Usage: /createaddition link <x> <y> <z> <x2> <y2> <z2> <copper|gold|electrum>",
    );
    return;
  };
  let Some(b) = command.argument("pos2").and_then(|value| match value {
    CommandArgumentValue::BlockPos(pos) => Some(*pos),
    _ => None,
  }) else {
    feedback(
      host,
      "Usage: /createaddition link <x> <y> <z> <x2> <y2> <z2> <copper|gold|electrum>",
    );
    return;
  };
  let Some(level) = command_level(host) else {
    feedback(host, "No level to write.");
    return;
  };
  let Some(keys) = Keys::resolve(host) else {
    return;
  };
  let Some(wire) = command_word(command, "wire").and_then(Wire::of_name) else {
    feedback(
      host,
      &reply(tag, "The wire is copper, gold or electrum.".to_owned()),
    );
    return;
  };
  let answer = link_blocks(host, level, keys, a, b, wire);
  feedback(host, &reply(tag, answer));
}

/// `/createaddition unlink <a> <b>`: take the wire between two connectors out.
pub(crate) fn command_unlink(host: &mut dyn ServerHost, command: &CommandInvocation) {
  let tag = tag_of(command);
  let Some(a) = command_pos(command) else {
    feedback(
      host,
      "Usage: /createaddition unlink <x> <y> <z> <x2> <y2> <z2>",
    );
    return;
  };
  let Some(b) = command.argument("pos2").and_then(|value| match value {
    CommandArgumentValue::BlockPos(pos) => Some(*pos),
    _ => None,
  }) else {
    feedback(
      host,
      "Usage: /createaddition unlink <x> <y> <z> <x2> <y2> <z2>",
    );
    return;
  };
  let Some(level) = command_level(host) else {
    feedback(host, "No level to write.");
    return;
  };
  let Some(keys) = Keys::resolve(host) else {
    return;
  };
  let answer = unlink_blocks(host, level, keys, a, b);
  feedback(host, &reply(tag, answer));
}

fn link_blocks(
  host: &mut dyn ServerHost,
  level: LevelRef,
  keys: Keys,
  a: Pos,
  b: Pos,
  wire: Wire,
) -> String {
  let Some(first) = saved_at(host, level, keys, a) else {
    return "No connector at the first position.".to_owned();
  };
  let Some(second) = saved_at(host, level, keys, b) else {
    return "No connector at the second position.".to_owned();
  };
  let (Some(from), Some(to)) = (first.kind.connector(), second.kind.connector()) else {
    return "Both positions must be connectors.".to_owned();
  };
  let proposal = Proposal {
    a,
    from,
    b,
    to,
    wire,
  };
  match proposal.check(host, level, keys) {
    Link::Refused(reason) => refusal_text(reason).to_owned(),
    Link::Linked => {
      let Some(from_node) =
        saved_at(host, level, keys, a).and_then(|saved| free_node(&saved.nbt, from))
      else {
        return refusal_text(Refusal::NodesFull).to_owned();
      };
      if proposal.store(host, level, keys, from_node) {
        "Nodes linked.".to_owned()
      } else {
        refusal_text(Refusal::NodesFull).to_owned()
      }
    }
  }
}

fn unlink_blocks(host: &mut dyn ServerHost, level: LevelRef, keys: Keys, a: Pos, b: Pos) -> String {
  let mut removed = false;
  for (here, there) in [(a, b), (b, a)] {
    let Some(before) = saved_at(host, level, keys, here) else {
      continue;
    };
    let wires = saved_wires(&before.nbt);
    let kept: Vec<SavedWire> = wires
      .iter()
      .filter(|saved| saved.pos != there)
      .cloned()
      .collect();
    if kept.len() == wires.len() {
      continue;
    }
    removed = true;
    let mut data = Compound::new();
    store_wires(&mut data, &kept);
    if let Some(entity) = host.block_entity(level, here) {
      let _ = entity.merge_data(host, data);
    }
  }
  if removed {
    "Wire removed.".to_owned()
  } else {
    "These connectors are not linked.".to_owned()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn wire(pos: Pos) -> SavedWire {
    SavedWire {
      pos,
      node: 0,
      wire: Wire::Copper,
    }
  }

  #[test]
  fn every_face_names_itself() {
    for face in Direction::ALL {
      assert_eq!(face_named(face_name(face)), Some(face), "{face:?}");
    }
    assert_eq!(face_named("sideways"), None);
    assert_eq!(face_named(""), None);
  }

  #[test]
  fn a_block_with_energy_settles_the_connector_pulling() {
    // Our own blocks: the rule the runtime's `placed_mode` documents.
    for kind in [Kind::Small, Kind::Large, Kind::Accumulator] {
      for face in Direction::ALL {
        assert_eq!(
          mounted_mode(Some(kind), false, face),
          Mode::Pull,
          "{kind:?} at {face:?}"
        );
      }
    }
  }

  #[test]
  fn a_foreign_block_answering_energy_is_a_source() {
    assert_eq!(mounted_mode(None, true, Direction::Up), Mode::Pull);
  }

  #[test]
  fn a_block_with_no_energy_settles_the_connector_idle() {
    assert_eq!(mounted_mode(None, false, Direction::Up), Mode::None);
  }

  #[test]
  fn the_last_wire_spool_becomes_the_empty_spool_and_is_not_given_twice() {
    // One wire spool: the hand holds the empty spool, and nothing else is returned.
    assert_eq!(
      spool_after_link(1, false),
      SpoolUse {
        hand: HandAfter::EmptySpool,
        returns: 0
      }
    );
    // More than one: one is used and the empty spool is handed over.
    assert_eq!(
      spool_after_link(5, false),
      SpoolUse {
        hand: HandAfter::OneFewer,
        returns: 1
      }
    );
  }

  #[test]
  fn a_creative_spool_lays_wire_without_using_any() {
    assert_eq!(
      spool_after_link(1, true),
      SpoolUse {
        hand: HandAfter::Untouched,
        returns: 0
      }
    );
  }

  #[test]
  fn a_tagged_reply_names_its_tag() {
    assert_eq!(
      reply(Some("q3"), "Nodes linked.".to_owned()),
      "Nodes linked. [#q3]"
    );
    assert_eq!(reply(None, "Nodes linked.".to_owned()), "Nodes linked.");
  }

  #[test]
  fn two_connectors_joined_by_a_wire_are_one_group() {
    let a = [0, 0, 0];
    let b = [4, 0, 0];
    let c = [8, 0, 0];
    let alone = [40, 0, 0];
    let wires = [vec![wire(b)], vec![wire(a), wire(c)], vec![wire(b)], vec![]];
    let found = groups(&[a, b, c, alone], &wires);
    assert_eq!(found, vec![vec![a, b, c], vec![alone]]);
  }

  #[test]
  fn a_wire_to_a_connector_that_is_not_there_groups_nothing() {
    let a = [0, 0, 0];
    let found = groups(&[a], &[vec![wire([99, 0, 0])]]);
    assert_eq!(found, vec![vec![a]]);
  }

  #[test]
  fn an_empty_spool_hands_back_one_spool_of_each_wires_own_kind() {
    let a = SavedWire {
      pos: [1, 0, 0],
      node: 0,
      wire: Wire::Copper,
    };
    let b = SavedWire {
      pos: [2, 0, 0],
      node: 1,
      wire: Wire::Gold,
    };
    let c = SavedWire {
      pos: [3, 0, 0],
      node: 2,
      wire: Wire::Copper,
    };
    assert_eq!(
      spools_back(&[a.clone(), b, c]),
      [(Wire::Copper, 2), (Wire::Gold, 1), (Wire::Electrum, 0)]
    );
    assert_eq!(
      spools_back(&[]),
      [(Wire::Copper, 0), (Wire::Gold, 0), (Wire::Electrum, 0)]
    );
  }

  #[test]
  fn the_energy_command_clamps_to_the_buffer() {
    assert_eq!(clamped(-5, 1_000), 0);
    assert_eq!(clamped(400, 1_000), 400);
    assert_eq!(clamped(9_999, 1_000), 1_000);
  }
}
