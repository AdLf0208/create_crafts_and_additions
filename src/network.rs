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

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use mod_api::{
  self, AnalogOutputEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockKey,
  BlockPart, Command, CommandArgumentValue, CommandInvocation, Compound, Direction, ENERGY,
  HasAnalogOutputEvent, Host, HostBlockStates, HostTransaction, InteractionResult, ItemKey,
  ItemPlace, ItemResource, ItemStack, LevelHost, LevelRef, LightEmissionEvent, ModEnergyBuffer,
  NeighbourUpdate, ServerHost, StateConstraint, StateForPlacementEvent, Tag, Text, UseItemOnEvent,
  move_energy,
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
/// The redstone relay: a redstone-gated bridge between two wire networks.
pub(crate) const RELAY: &str = "createaddition:redstone_relay";
/// The small light connector: a small connector whose lamp draws 1 FE a tick from its
/// network and lights while the network pays it.
pub(crate) const LIGHT_CONNECTOR: &str = "createaddition:small_light_connector";
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
/// The relay's `powered` state bit, which gates its bridge (`RedstoneRelayBlock.POWERED`).
const POWERED: &str = "powered";
/// The relay's `vertical` state bit: a relay clicked onto a side is a vertical one.
const VERTICAL: &str = "vertical";
/// A relay has eight wire nodes: 0–3 the input port, 4–7 the output port
/// (`RedstoneRelayBlockEntity.NODE_COUNT` / `WireNodeKind::portOf`).
const RELAY_NODES: usize = 8;
/// The relay bridges at the small connector's rate, the wire kind it connects
/// (`WireNodeKind.RELAY` reports `ConnectorType.Small`).
const RELAY_TRANSFER: i32 = 1_000;
/// The lamp's draw, in FE a tick (`small_light_connector_consumption`, 1 by default).
const LIGHT_DRAW: i32 = 1;
/// The lamp's saved hysteresis counter: `tick_toggle_timer` goes up while the network pays
/// the lamp and down while it does not, and its thresholds are `±light_period`.
const TOGGLE_TIMER: &str = "tick_toggle_timer";
/// The light a lit lamp emits, 0 to 15 (`SmallLightConnectorBlock.getLightEmission`).
const LIGHT_EMISSION: u8 = 15;
/// One network's saved buffer, in its controller's compound (0.1.0's keys, kept).
const NET_READY: &str = "netReady";
const NET_NEXT: &str = "netNext";
const NET_DEMAND: &str = "netDemand";
/// The saved wire list of a connector.
const LINKS: &str = "links";
/// The tower an accumulator block belongs to: its shape and its controller's position, saved in
/// each member's compound so a block answers for the tower it was last formed into.
const TOWER_SAVED_WIDTH: &str = "tw";
const TOWER_SAVED_HEIGHT: &str = "th";
const TOWER_SAVED_X: &str = "tx";
const TOWER_SAVED_Y: &str = "ty";
const TOWER_SAVED_Z: &str = "tz";

/// A block position.
pub(crate) type Pos = [i32; 3];

/// The four blocks this port owns, and the energy buffer each declares (the relay declares none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
  /// `createaddition:connector`: 1,000 FE/t either way, a 1,000 FE window.
  Small,
  /// `createaddition:large_connector`: 5,000 FE/t either way.
  Large,
  /// `createaddition:modular_accumulator`: 2,000,000 FE, 5,000 FE/t either way.
  Accumulator,
  /// `createaddition:redstone_relay`: an eight-node bridge from its input wire network to
  /// its output one while powered. Its 1,000 FE is the in-flight buffer, not a declared
  /// capability: upstream's relay exposes no energy storage.
  Relay,
  /// `createaddition:small_light_connector`: the small connector with a lamp, which draws
  /// 1 FE a tick out of its network and lights while the network pays it.
  Light,
}

impl Kind {
  /// The connector this block is, if it is one.
  fn connector(self) -> Option<Connector> {
    match self {
      Self::Small | Self::Light => Some(Connector::Small),
      Self::Large => Some(Connector::Large),
      Self::Accumulator | Self::Relay => None,
    }
  }

  /// How the readout names the block.
  fn label(self) -> &'static str {
    match self {
      Self::Small => "Connector",
      Self::Large => "Large Connector",
      Self::Accumulator => "Modular Accumulator",
      Self::Relay => "Redstone Relay",
      Self::Light => "Small Light Connector",
    }
  }

  /// The energy buffer the block declares (`Registrar::block_energy`): upstream's window for
  /// the two connectors and the accumulator's own capacity, reachable from every side. The
  /// relay declares none — its 1,000 FE is the bridge's in-flight buffer — so this only sizes
  /// its readout and the admin `energy` command.
  pub(crate) fn buffer(self) -> ModEnergyBuffer {
    let (capacity, max_insert, max_extract) = match self.connector() {
      Some(connector) => (
        connector.max_in().min(connector.max_out()),
        connector.max_in(),
        connector.max_out(),
      ),
      None if self == Self::Relay => (RELAY_TRANSFER, RELAY_TRANSFER, RELAY_TRANSFER),
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

  /// The block as a wire node, for the four blocks that take wires; the accumulator is
  /// not one.
  fn wire_node(self) -> Option<WireNode> {
    match self {
      Self::Small | Self::Light => Some(WireNode::Connector(Connector::Small)),
      Self::Large => Some(WireNode::Connector(Connector::Large)),
      Self::Relay => Some(WireNode::Relay),
      Self::Accumulator => None,
    }
  }
}

/// A block that takes wires: the two connectors and the redstone relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WireNode {
  /// A connector, with its own node count and wire length.
  Connector(Connector),
  /// The relay: eight nodes in two ports, small-connector reach and rate.
  Relay,
}

impl WireNode {
  /// How many nodes the block has.
  fn nodes(self) -> usize {
    match self {
      Self::Connector(connector) => connector.nodes(),
      Self::Relay => RELAY_NODES,
    }
  }

  /// The connector whose rate and reach the link rules use; the relay is a small one.
  fn connector(self) -> Connector {
    match self {
      Self::Connector(connector) => connector,
      Self::Relay => Connector::Small,
    }
  }
}

/// One of a relay's two ports: nodes 0–3 take energy in, nodes 4–7 give it out
/// (`WireNodeKind::portOf`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RelayPort {
  /// The input port, nodes 0–3.
  Input,
  /// The output port, nodes 4–7.
  Output,
}

impl RelayPort {
  /// The port's number, as upstream's `portOf` has it.
  const fn index(self) -> u8 {
    match self {
      Self::Input => 0,
      Self::Output => 1,
    }
  }

  /// The port a relay node belongs to.
  const fn of_node(node: i32) -> Self {
    if node < 4 { Self::Input } else { Self::Output }
  }
}

/// One port of one wire block: a connector has only port 0, a relay has 0 (input) and
/// 1 (output). Two ports are separate networks even when one relay joins them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PortMember {
  /// The block's position.
  pos: Pos,
  /// The port number (`RelayPort::index`; connectors are 0).
  port: u8,
}

impl PortMember {
  /// A connector's single port.
  const fn connector(pos: Pos) -> Self {
    Self { pos, port: 0 }
  }

  /// A relay's input or output port.
  const fn relay(pos: Pos, port: RelayPort) -> Self {
    Self {
      pos,
      port: port.index(),
    }
  }
}

/// What one tick of a relay's bridge does: how much it takes out of its input network's
/// pool into its own buffer, and how much it gives its output network's pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RelayMove {
  /// Energy taken from the input pool into the relay's buffer.
  into: i32,
  /// Energy put from the relay's buffer into the output pool.
  out: i32,
}

/// The registry keys this port resolves once per pass: the four blocks and the four spools.
#[derive(Clone, Copy)]
struct Keys {
  small: BlockKey,
  large: BlockKey,
  accumulator: BlockKey,
  relay: BlockKey,
  light: BlockKey,
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
      relay: host.block_key(RELAY)?,
      light: host.block_key(LIGHT_CONNECTOR)?,
      spool: item(SPOOL)?,
      copper: item(COPPER_SPOOL)?,
      gold: item(GOLD_SPOOL)?,
      electrum: item(ELECTRUM_SPOOL)?,
    })
  }

  /// Which of our blocks `block` is.
  fn kind(self, block: BlockKey) -> Option<Kind> {
    if block == self.small {
      Some(Kind::Small)
    } else if block == self.large {
      Some(Kind::Large)
    } else if block == self.accumulator {
      Some(Kind::Accumulator)
    } else if block == self.relay {
      Some(Kind::Relay)
    } else if block == self.light {
      Some(Kind::Light)
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

/// The first node of a wire block that has no wire in it.
fn free_node(nbt: &Compound, node: WireNode) -> Option<i32> {
  let used: BTreeSet<i32> = saved_wires(nbt).iter().map(|wire| wire.node).collect();
  (0..i32::try_from(node.nodes()).ok()?).find(|node| !used.contains(node))
}

/// The first free node of one relay port, or `None` when that port's four are taken. A
/// relay's port is not interchangeable: a wire in 0–3 feeds the bridge, one in 4–7 leaves
/// it, so a full input port must refuse the wire rather than silently use the output.
fn relay_free_node(nbt: &Compound, port: RelayPort) -> Option<i32> {
  let used: BTreeSet<i32> = saved_wires(nbt).iter().map(|wire| wire.node).collect();
  let (first, end) = match port {
    RelayPort::Input => (0, 4),
    RelayPort::Output => (4, 8),
  };
  (first..end).find(|node| !used.contains(node))
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

/// The horizontal direction 90° clockwise of `face`, as `Direction.getClockWise` has it
/// (north → east → south → west).
fn clockwise(face: Direction) -> Direction {
  match face {
    Direction::North => Direction::East,
    Direction::East => Direction::South,
    Direction::South => Direction::West,
    Direction::West => Direction::North,
    _ => face,
  }
}

/// The horizontal direction 90° counter-clockwise of `face` (north → west → south → east).
fn counter_clockwise(face: Direction) -> Direction {
  match face {
    Direction::North => Direction::West,
    Direction::West => Direction::South,
    Direction::South => Direction::East,
    Direction::East => Direction::North,
    _ => face,
  }
}

/// The horizontal direction a placer's yaw looks along, as `Direction.fromYRot` reads it:
/// 0° is south, 90° west, 180° north, 270° east.
fn yaw_facing(yaw: f32) -> Direction {
  match i32::try_from(((yaw.rem_euclid(360.0) + 45.0) / 90.0) as i64).unwrap_or(0) % 4 {
    0 => Direction::South,
    1 => Direction::West,
    2 => Direction::North,
    _ => Direction::East,
  }
}

/// The relay port a peer at `peer` sits on, as upstream's `getAvailableNode` picks the half
/// of the relay the click was in. A horizontal relay gives on the side its `facing` points
/// to (its output nodes' anchors are there) and takes on the opposite one; a vertical relay
/// has the two halves the other way round, as `getNodeOffset` and `getAvailableNode` have
/// them.
fn relay_port_for_peer(facing: Direction, vertical: bool, relay: Pos, peer: Pos) -> RelayPort {
  let [dx, _, dz] = [peer[0] - relay[0], peer[1] - relay[1], peer[2] - relay[2]];
  let [nx, _, nz] = facing.normal();
  let towards_facing = dx * nx + dz * nz > 0;
  match (vertical, towards_facing) {
    (false, true) => RelayPort::Output,
    (false, false) => RelayPort::Input,
    (true, true) => RelayPort::Input,
    (true, false) => RelayPort::Output,
  }
}

/// The two sides a relay reads redstone from (`calculateInputStrength`): above and below
/// when vertical, the two sides across its long axis when horizontal. A redstone block or
/// wire on either side powers it; the connectors' own axis does not.
fn relay_sides(facing: Direction, vertical: bool) -> [Direction; 2] {
  if vertical {
    [Direction::Up, Direction::Down]
  } else {
    [clockwise(facing), counter_clockwise(facing)]
  }
}

/// Whether the two sides' signals power the relay: any signal at all does
/// (`shouldBePowered`: strength > 0).
fn relay_power_from(strengths: [i32; 2]) -> bool {
  strengths.into_iter().max().unwrap_or(0) > 0
}

/// One tick of the relay's bridge, as `WireGraph.tickBridges` does it: nothing while
/// unpowered; otherwise the in-flight buffer is topped up from the input pool and then
/// given to the output pool, at most 1,000 FE a tick either way. The buffer holds at most
/// one tick's transfer, so energy can wait there but never exceed the wire's rate.
fn relay_bridge(powered: bool, buffer: i32, input_ready: i32, output_room: i32) -> RelayMove {
  if !powered {
    return RelayMove::default();
  }
  let buffer = buffer.clamp(0, RELAY_TRANSFER);
  let into = (RELAY_TRANSFER - buffer)
    .min(input_ready)
    .clamp(0, RELAY_TRANSFER);
  let held = buffer + into;
  let out = held.min(output_room).clamp(0, RELAY_TRANSFER);
  RelayMove { into, out }
}

/// The lamp's hysteresis period in ticks:
/// `10 + (|x*31 + y*45 + z*33| % 7) * 3`, `SmallLightConnectorBlockEntity`'s
/// `posTimeOffset`, so a wall of lamps does not blink in step.
fn light_period(pos: Pos) -> i32 {
  const BASE: i64 = 10;
  const STEP: i64 = 3;
  let [x, y, z] = [i64::from(pos[0]), i64::from(pos[1]), i64::from(pos[2])];
  let seed = (x * 31 + y * 45 + z * 33).abs();
  i32::try_from(BASE + (seed % 7) * STEP).unwrap_or(i32::MAX)
}

/// One tick of the lamp's counter: it rises while the network paid the lamp and falls while
/// it did not, clamped to `±period`. Answers the new counter and, when this tick crossed a
/// threshold, the state the lamp takes: `Some(true)` lit, `Some(false)` dark, `None` hold.
fn light_tick(timer: i32, period: i32, drew: bool) -> (i32, Option<bool>) {
  let period = period.max(1);
  let step = if drew { 1 } else { -1 };
  let next = timer.saturating_add(step).clamp(-period, period);
  let switched = if next >= period {
    Some(true)
  } else if next <= -period {
    Some(false)
  } else {
    None
  };
  (next, switched)
}

/// The lamp's own tick, once per level pass: its counter rises when this tick's network pass
/// paid it its [`LIGHT_DRAW`] and falls when it could not, and the block's `powered` bit
/// follows the counter's clamp (`SmallLightConnectorBlockEntity.specialTick`). The bit is the
/// game's own block state, so the light engine and the jar's on/off models read it.
fn light_clock(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, saved: &Saved, drew: bool) {
  if saved.kind != Kind::Light {
    return;
  }
  let timer = int_at(&saved.nbt, TOGGLE_TIMER);
  let (next, switched) = light_tick(timer, light_period(pos), drew);
  if next != timer {
    let mut data = Compound::new();
    put_int(&mut data, TOGGLE_TIMER, next);
    if let Some(entity) = host.block_entity(level, pos) {
      let _ = entity.merge_data(host, data);
    }
  }
  let Some(lit) = switched else {
    return;
  };
  let wanted = if lit { "true" } else { "false" };
  if level.block_property(host, pos, POWERED).as_deref() != Some(wanted) {
    let _ = host.set_block_property(pos, POWERED, wanted);
    // The light engine keeps a cell's answer until it is asked again; a `dynamic_light` block
    // that answers from its state is asked when the state changes, and this is that ask.
    level.check_light(host, pos);
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

/// The one block entity type of the four blocks: it makes them tick, and its tick only queues
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
    spool_click(host, event)
  }
}

/// The relay's Part: where a placed one sits, and what a spool does with it. A relay takes
/// wires exactly as a connector does; only its two ports differ, and the link chooses one.
pub(crate) struct RelayShape;
#[mod_api::horizon_part]
impl BlockPart for RelayShape {
  /// A relay clicked on the floor lies flat along the placer's view (as upstream rotates the
  /// horizontal direction clockwise); one clicked onto a side stands vertical against the
  /// face it was placed on (`getStateForPlacement`).
  fn state_for_placement(
    &self,
    level: &dyn mod_api::LevelHost,
    event: &mut StateForPlacementEvent<'_>,
  ) -> Option<Option<mod_api::BlockState>> {
    let face = event.context.face;
    let (vertical, facing) = if face == Direction::Up || face == Direction::Down {
      (false, clockwise(yaw_facing(event.context.yaw)))
    } else {
      (true, face.opposite())
    };
    let placed = level.state_with_property(
      event.state,
      VERTICAL,
      if vertical { "true" } else { "false" },
    )?;
    let placed = level.state_with_property(placed, FACING, face_name(facing))?;
    Some(Some(placed))
  }

  /// A wire spool clicks a relay exactly as it clicks a connector.
  fn use_item_on(
    &self,
    host: &mut dyn ServerHost,
    event: &mut UseItemOnEvent<'_>,
  ) -> Option<InteractionResult> {
    spool_click(host, event)
  }
}

/// The lamp's Part: a connector's placement and spool click, plus the light its `powered` bit
/// asks for. The block declares `dynamic_light`, so the light engine asks this part at a cell
/// (`IBlockExtension.getLightEmission`) instead of taking the block's static `light`.
pub(crate) struct LightShape;

#[mod_api::horizon_part]
impl BlockPart for LightShape {
  /// A placed lamp is mounted on the face it was clicked against, as the small connector is.
  fn state_for_placement(
    &self,
    level: &dyn mod_api::LevelHost,
    event: &mut StateForPlacementEvent<'_>,
  ) -> Option<Option<mod_api::BlockState>> {
    let facing = face_name(event.context.face.opposite());
    let placed = level.state_with_property(event.state, FACING, facing)?;
    Some(Some(placed))
  }

  /// A wire spool clicks a lamp exactly as it clicks a connector.
  fn use_item_on(
    &self,
    host: &mut dyn ServerHost,
    event: &mut UseItemOnEvent<'_>,
  ) -> Option<InteractionResult> {
    spool_click(host, event)
  }

  /// The lamp glows at full light while its `powered` bit is set (upstream's
  /// `getLightEmission` answers 15 for the lit state and 0 for the dark one), and the answer
  /// is the state's alone, so it cannot drift from the bit the pass writes.
  fn light_emission(&self, host: &dyn Host, event: &mut LightEmissionEvent) -> Option<u8> {
    let keys = Keys::resolve(host)?;
    let lit = host
      .block_states()
      .matches(
        event.state,
        keys.light,
        &[StateConstraint::new(POWERED, "true")],
      )?
      .ok()?;
    lit.then_some(LIGHT_EMISSION)
  }
}

/// The accumulator's comparator half: `has_analog_output` says a comparator may ask, and
/// `analog_output` answers the fill the pass last computed for its tower.
pub(crate) struct AccumulatorShape {
  signals: Arc<Mutex<Signals>>,
}

impl AccumulatorShape {
  pub(crate) fn new(signals: Arc<Mutex<Signals>>) -> Self {
    Self { signals }
  }
}

#[mod_api::horizon_part]
impl BlockPart for AccumulatorShape {
  /// `ModularAccumulatorBlock.hasAnalogOutputSignal` answers `true` for every state of the
  /// block, so the comparator always asks (an empty tower then answers 0).
  fn has_analog_output(&self, level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
    if level.state_block(event.state) == Some(ACCUMULATOR) {
      event.output = true;
    }
  }

  /// `ModularAccumulatorBlock.getAnalogOutputSignal` asks the controller's `getFillState()` and
  /// maps it with `ComparatorUtil.fractionToRedstoneLevel`. The tower's pool is read in the pass
  /// (a level host cannot open a block entity), so this reports what that pass computed; the
  /// pass tells the comparators when the answer changed.
  fn analog_output(&self, _: &dyn LevelHost, event: &mut AnalogOutputEvent) {
    let dimension = event.dimension.key().id();
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    let answer = self
      .signals
      .lock()
      .expect("the createaddition comparator answers lock is held for one lookup")
      .value(dimension, pos);
    if let Some(level) = answer {
      event.output = level;
    }
  }
}

/// The spool click both wire parts share: the empty spool takes the clicked block's wires
/// out, a wire spool's first click selects it and its second lays the wire.
fn spool_click(
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
  let target_node = target.kind.wire_node()?;
  let Some(first) = pending_of(stack) else {
    // The first click: remember the block and the node it offers.
    let Some(node) = free_node(&target.nbt, target_node) else {
      feedback(host, refusal_text(Refusal::NodesFull));
      return Some(InteractionResult::Success);
    };
    offer(host, &place, Some((click.pos, node)));
    feedback(host, "Wire connector selected.");
    return Some(InteractionResult::Success);
  };
  let (from, _selected_node) = first;
  offer(host, &place, None);
  if from == click.pos {
    return Some(InteractionResult::Success);
  }
  // The first block may have gone since the first click: the click is still ours (0.1.0
  // claimed it before reading anything), but there is nothing left to link to.
  let Some(from_saved) = saved_at(host, level, keys, from) else {
    return Some(InteractionResult::Success);
  };
  let Some(from_node) = from_saved.kind.wire_node() else {
    return Some(InteractionResult::Success);
  };
  let proposal = Proposal {
    a: from,
    from: from_node,
    b: click.pos,
    to: target_node,
    wire,
  };
  match proposal.check(host, level, keys) {
    Link::Refused(reason) => feedback(host, refusal_text(reason)),
    Link::Linked => {
      if !proposal.store(host, level, keys) {
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

/// A wire a click or a command proposes between two wire nodes.
#[derive(Clone, Copy, Debug)]
struct Proposal {
  a: Pos,
  from: WireNode,
  b: Pos,
  to: WireNode,
  wire: Wire,
}

impl Proposal {
  /// The node a wire's `at` end goes in. A connector takes its first free node; a relay's
  /// port is decided by where the other end stands (its input or output half), and that
  /// port must still have a free node.
  fn node_for(
    self,
    host: &dyn Host,
    level: LevelRef,
    keys: Keys,
    at: Pos,
    node: WireNode,
    peer: Pos,
  ) -> Option<i32> {
    let saved = saved_at(host, level, keys, at)?;
    match node {
      WireNode::Relay => {
        let facing = level
          .block_property(host, at, FACING)
          .and_then(|name| face_named(&name))
          .unwrap_or(Direction::North);
        let vertical = level.block_property(host, at, VERTICAL).as_deref() == Some("true");
        let port = relay_port_for_peer(facing, vertical, at, peer);
        relay_free_node(&saved.nbt, port)
      }
      WireNode::Connector(_) => free_node(&saved.nbt, node),
    }
  }

  /// The rule check the spool and the command share: do both ends accept this wire?
  fn check(self, host: &dyn Host, level: LevelRef, keys: Keys) -> Link {
    let at = |pos: Pos| saved_at(host, level, keys, pos);
    let from_wires = at(self.a).map_or_else(Vec::new, |saved| saved_wires(&saved.nbt));
    let to_wires = at(self.b).map_or_else(Vec::new, |saved| saved_wires(&saved.nbt));
    let free_from = self.node_for(host, level, keys, self.a, self.from, self.b);
    let free_to = self.node_for(host, level, keys, self.b, self.to, self.a);
    plan_link(
      End {
        pos: self.a,
        connector: self.from.connector(),
        has_free_node: free_from.is_some(),
        linked_to_other: from_wires.iter().any(|saved| saved.pos == self.b),
      },
      End {
        pos: self.b,
        connector: self.to.connector(),
        has_free_node: free_to.is_some(),
        linked_to_other: to_wires.iter().any(|saved| saved.pos == self.a),
      },
      self.wire,
    )
  }

  /// Lay the wire in both blocks' saved compounds; answers whether both ends took it.
  fn store(self, host: &mut dyn ServerHost, level: LevelRef, keys: Keys) -> bool {
    let Some(from_node) = self.node_for(host, level, keys, self.a, self.from, self.b) else {
      return false;
    };
    let Some(to_node) = self.node_for(host, level, keys, self.b, self.to, self.a) else {
      return false;
    };
    let mut left =
      saved_at(host, level, keys, self.a).map_or_else(Vec::new, |saved| saved_wires(&saved.nbt));
    left.push(SavedWire {
      pos: self.b,
      node: from_node,
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
///
/// A wire is removed only where **both** ends are loaded and record each other: the peer's
/// reciprocal edit is committed and re-read first, then this connector's, and a spool is handed
/// back only for a wire confirmed gone from both sides. A peer that is unloaded, or whose saved
/// data does not update, keeps its edge and yields no refund — so the same wire cannot be
/// refunded twice and no one-sided edge is left behind for the network walk to treat as live.
fn remove_wires(host: &mut dyn ServerHost, level: LevelRef, keys: Keys, pos: Pos) {
  let Some(saved) = saved_at(host, level, keys, pos) else {
    return;
  };
  if saved.kind.wire_node().is_none() {
    return;
  }
  let wires = saved_wires(&saved.nbt);
  if wires.is_empty() {
    feedback(host, "This connector has no wire.");
    return;
  }
  let mut removed: Vec<SavedWire> = Vec::new();
  for wire in &wires {
    let Some(other) = saved_at(host, level, keys, wire.pos) else {
      continue;
    };
    if reciprocal_wires(&wires, pos, &saved_wires(&other.nbt), wire.pos)
      .iter()
      .all(|known| known.pos != wire.pos)
    {
      continue;
    }
    let kept: Vec<SavedWire> = saved_wires(&other.nbt)
      .into_iter()
      .filter(|saved| saved.pos != pos)
      .collect();
    let mut data = Compound::new();
    store_wires(&mut data, &kept);
    let stored = host
      .block_entity(level, wire.pos)
      .is_some_and(|entity| entity.merge_data(host, data).is_ok());
    if !stored {
      continue;
    }
    // Re-read the peer: only a saved list that no longer names this connector counts.
    let still_there = saved_at(host, level, keys, wire.pos)
      .is_some_and(|after| saved_wires(&after.nbt).iter().any(|saved| saved.pos == pos));
    if still_there {
      continue;
    }
    removed.push(wire.clone());
  }
  if removed.is_empty() {
    feedback(
      host,
      "No wire was removed: its other end is not loaded or did not answer.",
    );
    return;
  }
  let kept: Vec<SavedWire> = wires
    .iter()
    .filter(|wire| !removed.iter().any(|gone| gone.pos == wire.pos))
    .cloned()
    .collect();
  let mut data = Compound::new();
  store_wires(&mut data, &kept);
  let stored = host
    .block_entity(level, pos)
    .is_some_and(|entity| entity.merge_data(host, data).is_ok());
  // Refund together with the commit: only wires this connector's own saved list no longer
  // names, after a successful write, may hand a spool back.
  let confirmed: Vec<SavedWire> = if stored {
    saved_at(host, level, keys, pos)
      .map(|after| {
        let now = saved_wires(&after.nbt);
        removed
          .iter()
          .filter(|gone| !now.iter().any(|wire| wire.pos == gone.pos))
          .cloned()
          .collect()
      })
      .unwrap_or_default()
  } else {
    Vec::new()
  };
  if confirmed.is_empty() {
    feedback(host, "The wire was not removed.");
    return;
  }
  if let Some(player) = host.players().acting_player()
    && player.is_creative(host) != Some(true)
  {
    for (wire, count) in spools_back(&confirmed) {
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

/// The wires of `own` at `own_pos` that point at `peer` and that the peer records back: the
/// only ones a reciprocal removal may touch.
fn reciprocal_wires(
  own: &[SavedWire],
  own_pos: Pos,
  peer: &[SavedWire],
  peer_pos: Pos,
) -> Vec<SavedWire> {
  own
    .iter()
    .filter(|wire| wire.pos == peer_pos)
    .filter(|_| peer.iter().any(|known| known.pos == own_pos))
    .cloned()
    .collect()
}

// --- the level pass -------------------------------------------------------

/// Settle each queued connector's placement mode, let the queued accumulators feed the blocks
/// above and below, then run every wire network among the queued connectors.
pub(crate) fn level_ticked(
  host: &mut dyn ServerHost,
  level: LevelRef,
  queued: &BTreeSet<Pos>,
  signals: &Mutex<Signals>,
) {
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
    if saved.kind == Kind::Relay {
      update_relay(host, level, *pos);
    }
  }
  // Form the accumulators into towers before anything reads them: the pass's own reads, the
  // blocks' output sides and the energy capability all answer for the tower.
  let accumulators: BTreeSet<Pos> = blocks
    .iter()
    .filter(|(_, saved)| saved.kind == Kind::Accumulator)
    .map(|(pos, _)| *pos)
    .collect();
  let towers = plan_towers(&accumulators);
  reconcile_towers(host, level, &blocks, &towers);
  for (pos, saved) in &blocks {
    if saved.kind == Kind::Accumulator {
      let tower = towers.get(pos).copied().unwrap_or(Tower::single(*pos));
      accumulator_output(host, level, *pos, tower.role(*pos));
    }
  }
  let nodes: Vec<(PortMember, Vec<SavedWire>)> = blocks
    .iter()
    .flat_map(|(pos, saved)| ports_of(*pos, saved))
    .collect();
  // The lamps paid this tick, so each one's counter can rise only on a tick its network
  // actually gave it its FE. A lamp outside any network is not in `paid` and falls.
  let mut paid: BTreeSet<Pos> = BTreeSet::new();
  if nodes.len() >= 2 {
    let mut nodes = nodes;
    nodes.sort_by_key(|(member, _)| *member);
    let members: Vec<PortMember> = nodes.iter().map(|(member, _)| *member).collect();
    let wires: Vec<Vec<SavedWire>> = nodes.iter().map(|(_, wires)| wires.clone()).collect();
    for group in net_groups(&members, &wires) {
      if group.len() >= 2 {
        run_network(host, level, keys, &group, &mut paid);
      }
    }
  }
  // Every queued lamp ticks its own counter once, network or not: upstream's `specialTick`
  // runs before the mode check, so an idle or unlinked lamp still counts down and goes dark.
  for (pos, saved) in &blocks {
    if saved.kind == Kind::Light {
      light_clock(host, level, *pos, saved, paid.contains(pos));
    }
  }
  // The comparator answer is the fill *after* this tick's moves, so it is the last thing the
  // pass does: every accumulator block answers its tower's fraction, and a changed answer tells
  // the comparators beside it to read again.
  note_tower_signals(host, level, &blocks, &towers, signals);
}

/// Record each accumulator's comparator answer for the hook and tell the comparators beside a
/// block whose answer changed (`Level.updateNeighbourForOutputSignal`). `analog_output` is asked
/// with a level host, which cannot open a block entity, so the answer is computed here, from the
/// charges the pass just wrote.
fn note_tower_signals(
  host: &mut dyn ServerHost,
  level: LevelRef,
  blocks: &[(Pos, Saved)],
  towers: &BTreeMap<Pos, Tower>,
  signals: &Mutex<Signals>,
) {
  let dimension = host.dimension().to_owned();
  let changed: Vec<Pos> = {
    // The lock is never held across `update_neighbors`: the comparator that update wakes asks
    // `analog_output`, which takes the same lock.
    let mut answers = signals
      .lock()
      .expect("the createaddition comparator answers lock is held for one pass");
    let mut changed = Vec::new();
    for (pos, saved) in blocks {
      if saved.kind != Kind::Accumulator {
        continue;
      }
      let Some(tower) = towers.get(pos).copied() else {
        continue;
      };
      if answers.note(
        &dimension,
        *pos,
        tower_signal(tower, tower_amount(host, level, tower)),
      ) {
        changed.push(*pos);
      }
    }
    changed
  };
  for pos in changed {
    if let Some(state) = level.block_state(host, pos) {
      let _ = level.update_neighbors(host, pos, state, NeighbourUpdate::OutputSignal);
    }
  }
}

/// Read the relay's two redstone sides and keep its `powered` bit in step, as
/// `RedstoneRelayBlock.updateState`'s scheduled tick does: any signal on a side powers the
/// bridge, none unpowered it. The bit is the block's own state, so `ctl block` reads it.
fn update_relay(host: &mut dyn ServerHost, level: LevelRef, pos: Pos) {
  let facing = level
    .block_property(host, pos, FACING)
    .and_then(|name| face_named(&name))
    .unwrap_or(Direction::North);
  let vertical = level.block_property(host, pos, VERTICAL).as_deref() == Some("true");
  let powered = relay_powered(host, level, pos, facing, vertical);
  let wanted = if powered { "true" } else { "false" };
  if level.block_property(host, pos, POWERED).as_deref() != Some(wanted) {
    let _ = host.set_block_property(pos, POWERED, wanted);
  }
}

/// The signal the relay's two sides give it (`calculateInputStrength`): the neighbour's
/// signal towards the relay, and — on the horizontal sides — the strong power as well.
fn relay_powered(
  host: &dyn Host,
  level: LevelRef,
  pos: Pos,
  facing: Direction,
  vertical: bool,
) -> bool {
  let mut strengths = [0; 2];
  for (slot, side) in relay_sides(facing, vertical).into_iter().enumerate() {
    let neighbour = offset(pos, side);
    let towards = side.opposite();
    let signal = level.signal(host, neighbour, towards).unwrap_or(0);
    let direct = if vertical {
      0
    } else {
      level.direct_signal(host, neighbour, towards).unwrap_or(0)
    };
    strengths[slot] = signal.max(direct);
  }
  relay_power_from(strengths)
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
    Some(Kind::Relay) => Mode::None,
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
/// (`ModularAccumulatorBlockEntity.tickOutput`). Only the tower's highest block has its `top`
/// bit and only its lowest the `bottom` one, so a tower feeds only what stands above and below
/// it and never itself.
fn accumulator_output(
  host: &mut dyn ServerHost,
  level: LevelRef,
  pos: Pos,
  (top, bottom): (bool, bool),
) {
  let sides: &[Direction] = match (top, bottom) {
    (true, true) => &[Direction::Up, Direction::Down],
    (true, false) => &[Direction::Up],
    (false, true) => &[Direction::Down],
    (false, false) => &[],
  };
  for &side in sides {
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

/// Write the plan into the accumulators of this pass: each block's saved tower, and the `top`
/// / `bottom` state bits its role gives it (upstream's `notifyMultiUpdated`). Only a change is
/// written, so a settled tower costs two property reads a block a tick.
fn reconcile_towers(
  host: &mut dyn ServerHost,
  level: LevelRef,
  blocks: &[(Pos, Saved)],
  towers: &BTreeMap<Pos, Tower>,
) {
  for (pos, saved) in blocks {
    if saved.kind != Kind::Accumulator {
      continue;
    }
    let Some(tower) = towers.get(pos).copied() else {
      continue;
    };
    if !tower_is_saved(&saved.nbt, tower) {
      let mut data = Compound::new();
      save_tower(&mut data, tower);
      if let Some(entity) = host.block_entity(level, *pos) {
        let _ = entity.merge_data(host, data);
      }
    }
    let (top, bottom) = tower.role(*pos);
    for (key, wanted) in [("top", top), ("bottom", bottom)] {
      let value = if wanted { "true" } else { "false" };
      if level.block_property(host, *pos, key).as_deref() != Some(value) {
        let _ = host.set_block_property(*pos, key, value);
      }
    }
  }
}

/// The accumulator's energy as the game asks for it: the whole tower behind every one of its
/// blocks, as upstream's `handlerForCapability` gives each part the controller's storage.
///
/// The declared one-block buffer (`Kind::buffer`) is what the game answers with; this provider
/// is registered ahead of it, so a formed tower's pooled charge and its `blocks * 2,000,000`
/// capacity are what a neighbour sees, and one connector on any part drains the tower.
struct AccumulatorEnergy {
  level: LevelRef,
  pos: Pos,
}

impl AccumulatorEnergy {
  /// The tower this block belongs to, or its own single before the pass formed it.
  fn tower(&self, host: &dyn Host) -> Tower {
    host
      .block_entity(self.level, self.pos)
      .and_then(|entity| entity.data(host))
      .map_or_else(
        || Tower::single(self.pos),
        |nbt| tower_saved(&nbt, self.pos),
      )
  }

  /// Writes the charges of `before`/`after` back for the blocks that changed.
  fn write_charges(&self, host: &mut dyn ServerHost, changed: &[(Pos, i32)]) {
    for &(pos, charge) in changed {
      let mut data = Compound::new();
      put_int(&mut data, CHARGE, charge);
      if let Some(entity) = host.block_entity(self.level, pos) {
        let _ = entity.merge_data(host, data);
      }
    }
  }
}

impl mod_api::EnergyHandler for AccumulatorEnergy {
  fn amount(&self, host: &mut dyn ServerHost) -> u32 {
    let tower = self.tower(host);
    u32::try_from(tower_amount(host, self.level, tower).max(0)).unwrap_or(0)
  }

  fn capacity(&self, host: &mut dyn ServerHost) -> u32 {
    u32::try_from(self.tower(host).capacity()).unwrap_or(u32::MAX)
  }

  /// One call moves at most 5,000 FE, as the accumulator's declared buffer does
  /// (`accumulator_max_input`).
  fn insert(&self, host: &mut dyn ServerHost, _tx: &mod_api::Transaction, amount: u32) -> u32 {
    let want = i32::try_from(amount.min(TRANSFER_CAP)).unwrap_or(ACCUMULATOR_TRANSFER);
    let tower = self.tower(host);
    let mut charges = tower_charges(host, self.level, tower);
    let before = charges.clone();
    let added = store_tower(&mut charges, want);
    let changed: Vec<(Pos, i32)> = charges
      .iter()
      .zip(&before)
      .filter(|((_, after), (_, before))| after != before)
      .map(|((pos, after), _)| (*pos, *after))
      .collect();
    self.write_charges(host, &changed);
    u32::try_from(added).unwrap_or(0)
  }

  /// The same 5,000 FE a call a drain moves; a drain takes the tower's highest blocks first.
  fn extract(&self, host: &mut dyn ServerHost, _tx: &mod_api::Transaction, amount: u32) -> u32 {
    let want = i32::try_from(amount.min(TRANSFER_CAP)).unwrap_or(ACCUMULATOR_TRANSFER);
    let tower = self.tower(host);
    let mut charges = tower_charges(host, self.level, tower);
    let before = charges.clone();
    let taken = drain_tower(&mut charges, want);
    let changed: Vec<(Pos, i32)> = charges
      .iter()
      .zip(&before)
      .filter(|((_, after), (_, before))| after != before)
      .map(|((pos, after), _)| (*pos, *after))
      .collect();
    self.write_charges(host, &changed);
    u32::try_from(taken).unwrap_or(0)
  }
}

/// The most one capability call moves, as the accumulator's declared buffer allows.
const TRANSFER_CAP: u32 = ACCUMULATOR_TRANSFER as u32;

/// The accumulator's capability handler at one position, for `Registrar::block_capability_before_game`.
pub(crate) fn accumulator_energy(level: LevelRef, pos: Pos) -> Box<dyn mod_api::EnergyHandler> {
  Box::new(AccumulatorEnergy { level, pos })
}

// --- the modular accumulator tower ----------------------------------------

/// How many blocks wide an accumulator tower may be, each way
/// (`accumulator_max_width`, 3 by default).
const TOWER_WIDTH: i32 = 3;
/// How many blocks high an accumulator tower may be (`accumulator_max_height`, 5 by default).
const TOWER_HEIGHT: i32 = 5;

/// A formed accumulator tower: a square footprint `width` × `width` and `height` blocks high,
/// anchored at its lowest corner. Upstream's `CAConnectivityHandler` forms the same shape into
/// one controller, whose `getTotalAccumulatorSize()` is `width * width * height`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tower {
  /// The lowest corner: upstream's controller, and the anchor of the box.
  controller: Pos,
  /// The footprint's width, in blocks, both ways.
  width: i32,
  /// How many blocks high the tower is.
  height: i32,
}

impl Tower {
  /// One block on its own: what an accumulator is before (and when it never becomes) a tower.
  fn single(pos: Pos) -> Self {
    Self {
      controller: pos,
      width: 1,
      height: 1,
    }
  }

  /// How many blocks the tower is made of.
  fn blocks(self) -> i32 {
    self.width * self.width * self.height
  }

  /// The tower's whole capacity: 2,000,000 FE a block.
  fn capacity(self) -> i32 {
    self.blocks() * ACCUMULATOR_CAPACITY
  }

  /// The y of the tower's highest layer.
  fn top(self) -> i32 {
    self.controller[1] + self.height - 1
  }

  /// Whether `pos` is one of the tower's blocks. The comparison widens to `i64` so a saved
  /// controller near `i32::MAX` cannot overflow `controller + width` before the box is refused.
  fn contains(self, pos: Pos) -> bool {
    let [x, y, z] = self.controller;
    let (x, y, z) = (i64::from(x), i64::from(y), i64::from(z));
    let (px, py, pz) = (i64::from(pos[0]), i64::from(pos[1]), i64::from(pos[2]));
    px >= x
      && px < x + i64::from(self.width)
      && pz >= z
      && pz < z + i64::from(self.width)
      && py >= y
      && py < y + i64::from(self.height)
  }

  /// The tower's blocks, lowest layer first, the controller's own column before the others: the
  /// order energy fills in.
  fn members(self) -> Vec<Pos> {
    let mut members = Vec::with_capacity(usize::try_from(self.blocks()).unwrap_or(0));
    for y in 0..self.height {
      for z in 0..self.width {
        for x in 0..self.width {
          members.push([
            self.controller[0] + x,
            self.controller[1] + y,
            self.controller[2] + z,
          ]);
        }
      }
    }
    members
  }

  /// The `(top, bottom)` state bits of `pos`, as upstream's `notifyMultiUpdated` sets them: only
  /// the highest layer outputs to the block above, and only the lowest layer to the block below.
  fn role(self, pos: Pos) -> (bool, bool) {
    (pos[1] == self.top(), pos[1] == self.controller[1])
  }
}

/// An accumulator position ordered the way the plan takes anchors: lowest layer first, then x,
/// then z (upstream's controller order). A `BTreeSet` of these is the ordered anchor set, so the
/// pass takes the next anchor with one lookup instead of rescanning the remainder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Anchor(Pos);

impl Ord for Anchor {
  fn cmp(&self, other: &Self) -> Ordering {
    (self.0[1], self.0[0], self.0[2]).cmp(&(other.0[1], other.0[0], other.0[2]))
  }
}

impl PartialOrd for Anchor {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

/// Counts the ordered-set lookups a plan performs. Production uses [`Silent`] (no counter); the
/// linearity test uses [`Lookups`], so its evidence is a work unit and never a wall clock.
trait Work {
  /// The plan took the next anchor from the ordered set.
  fn anchor(&mut self) {}
  /// The plan probed one cell of a candidate box.
  fn probe(&mut self) {}
  /// The plan removed one member from the ordered set.
  fn remove(&mut self) {}
}

/// The production meter: a plan's work is not needed at run time, so nothing is counted.
struct Silent;

impl Work for Silent {}

/// The set lookups one plan performed, as the linearity test's work unit.
#[cfg(test)]
#[derive(Debug, Default, PartialEq, Eq)]
struct Lookups {
  /// Anchors taken, one per tower planned.
  anchors: usize,
  /// Candidate-box cells probed.
  probes: usize,
  /// Members removed.
  removals: usize,
}

#[cfg(test)]
impl Work for Lookups {
  fn anchor(&mut self) {
    self.anchors += 1;
  }
  fn probe(&mut self) {
    self.probes += 1;
  }
  fn remove(&mut self) {
    self.removals += 1;
  }
}

#[cfg(test)]
impl Lookups {
  fn total(&self) -> usize {
    self.anchors + self.probes + self.removals
  }
}

/// The ordered anchor set of one plan, with the meter it reports to.
struct Anchors<W: Work> {
  set: BTreeSet<Anchor>,
  work: W,
}

impl<W: Work> Anchors<W> {
  fn new(positions: &BTreeSet<Pos>, work: W) -> Self {
    Self {
      set: positions.iter().copied().map(Anchor).collect(),
      work,
    }
  }

  /// The lowest remaining anchor, one ordered-set lookup. It stays in the set so the box that
  /// starts at it can include it.
  fn first(&mut self) -> Option<Pos> {
    let anchor = self.set.first().copied()?;
    self.work.anchor();
    Some(anchor.0)
  }

  fn contains(&mut self, pos: Pos) -> bool {
    self.work.probe();
    self.set.contains(&Anchor(pos))
  }

  fn remove(&mut self, pos: Pos) {
    self.work.remove();
    self.set.remove(&Anchor(pos));
  }
}

/// The largest tower anchored at `anchor` whose blocks are all in `available`: upstream's
/// `tryToFormNewMulti`, which tries every width up to 3 and keeps the biggest box, a wider one
/// on a tie.
fn largest_tower<W: Work>(available: &mut Anchors<W>, anchor: Pos) -> Tower {
  let mut best = Tower::single(anchor);
  for width in 1..=TOWER_WIDTH {
    let mut height = 0;
    'layers: for step in 0..TOWER_HEIGHT {
      for x in 0..width {
        for z in 0..width {
          let at = [anchor[0] + x, anchor[1] + step, anchor[2] + z];
          if !available.contains(at) {
            break 'layers;
          }
        }
      }
      height += 1;
    }
    let candidate = Tower {
      controller: anchor,
      width,
      height,
    };
    if height > 0 && candidate.blocks() >= best.blocks() {
      best = candidate;
    }
  }
  best
}

/// Every accumulator of `positions` and the tower it belongs to, each tower anchored at the
/// lowest remaining block and filled greedily (upstream's own `formMulti` order). The ordered
/// anchor set makes one pass linear in the accumulators: one anchor lookup and one removal per
/// member, plus a constant number of box probes per tower — the remainder is never rescanned.
fn plan_towers_with<W: Work>(positions: &BTreeSet<Pos>, work: W) -> (BTreeMap<Pos, Tower>, W) {
  let mut left = Anchors::new(positions, work);
  let mut planned = BTreeMap::new();
  while let Some(anchor) = left.first() {
    let tower = largest_tower(&mut left, anchor);
    for member in tower.members() {
      left.remove(member);
      planned.insert(member, tower);
    }
  }
  (planned, left.work)
}

/// Every accumulator of `positions` and the tower it belongs to.
fn plan_towers(positions: &BTreeSet<Pos>) -> BTreeMap<Pos, Tower> {
  plan_towers_with(positions, Silent).0
}

/// The counted variant of [`plan_towers`], for the linearity test only.
#[cfg(test)]
fn plan_towers_counted(positions: &BTreeSet<Pos>) -> (BTreeMap<Pos, Tower>, Lookups) {
  plan_towers_with(positions, Lookups::default())
}

/// The tower a saved compound names, or the block's own single when it names none yet.
fn tower_saved(nbt: &Compound, pos: Pos) -> Tower {
  let width = int_at(nbt, TOWER_SAVED_WIDTH);
  let height = int_at(nbt, TOWER_SAVED_HEIGHT);
  // A save can be edited, so the shape is checked against the 3x3x5 limit *before* any
  // coordinate or capacity arithmetic: a doctored width/height could otherwise overflow
  // `capacity`, allocate an enormous `members()` vector, or make `controller + width` overflow.
  // An out-of-range shape is read as the block's own single, never multiplied.
  if !(1..=TOWER_WIDTH).contains(&width) || !(1..=TOWER_HEIGHT).contains(&height) {
    return Tower::single(pos);
  }
  let controller = [
    int_at(nbt, TOWER_SAVED_X),
    int_at(nbt, TOWER_SAVED_Y),
    int_at(nbt, TOWER_SAVED_Z),
  ];
  let tower = Tower {
    controller,
    width,
    height,
  };
  if tower.contains(pos) {
    tower
  } else {
    Tower::single(pos)
  }
}

/// Whether `nbt` already carries `tower`, so the pass writes only when it changed.
fn tower_is_saved(nbt: &Compound, tower: Tower) -> bool {
  let mut wanted = Compound::new();
  save_tower(&mut wanted, tower);
  [
    TOWER_SAVED_WIDTH,
    TOWER_SAVED_HEIGHT,
    TOWER_SAVED_X,
    TOWER_SAVED_Y,
    TOWER_SAVED_Z,
  ]
  .iter()
  .all(|key| wanted.get(&(*key).into()) == nbt.get(&(*key).into()))
}

/// Save `tower` into a compound: its shape and the controller's position.
fn save_tower(nbt: &mut Compound, tower: Tower) {
  put_int(nbt, TOWER_SAVED_WIDTH, tower.width);
  put_int(nbt, TOWER_SAVED_HEIGHT, tower.height);
  put_int(nbt, TOWER_SAVED_X, tower.controller[0]);
  put_int(nbt, TOWER_SAVED_Y, tower.controller[1]);
  put_int(nbt, TOWER_SAVED_Z, tower.controller[2]);
}

/// The charges of a tower's blocks that still stand, in fill order: the first `count` entries of
/// `tower.members()` that are still accumulators.
fn tower_charges(host: &dyn Host, level: LevelRef, tower: Tower) -> Vec<(Pos, i32)> {
  tower
    .members()
    .into_iter()
    .filter(|&pos| is_accumulator(host, level, pos))
    .map(|pos| {
      let charge = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(host))
        .map_or(0, |nbt| int_at(&nbt, CHARGE).clamp(0, ACCUMULATOR_CAPACITY));
      (pos, charge)
    })
    .collect()
}

/// What a tower holds: the sum of its blocks' charges.
fn tower_amount(host: &dyn Host, level: LevelRef, tower: Tower) -> i32 {
  tower_charges(host, level, tower)
    .into_iter()
    .map(|(_, charge)| charge)
    .sum()
}

/// Whether the block at `pos` is still one of the accumulator blocks.
fn is_accumulator(host: &dyn Host, level: LevelRef, pos: Pos) -> bool {
  let Some(block) = level.block(host, pos) else {
    return false;
  };
  host
    .block_key(ACCUMULATOR)
    .is_some_and(|accumulator| accumulator == block)
}

/// Adds up to `amount` FE to a tower's blocks, lowest first, each up to its own 2,000,000: how
/// much the tower took.
fn store_tower(charges: &mut [(Pos, i32)], amount: i32) -> i32 {
  let mut left = amount.max(0);
  let mut added = 0;
  for (_, charge) in charges.iter_mut() {
    let space = ACCUMULATOR_CAPACITY - *charge;
    let take = space.min(left);
    *charge += take;
    added += take;
    left -= take;
    if left == 0 {
      break;
    }
  }
  added
}

/// Takes up to `amount` FE out of a tower's blocks, highest first: how much came out.
fn drain_tower(charges: &mut [(Pos, i32)], amount: i32) -> i32 {
  let mut left = amount.max(0);
  let mut taken = 0;
  for (_, charge) in charges.iter_mut().rev() {
    let give = (*charge).min(left);
    *charge -= give;
    taken += give;
    left -= give;
    if left == 0 {
      break;
    }
  }
  taken
}

// --- the accumulator's comparator output ----------------------------------

/// `ComparatorUtil.fractionToRedstoneLevel`, verbatim from the parent's source
/// (`com.simibubi.create.foundation.blockEntity.ComparatorUtil`, Create 1.21.1):
/// `floor(clamp(frac * 14 + (frac > 0 ? 1 : 0), 0, 15))`. The extra 1 is what makes a container
/// just above empty read 1 and half full read 8 rather than 7.
///
/// The fraction itself is the jar's **float** (`ModularAccumulatorBlockEntity.getFillState()`
/// returns `(float) energyStored / capacity`), widened to the `double` this function takes, so
/// the f32 division is kept: at a 45-block tower's 90,000,000 FE the float and the tidy double
/// disagree on 18 exact charges.
fn fraction_to_level(stored: i32, capacity: i32) -> i32 {
  if capacity <= 0 {
    return 0;
  }
  let fraction = f64::from(stored.max(0) as f32 / capacity as f32);
  let extra = if fraction > 0.0 { 1.0 } else { 0.0 };
  let level = (fraction * 14.0 + extra).clamp(0.0, 15.0).floor();
  i32::try_from(level as i64).unwrap_or(15)
}

/// The signal a tower gives a comparator: its whole pool over its whole capacity, which is
/// upstream's `getFillState()` on the controller (`energyCapability.getEnergyStored()` /
/// `getMaxEnergyStored()`). A 1x3 tower of 6,000,000 FE at half charge therefore reads 8, where
/// one block holding 3,000,000 would read 15.
fn tower_signal(tower: Tower, stored: i32) -> i32 {
  fraction_to_level(stored, tower.capacity())
}

/// The comparator answer each accumulator block last gave, keyed by the dimension's id and then
/// the block position. `analog_output` is asked with a level host only, which cannot read a block
/// entity, so the pass computes the answer from the saved charges and this is what the hook
/// reads back (the same split `functionalstorage` uses at this pin).
///
/// One entry per placed accumulator; a broken block's entry is left behind (it is the answer the
/// block would still give, and a reloaded chunk's comparator may ask before the next pass runs).
#[derive(Default)]
pub(crate) struct Signals {
  levels: HashMap<String, HashMap<Pos, i32>>,
}

impl Signals {
  /// Record `value` as `pos`'s answer. `true` when it differs from the last one, which is when
  /// the comparators beside the block must be told to read again (`Level.updateNeighbourForOutputSignal`);
  /// a first answer counts as a change.
  fn note(&mut self, dimension: &str, pos: Pos, value: i32) -> bool {
    let previous = self
      .levels
      .entry(dimension.to_owned())
      .or_default()
      .insert(pos, value);
    previous != Some(value)
  }

  /// The last answer for `pos`, until the pass overwrites it.
  fn value(&self, dimension: &str, pos: Pos) -> Option<i32> {
    self.levels.get(dimension)?.get(&pos).copied()
  }
}

/// The wire ports a block offers: a connector's one, a relay's two (its saved wires split by
/// node: 0–3 input, 4–7 output), an accumulator's none.
fn ports_of(pos: Pos, saved: &Saved) -> Vec<(PortMember, Vec<SavedWire>)> {
  let wires = saved_wires(&saved.nbt);
  match saved.kind {
    Kind::Relay => {
      let (input, output): (Vec<SavedWire>, Vec<SavedWire>) = wires
        .into_iter()
        .partition(|wire| RelayPort::of_node(wire.node) == RelayPort::Input);
      vec![
        (PortMember::relay(pos, RelayPort::Input), input),
        (PortMember::relay(pos, RelayPort::Output), output),
      ]
    }
    kind if kind.wire_node().is_some() => vec![(PortMember::connector(pos), wires)],
    _ => Vec::new(),
  }
}

/// The connector groups of one pass, each a list of ports in the order they were given. Two
/// ports share a group when a wire joins them, directly or through others; the two ports of
/// one relay never do, because the relay bridges between them instead.
fn net_groups(members: &[PortMember], wires: &[Vec<SavedWire>]) -> Vec<Vec<PortMember>> {
  let mut parents: Vec<usize> = (0..members.len()).collect();
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
      // The peer's port is the one whose own list names this port back: a relay records
      // the wire under the node it went in, and that node is the port.
      for (other, member) in members.iter().enumerate() {
        if member.pos != wire.pos {
          continue;
        }
        if !wires[other].iter().any(|back| back.pos == members[at].pos) {
          continue;
        }
        let (a, b) = (root(&mut parents, at), root(&mut parents, other));
        if a != b {
          parents[a.max(b)] = a.min(b);
        }
      }
    }
  }
  let mut found: BTreeMap<usize, Vec<PortMember>> = BTreeMap::new();
  for (at, member) in members.iter().enumerate() {
    let group = root(&mut parents, at);
    found.entry(group).or_default().push(*member);
  }
  found.into_values().collect()
}

/// One port's state at the start of a network's tick.
struct WireMember {
  /// The port: its block positions and which port of it.
  at: PortMember,
  /// What the block is (a connector, or the relay port).
  kind: Kind,
  /// A connector's mode. A relay port has none: its role is the port.
  mode: Mode,
  /// The relay's in-flight buffer (unused for a connector).
  buffer: i32,
}

/// One network's tick: swap the buffer, feed every push connector from it, top the relay's
/// bridge up (or give its output network what the bridge holds), pay every lamp its
/// [`LIGHT_DRAW`], then draw every pull connector into it. The buffer lives in the saved
/// compound of the network's lowest-positioned member, so a reload cannot lose it; a relay's
/// in-flight buffer lives in the relay's own compound. `paid` collects the lamps this tick
/// actually paid, for their own counter.
fn run_network(
  host: &mut dyn ServerHost,
  level: LevelRef,
  keys: Keys,
  members: &[PortMember],
  paid: &mut BTreeSet<Pos>,
) {
  let nodes: Vec<WireMember> = members
    .iter()
    .filter_map(|&at| {
      let saved = saved_at(host, level, keys, at.pos)?;
      Some(WireMember {
        at,
        kind: saved.kind,
        mode: mode_saved(&saved.nbt).unwrap_or(Mode::None),
        buffer: int_at(&saved.nbt, CHARGE),
      })
    })
    .collect();
  if nodes.len() < 2 {
    return;
  }
  let Some(controller) = nodes.iter().map(|node| node.at.pos).min() else {
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
    for node in nodes
      .iter()
      .filter(|node| node.kind.connector().is_some() && node.mode == Mode::Push)
    {
      let Some(connector) = node.kind.connector() else {
        continue;
      };
      let Some((at, side)) = mounted(host, level, node.at.pos) else {
        continue;
      };
      let Some(sink) = level.capability(host, &ENERGY, at, Some(side)) else {
        continue;
      };
      let room = sink.capacity(host).saturating_sub(sink.amount(host));
      let amount = push_amount(
        connector.max_out(),
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
    // The relay's two ports are one conduit: the input port tops its buffer up out of the
    // input network's pool, the output port gives the output network's pool what the
    // buffer holds. The bridge is gated on `powered`; unpowered, nothing crosses.
    let mut buffers: BTreeMap<Pos, i32> = nodes
      .iter()
      .filter(|node| node.kind == Kind::Relay)
      .map(|node| (node.at.pos, node.buffer))
      .collect();
    for node in nodes.iter().filter(|node| node.kind == Kind::Relay) {
      let powered = level.block_property(host, node.at.pos, POWERED).as_deref() == Some("true");
      let buffer = buffers.get(&node.at.pos).copied().unwrap_or(0);
      if node.at.port == RelayPort::Input.index() {
        let intake = relay_bridge(powered, buffer, pool.ready, 0).into;
        let took = pool.take(intake);
        if took > 0 {
          buffers.insert(node.at.pos, buffer + took);
        }
        // The bridge's demand is what sizes the input network's buffer, as upstream's
        // `EnergyNetwork.demand` reads the far side's demand.
        wanted += intake;
      } else {
        let given = pool.put(
          relay_bridge(powered, buffer, 0, pool.room(capacity)).out,
          capacity,
        );
        if given > 0 {
          buffers.insert(node.at.pos, buffer - given);
        }
      }
    }
    // Every lamp asks its network for its 1 FE a tick (`specialTick`'s `network.demand(1)` and
    // `pull(consumption, false)`), whichever mode it is in: a lamp is a consumer, not a
    // connector. The demand sizes the buffer even in a tick that cannot pay; the draw itself
    // comes out of this tick's ready half, and only a lamp that got its FE counts that tick.
    for node in nodes.iter().filter(|node| node.kind == Kind::Light) {
      let took = pool.take(LIGHT_DRAW);
      wanted += LIGHT_DRAW;
      if took > 0 {
        paid.insert(node.at.pos);
      }
    }
    for node in nodes
      .iter()
      .filter(|node| node.kind.connector().is_some() && node.mode == Mode::Pull)
    {
      let Some(connector) = node.kind.connector() else {
        continue;
      };
      let Some((at, side)) = mounted(host, level, node.at.pos) else {
        continue;
      };
      let Some(source) = level.capability(host, &ENERGY, at, Some(side)) else {
        continue;
      };
      let amount = pull_amount(
        connector.max_in(),
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
    Ok::<_, ()>((pool, wanted, buffers))
  });
  let Ok((pool, wanted, buffers)) = outcome else {
    return;
  };
  let unchanged = int_at(&nbt, NET_READY) == pool.ready
    && int_at(&nbt, NET_NEXT) == pool.next
    && int_at(&nbt, NET_DEMAND) == wanted;
  if !unchanged {
    let mut data = Compound::new();
    put_int(&mut data, NET_READY, pool.ready);
    put_int(&mut data, NET_NEXT, pool.next);
    put_int(&mut data, NET_DEMAND, wanted);
    if let Some(entity) = host.block_entity(level, controller) {
      let _ = entity.merge_data(host, data);
    }
  }
  for (at, buffer) in buffers {
    if nodes
      .iter()
      .any(|node| node.at.pos == at && node.buffer == buffer)
    {
      continue;
    }
    let mut data = Compound::new();
    put_int(&mut data, CHARGE, buffer);
    if let Some(entity) = host.block_entity(level, at) {
      let _ = entity.merge_data(host, data);
    }
  }
}

// --- the commands ---------------------------------------------------------

/// The five usages `/createaddition` declares. The optional `tag:word` is the harness's marker:
/// the reply names it, so a line from an earlier command cannot be read as this one's answer.
pub(crate) fn command_usages() -> [&'static str; 5] {
  [
    "createaddition info <pos:block_pos> [tag:text]",
    "createaddition energy <pos:block_pos> <amount:integer> [tag:text]",
    "createaddition link <pos:block_pos> <pos2:block_pos> <wire:word> [tag:text]",
    "createaddition unlink <pos:block_pos> <pos2:block_pos> [tag:text]",
    "createaddition mode <pos:block_pos> <mode:word> [tag:text]",
  ]
}

/// The command tree: `info` and `mode` are ordinary player controls (level 0, with the reach
/// check below), while `energy`, `link` and `unlink` are administrative writes that stand in for
/// an item the player would have to hold (a spool) and are operator-only (level 2).
pub(crate) fn command_declarations() -> Vec<Command> {
  command_usages()
    .iter()
    .map(|usage| {
      let player_control =
        usage.starts_with("createaddition info") || usage.starts_with("createaddition mode");
      if player_control {
        Command::new(usage)
      } else {
        Command::op(usage, 2)
      }
    })
    .collect()
}

/// A player's interaction reach, in blocks: vanilla's ~4.5 plus a margin.
const REACH: f64 = 6.0;

/// Whether a block at `pos` is within a player at `player`: the ordinary interaction distance,
/// measured from the block's centre to the player's feet.
fn within_reach(player: [f64; 3], pos: Pos) -> bool {
  let dx = player[0] - (f64::from(pos[0]) + 0.5);
  let dy = player[1] - (f64::from(pos[1]) + 0.5);
  let dz = player[2] - (f64::from(pos[2]) + 0.5);
  dx * dx + dy * dy + dz * dz <= REACH * REACH
}

/// Whether the acting source may touch the block at `pos`: an operator may anywhere, a player
/// only within reach, and the console (which has no acting player) anywhere.
fn has_access(host: &dyn ServerHost, pos: Pos) -> bool {
  let Some(player) = host.players().acting_player() else {
    return true;
  };
  if player
    .permission_level(host)
    .is_some_and(|level| level >= 2)
  {
    return true;
  }
  player
    .entity()
    .position(host)
    .is_some_and(|at| within_reach(at, pos))
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
  if !has_access(host, pos) {
    feedback(host, "That block is too far away.");
    return;
  }
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

/// A block's line of text: an accumulator's charge, a connector's mode, buffer, links and
/// network buffer, or a relay's power bit, in-flight buffer and its two wire lists.
fn info_line(host: &dyn Host, level: LevelRef, pos: Pos, saved: &Saved) -> String {
  let time = level.game_time(host).unwrap_or(0);
  let Some(connector) = saved.kind.connector() else {
    if saved.kind == Kind::Relay {
      let wires = saved_wires(&saved.nbt);
      let (input, output): (Vec<SavedWire>, Vec<SavedWire>) = wires
        .into_iter()
        .partition(|wire| RelayPort::of_node(wire.node) == RelayPort::Input);
      let powered = level.block_property(host, pos, POWERED).as_deref() == Some("true");
      return format!(
        "{} at {},{},{}: powered {}, relay {}/{} FE, input [{}], output [{}], \
         pool {}/{}, demand {}, t={}",
        Kind::Relay.label(),
        pos[0],
        pos[1],
        pos[2],
        powered,
        int_at(&saved.nbt, CHARGE).clamp(0, RELAY_TRANSFER),
        RELAY_TRANSFER,
        links_text(&input),
        links_text(&output),
        int_at(&saved.nbt, NET_READY),
        int_at(&saved.nbt, NET_NEXT),
        int_at(&saved.nbt, NET_DEMAND),
        time
      );
    }
    let tower = tower_saved(&saved.nbt, pos);
    let charge = tower_amount(host, level, tower).max(0);
    let capacity = tower.capacity();
    if tower.blocks() == 1 {
      return format!(
        "{} at {},{},{}: {}/{} FE (t={})",
        Kind::Accumulator.label(),
        pos[0],
        pos[1],
        pos[2],
        charge,
        capacity,
        time
      );
    }
    let [x, y, z] = tower.controller;
    return format!(
      "{} at {},{},{}: {}/{} FE, tower {}x{} at {},{},{} (t={})",
      Kind::Accumulator.label(),
      pos[0],
      pos[1],
      pos[2],
      charge,
      capacity,
      tower.width,
      tower.height,
      x,
      y,
      z,
      time
    );
  };
  let charge = int_at(&saved.nbt, CHARGE).max(0);
  let links = links_text(&saved_wires(&saved.nbt));
  if saved.kind == Kind::Light {
    let period = light_period(pos);
    let timer = int_at(&saved.nbt, TOGGLE_TIMER).clamp(-period, period);
    let lit = level.block_property(host, pos, POWERED).as_deref() == Some("true");
    return format!(
      "{} at {},{},{}: mode {}, powered {}, lamp timer {}/{}, draw {} FE/t, \
       buffer {}/{} FE, links [{}], pool {}/{}, demand {}, t={}",
      Kind::Light.label(),
      pos[0],
      pos[1],
      pos[2],
      mode_saved(&saved.nbt).unwrap_or(Mode::None).name(),
      lit,
      timer,
      period,
      LIGHT_DRAW,
      charge,
      connector.max_in().min(connector.max_out()),
      links,
      int_at(&saved.nbt, NET_READY),
      int_at(&saved.nbt, NET_NEXT),
      int_at(&saved.nbt, NET_DEMAND),
      time
    );
  }
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

/// A block's saved wires as the readout prints them: `(x,y,z)/node/wire`, space separated.
fn links_text(wires: &[SavedWire]) -> String {
  wires
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
    .join(" ")
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
  if saved.kind == Kind::Accumulator {
    // An accumulator's charge is its whole tower's: seeding one block fills the pool, up to
    // the tower's capacity, so a formed tower can hold more than one block's 2,000,000.
    let tower = tower_saved(&saved.nbt, pos);
    let after = clamped(amount, tower.capacity());
    let mut charges = tower_charges(host, level, tower);
    for cell in &mut charges {
      cell.1 = 0;
    }
    store_tower(&mut charges, after);
    let wrote = host.transaction(|host, _tx| {
      for &(at, charge) in &charges {
        let mut data = Compound::new();
        put_int(&mut data, CHARGE, charge);
        if let Some(entity) = host.block_entity(level, at) {
          let _ = entity.merge_data(host, data);
        }
      }
      Ok::<(), ()>(())
    });
    if wrote.is_err() {
      feedback(host, "No Create Crafts & Additions block there.");
      return;
    }
    feedback(host, &reply(tag, format!("Energy set to {after} FE.")));
    return;
  }
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
  if !has_access(host, pos) {
    feedback(host, "That block is too far away.");
    return;
  }
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
  let (Some(from), Some(to)) = (first.kind.wire_node(), second.kind.wire_node()) else {
    return "Both positions must be connectors or a relay.".to_owned();
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
      if proposal.store(host, level, keys) {
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

  fn at(positions: &[[i32; 3]]) -> BTreeSet<Pos> {
    positions.iter().copied().collect()
  }

  fn charges(cells: &[[i32; 3]]) -> Vec<(Pos, i32)> {
    cells.iter().map(|&pos| (pos, 0)).collect()
  }

  fn column(base: Pos, height: i32) -> Vec<[i32; 3]> {
    (0..height)
      .map(|dy| [base[0], base[1] + dy, base[2]])
      .collect()
  }

  #[test]
  fn one_accumulator_is_a_single_block_tower() {
    let plan = plan_towers(&at(&[[0, 64, 0]]));
    let tower = plan[&[0, 64, 0]];
    assert_eq!(tower, Tower::single([0, 64, 0]));
    assert_eq!(tower.blocks(), 1);
    assert_eq!(tower.capacity(), ACCUMULATOR_CAPACITY);
    assert_eq!(tower.role([0, 64, 0]), (true, true));
  }

  #[test]
  fn a_column_of_three_is_one_tower_of_six_million_fe() {
    let blocks = column([3, 64, 7], 3);
    let plan = plan_towers(&at(&blocks));
    let tower = plan[&[3, 64, 7]];
    assert_eq!(tower.controller, [3, 64, 7]);
    assert_eq!((tower.width, tower.height), (1, 3));
    assert_eq!(tower.blocks(), 3);
    assert_eq!(tower.capacity(), 3 * ACCUMULATOR_CAPACITY);
    for block in &blocks {
      assert_eq!(
        plan[block], tower,
        "every block of the column is the same tower"
      );
    }
    assert_eq!(tower.role([3, 64, 7]), (false, true));
    assert_eq!(tower.role([3, 65, 7]), (false, false));
    assert_eq!(tower.role([3, 66, 7]), (true, false));
  }

  #[test]
  fn a_three_by_three_plate_is_one_tower_of_eighteen_million_fe() {
    let mut blocks = Vec::new();
    for x in 0..3 {
      for z in 0..3 {
        blocks.push([10 + x, 64, -2 + z]);
      }
    }
    let plan = plan_towers(&at(&blocks));
    let tower = plan[&[10, 64, -2]];
    assert_eq!((tower.width, tower.height), (3, 1));
    assert_eq!(tower.blocks(), 9);
    assert_eq!(tower.capacity(), 9 * ACCUMULATOR_CAPACITY);
  }

  #[test]
  fn a_flat_row_is_not_a_tower() {
    // The footprint is square: three blocks in a line are three towers of one, as upstream's
    // `tryToFormNewMultiOfWidth` finds no 2x2 or 3x3 layer.
    let plan = plan_towers(&at(&[[0, 64, 0], [1, 64, 0], [2, 64, 0]]));
    assert_eq!(plan.len(), 3);
    for (_, tower) in plan {
      assert_eq!(tower.blocks(), 1);
      assert_eq!(tower.capacity(), ACCUMULATOR_CAPACITY);
    }
  }

  #[test]
  fn a_column_of_six_is_a_five_high_tower_and_a_single() {
    let plan = plan_towers(&at(&column([0, 64, 0], 6)));
    let tower = plan[&[0, 64, 0]];
    assert_eq!((tower.width, tower.height), (1, 5));
    assert_eq!(tower.capacity(), 5 * ACCUMULATOR_CAPACITY);
    let above = plan[&[0, 69, 0]];
    assert_eq!(above, Tower::single([0, 69, 0]));
    assert_eq!(above.capacity(), ACCUMULATOR_CAPACITY);
    assert_eq!(tower.role([0, 68, 0]), (true, false));
  }

  #[test]
  fn a_cube_and_a_stray_block_form_the_largest_box_and_a_single() {
    let mut blocks = Vec::new();
    for x in 0..2 {
      for y in 0..2 {
        for z in 0..2 {
          blocks.push([x, 64 + y, z]);
        }
      }
    }
    blocks.push([2, 64, 0]);
    let plan = plan_towers(&at(&blocks));
    let cube = plan[&[0, 64, 0]];
    assert_eq!((cube.width, cube.height), (2, 2));
    assert_eq!(cube.capacity(), 8 * ACCUMULATOR_CAPACITY);
    assert_eq!(plan[&[2, 64, 0]], Tower::single([2, 64, 0]));
  }

  #[test]
  fn storing_fills_the_lowest_block_first_and_draining_takes_the_highest_first() {
    let tower = Tower {
      controller: [0, 64, 0],
      width: 1,
      height: 3,
    };
    let mut cells = charges(&tower.members());
    assert_eq!(store_tower(&mut cells, 5_000_000), 5_000_000);
    assert_eq!(
      cells.iter().map(|(_, charge)| *charge).collect::<Vec<_>>(),
      vec![ACCUMULATOR_CAPACITY, ACCUMULATOR_CAPACITY, 1_000_000]
    );
    // The tower is full at three blocks: a fourth block's worth is refused, not lost.
    assert_eq!(store_tower(&mut cells, 7_000_000), 1_000_000);
    assert_eq!(
      cells.iter().map(|(pos, _)| *pos).collect::<Vec<_>>(),
      tower.members()
    );
    // Draining starts at the top, so the bottom block alone never has to hold the whole amount:
    // the 6,000,000 the tower holds leave the top two blocks, and the bottom keeps its 2,000,000.
    assert_eq!(drain_tower(&mut cells, 4_000_000), 4_000_000);
    assert_eq!(
      cells.iter().map(|(_, charge)| *charge).collect::<Vec<_>>(),
      vec![ACCUMULATOR_CAPACITY, 0, 0]
    );
  }

  #[test]
  fn a_tower_never_gives_more_than_its_blocks_hold() {
    let tower = Tower {
      controller: [0, 64, 0],
      width: 1,
      height: 3,
    };
    let mut cells = charges(&tower.members());
    cells[1].1 = 1_000_000;
    assert_eq!(drain_tower(&mut cells, 9_000_000), 1_000_000);
    assert_eq!(cells.iter().map(|(_, charge)| *charge).sum::<i32>(), 0);
  }

  #[test]
  fn a_saved_tower_is_read_back_and_a_stale_one_falls_back_to_a_single() {
    let tower = Tower {
      controller: [4, 64, -1],
      width: 2,
      height: 2,
    };
    let mut nbt = Compound::new();
    save_tower(&mut nbt, tower);
    assert!(tower_is_saved(&nbt, tower));
    for member in tower.members() {
      assert_eq!(tower_saved(&nbt, member), tower);
    }
    // A compound whose saved box does not contain the block (a tower that moved under it)
    // is read as the block's own single, never as a box it is not in.
    assert_eq!(tower_saved(&nbt, [9, 64, 9]), Tower::single([9, 64, 9]));
    let mut elsewhere = Compound::new();
    save_tower(
      &mut elsewhere,
      Tower {
        controller: [4, 64, -1],
        width: 1,
        height: 3,
      },
    );
    assert!(!tower_is_saved(&elsewhere, tower));
  }

  /// A block entity compound naming a tower, as a doctored save can: `save_tower` writes any
  /// shape the planner could hand it, and the test deliberately hands it shapes the planner
  /// never builds.
  fn tower_nbt(controller: Pos, width: i32, height: i32) -> Compound {
    let mut nbt = Compound::new();
    save_tower(
      &mut nbt,
      Tower {
        controller,
        width,
        height,
      },
    );
    nbt
  }

  #[test]
  fn a_saved_tower_beyond_the_limit_is_read_as_a_single_block() {
    let pos = [4, 70, 9];
    for (width, height) in [
      (TOWER_WIDTH + 1, 1),
      (1, TOWER_HEIGHT + 1),
      (1_000, 1_000),
      (i32::MAX, i32::MAX),
      (0, 1),
      (1, 0),
      (-3, 5),
    ] {
      let nbt = tower_nbt(pos, width, height);
      let tower = tower_saved(&nbt, pos);
      assert_eq!(
        tower,
        Tower::single(pos),
        "a saved {width}x{height} tower must be refused before any arithmetic"
      );
      assert_eq!(tower.blocks(), 1, "width {width} height {height}");
      assert_eq!(
        tower.capacity(),
        ACCUMULATOR_CAPACITY,
        "width {width} height {height}"
      );
      assert_eq!(tower.members(), vec![pos], "width {width} height {height}");
    }
  }

  #[test]
  fn a_saved_controller_past_the_coordinate_limit_does_not_overflow() {
    // A legal box whose `controller + width` would overflow i32 (`i32::MAX - 2 + 3`): the read
    // must not panic, and the tower it returns must have sane arithmetic.
    let pos = [i32::MAX, 64, 0];
    let nbt = tower_nbt([i32::MAX - 2, 64, 0], TOWER_WIDTH, 1);
    let tower = tower_saved(&nbt, pos);
    assert_eq!((tower.width, tower.height), (TOWER_WIDTH, 1));
    assert_eq!(
      tower.capacity(),
      TOWER_WIDTH * TOWER_WIDTH * ACCUMULATOR_CAPACITY
    );
    assert_eq!(
      tower.members().len(),
      usize::try_from(TOWER_WIDTH * TOWER_WIDTH).unwrap()
    );

    // A controller nowhere near the block is refused, however large its coordinates.
    let elsewhere = tower_nbt([i32::MAX, i32::MAX, i32::MAX], TOWER_WIDTH, TOWER_HEIGHT);
    let pos = [0, 64, 0];
    assert_eq!(tower_saved(&elsewhere, pos), Tower::single(pos));
  }

  /// `count` accumulator blocks five apart on one layer: no two can form a box, so the plan
  /// makes `count` towers of one.
  fn disconnected(count: i32) -> BTreeSet<Pos> {
    (0..count).map(|i| [i * 5, 64, 0]).collect()
  }

  #[test]
  fn the_tower_plan_is_linear_in_the_number_of_disconnected_accumulators() {
    let base = plan_towers_counted(&disconnected(64)).1;
    let doubled = plan_towers_counted(&disconnected(128)).1;
    assert_eq!(
      doubled.anchors,
      2 * base.anchors,
      "doubling the accumulators must double the anchor work; rescanning the remainder for \
       every disconnected accumulator would quadruple it"
    );
    assert_eq!(base.anchors, 64, "one anchor taken per accumulator");
    assert_eq!(base.removals, 64, "one member removed per accumulator");
    assert_eq!(
      doubled.probes,
      2 * base.probes,
      "the box probes are a constant per anchor"
    );
    assert_eq!(
      doubled.total(),
      2 * base.total(),
      "the whole plan is linear: 64 accumulators take {base:?}, 128 take {doubled:?}"
    );
    // The exact work units, so a regression to a remainder scan is caught by a number and not
    // only by a ratio: each disconnected block costs one anchor, six box probes and one removal.
    assert_eq!(
      base,
      Lookups {
        anchors: 64,
        probes: 384,
        removals: 64,
      }
    );
    assert_eq!(
      doubled,
      Lookups {
        anchors: 128,
        probes: 768,
        removals: 128,
      }
    );
  }

  fn wire(pos: Pos) -> SavedWire {
    SavedWire {
      pos,
      node: 0,
      wire: Wire::Copper,
    }
  }

  #[test]
  fn a_wire_is_removed_only_where_the_peer_records_it() {
    let here = [0, 64, 0];
    let there = [1, 64, 0];
    let elsewhere = [2, 64, 0];
    let mine = vec![wire(there), wire(elsewhere)];
    // The peer records this connector: its edge is reciprocal and may go.
    assert_eq!(
      reciprocal_wires(&mine, here, &[wire(here)], there)
        .iter()
        .map(|known| known.pos)
        .collect::<Vec<_>>(),
      vec![there]
    );
    // The peer does not name this connector: no removal, so no second refund.
    assert!(reciprocal_wires(&mine, here, &[wire(elsewhere)], there).is_empty());
    assert!(reciprocal_wires(&mine, here, &[], there).is_empty());
  }

  #[test]
  fn the_mutating_commands_are_operator_only_and_the_controls_are_not() {
    let declared = command_declarations();
    let level_of = |root: &str| {
      declared
        .iter()
        .find(|command| command.usage.starts_with(root))
        .unwrap_or_else(|| panic!("{root} is declared"))
        .level
    };
    assert_eq!(level_of("createaddition energy"), 2);
    assert_eq!(level_of("createaddition link"), 2);
    assert_eq!(level_of("createaddition unlink"), 2);
    assert_eq!(level_of("createaddition mode"), 0, "a player control");
    assert_eq!(level_of("createaddition info"), 0, "a player readout");
  }

  #[test]
  fn a_block_beyond_reach_is_not_a_player_control() {
    let pos = [0, 64, 0];
    assert!(
      within_reach([0.5, 64.0, 4.5], pos),
      "four blocks away is in reach"
    );
    assert!(
      !within_reach([0.5, 64.0, 7.5], pos),
      "seven blocks away is not"
    );
    assert!(!within_reach([0.5, 74.0, 0.5], pos), "ten blocks up is not");
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
    let members = [
      PortMember::connector(a),
      PortMember::connector(b),
      PortMember::connector(c),
      PortMember::connector(alone),
    ];
    let found = net_groups(&members, &wires);
    assert_eq!(
      found,
      vec![members[..3].to_vec(), vec![PortMember::connector(alone)]]
    );
  }

  #[test]
  fn a_wire_to_a_connector_that_is_not_there_groups_nothing() {
    let a = [0, 0, 0];
    let found = net_groups(&[PortMember::connector(a)], &[vec![wire([99, 0, 0])]]);
    assert_eq!(found, vec![vec![PortMember::connector(a)]]);
  }

  // --- the redstone relay ---------------------------------------------------

  /// A saved wire at `pos` in `node`.
  fn node_wire(pos: Pos, node: i32) -> SavedWire {
    SavedWire {
      pos,
      node,
      wire: Wire::Copper,
    }
  }

  #[test]
  fn a_relay_has_eight_nodes_in_two_ports() {
    assert_eq!(RELAY_NODES, 8);
    assert_eq!(WireNode::Relay.nodes(), 8);
    assert_eq!(
      WireNode::Relay.connector().wire_length(),
      16,
      "a relay's wire is bounded like a small connector's"
    );
    assert_eq!(RelayPort::of_node(0), RelayPort::Input);
    assert_eq!(RelayPort::of_node(3), RelayPort::Input);
    assert_eq!(RelayPort::of_node(4), RelayPort::Output);
    assert_eq!(RelayPort::of_node(7), RelayPort::Output);
    assert_eq!(RelayPort::Input.index(), 0);
    assert_eq!(RelayPort::Output.index(), 1);
  }

  #[test]
  fn a_horizontal_relay_takes_on_the_far_side_and_gives_on_the_facing_side() {
    let relay = [10, 64, 0];
    assert_eq!(
      relay_port_for_peer(Direction::East, false, relay, [9, 64, 0]),
      RelayPort::Input
    );
    assert_eq!(
      relay_port_for_peer(Direction::East, false, relay, [11, 64, 0]),
      RelayPort::Output
    );
    assert_eq!(
      relay_port_for_peer(Direction::North, false, relay, [10, 64, 1]),
      RelayPort::Input
    );
    assert_eq!(
      relay_port_for_peer(Direction::North, false, relay, [10, 64, -1]),
      RelayPort::Output
    );
  }

  #[test]
  fn a_vertical_relay_reads_above_and_below_and_swaps_its_ports() {
    assert_eq!(
      relay_sides(Direction::East, true),
      [Direction::Up, Direction::Down]
    );
    assert_eq!(
      relay_sides(Direction::East, false),
      [Direction::South, Direction::North]
    );
    assert_eq!(
      relay_port_for_peer(Direction::East, true, [0, 64, 0], [1, 64, 0]),
      RelayPort::Input
    );
    assert_eq!(
      relay_port_for_peer(Direction::East, true, [0, 64, 0], [-1, 64, 0]),
      RelayPort::Output
    );
  }

  #[test]
  fn a_full_relay_port_refuses_rather_than_using_the_other_one() {
    let mut nbt = Compound::new();
    store_wires(
      &mut nbt,
      &[
        node_wire([1, 0, 0], 0),
        node_wire([2, 0, 0], 1),
        node_wire([3, 0, 0], 2),
        node_wire([4, 0, 0], 3),
      ],
    );
    assert_eq!(relay_free_node(&nbt, RelayPort::Input), None);
    assert_eq!(relay_free_node(&nbt, RelayPort::Output), Some(4));
    let empty = Compound::new();
    assert_eq!(relay_free_node(&empty, RelayPort::Input), Some(0));
    assert_eq!(relay_free_node(&empty, RelayPort::Output), Some(4));
  }

  #[test]
  fn an_unpowered_relay_carries_nothing_even_with_energy_in_flight() {
    assert_eq!(
      relay_bridge(false, 800, 1_000, 80_000),
      RelayMove::default(),
      "the redstone signal is the gate: no bridge may move anything unpowered"
    );
  }

  #[test]
  fn a_powered_relay_carries_a_thousand_fe_a_tick_and_keeps_the_rest() {
    // An empty buffer and 800 waiting: the whole 800 crosses this tick.
    assert_eq!(
      relay_bridge(true, 0, 800, 80_000),
      RelayMove {
        into: 800,
        out: 800
      }
    );
    // A 500 buffer and 900 more: the buffer caps at 1,000 and at most 1,000 leaves.
    assert_eq!(
      relay_bridge(true, 500, 900, 80_000),
      RelayMove {
        into: 500,
        out: 1_000
      }
    );
    // A full output network holds the rest in the relay, conserved.
    assert_eq!(
      relay_bridge(true, 500, 900, 300),
      RelayMove {
        into: 500,
        out: 300
      }
    );
    // The rate caps the intake however much the input network holds.
    assert_eq!(
      relay_bridge(true, 0, 5_000, 80_000),
      RelayMove {
        into: 1_000,
        out: 1_000
      }
    );
  }

  #[test]
  fn any_signal_on_either_side_powers_the_relay() {
    assert!(!relay_power_from([0, 0]));
    assert!(relay_power_from([15, 0]));
    assert!(relay_power_from([0, 1]));
  }

  #[test]
  fn a_relay_splits_a_wire_run_into_an_input_and_an_output_network() {
    let c1 = [0, 0, 0];
    let relay = [1, 0, 0];
    let c2 = [2, 0, 0];
    let extra = [0, 0, 1];
    let members = [
      PortMember::connector(c1),
      PortMember::relay(relay, RelayPort::Input),
      PortMember::relay(relay, RelayPort::Output),
      PortMember::connector(c2),
      PortMember::connector(extra),
    ];
    let wires = [
      vec![node_wire(relay, 0), node_wire(extra, 0)],
      vec![node_wire(c1, 0), node_wire(extra, 1)],
      vec![node_wire(c2, 4)],
      vec![node_wire(relay, 0)],
      vec![node_wire(c1, 0), node_wire(relay, 1)],
    ];
    let found = net_groups(&members, &wires);
    assert_eq!(
      found,
      vec![
        vec![members[0], members[1], members[4]],
        vec![members[2], members[3]]
      ],
      "the relay's two ports are separate networks, and the extra connector joins only the input one"
    );
  }

  #[test]
  fn the_relay_rig_moves_energy_only_while_powered() {
    // The e2e's rig in the rules: source connector → input network → relay → output
    // network → sink connector, with the pool ceilings the pass computes.
    let run = |powered: bool| {
      let mut source = 100_000;
      let mut sink = 0;
      let input_cap = buffer_capacity(2, RELAY_TRANSFER);
      let output_cap = buffer_capacity(2, RELAY_TRANSFER);
      let mut input = Pool::default();
      let mut output = Pool::default();
      let mut relay = 0;
      for _ in 0..20 {
        input.swap();
        let pulled = pull_amount(1_000, source, input.room(input_cap));
        input.put(pulled, input_cap);
        source -= pulled;
        let intake = relay_bridge(powered, relay, input.ready, 0).into;
        relay += input.take(intake);
        output.swap();
        let given = output.put(
          relay_bridge(powered, relay, 0, output.room(output_cap)).out,
          output_cap,
        );
        relay -= given;
        let pushed = push_amount(1_000, 2_000_000 - sink, output.ready);
        output.take(pushed);
        sink += pushed;
        assert_eq!(
          source + sink + input.total() + output.total() + relay,
          100_000,
          "energy moved through the relay, never made or lost"
        );
      }
      sink
    };
    assert_eq!(
      run(false),
      0,
      "an unpowered relay must not let one FE reach the sink"
    );
    assert!(
      run(true) > 0,
      "a powered relay must carry energy from its input to its output"
    );
  }

  #[test]
  fn a_player_facing_north_places_the_relay_east_west() {
    assert_eq!(yaw_facing(0.0), Direction::South);
    assert_eq!(yaw_facing(90.0), Direction::West);
    assert_eq!(yaw_facing(180.0), Direction::North);
    assert_eq!(yaw_facing(270.0), Direction::East);
    assert_eq!(yaw_facing(-90.0), Direction::East);
    assert_eq!(clockwise(Direction::North), Direction::East);
    assert_eq!(clockwise(Direction::East), Direction::South);
    assert_eq!(counter_clockwise(Direction::East), Direction::North);
    assert_eq!(counter_clockwise(Direction::North), Direction::West);
  }

  // --- the small light connector -------------------------------------------

  #[test]
  fn the_light_connector_is_a_small_connector_with_a_lamp() {
    assert_eq!(Kind::Light.connector(), Some(Connector::Small));
    assert_eq!(Kind::Light.label(), "Small Light Connector");
    assert_eq!(
      Kind::Light.buffer().capacity,
      Connector::Small.max_in().min(Connector::Small.max_out()),
      "a lamp is a small connector: its window is the small connector's 1,000 FE"
    );
    assert_eq!(
      Kind::Light.wire_node(),
      Some(WireNode::Connector(Connector::Small))
    );
    assert_eq!(LIGHT_DRAW, 1, "a lamp spends one FE a tick");
    assert_eq!(LIGHT_EMISSION, 15, "a lit lamp is a full light source");
  }

  #[test]
  fn a_lamps_period_is_its_own_ten_to_twenty_eight_tick_cycle() {
    // `SmallLightConnectorBlockEntity`: 10 + (|x*31 + y*45 + z*33| % 7) * 3.
    assert_eq!(light_period([0, 64, 0]), 19, "2880 % 7 = 3");
    assert_eq!(light_period([254, 100, 0]), 25, "12374 % 7 = 5");
    assert_eq!(
      light_period([-5, 64, 3]),
      19,
      "the seed is taken absolute: 2824 % 7 = 3"
    );
    assert_eq!(light_period([1, 64, 0]), 28, "2911 % 7 = 6");
    for x in -8..8 {
      for z in -8..8 {
        let period = light_period([x, 100, z]);
        assert!(
          (10..=28).contains(&period),
          "a lamp at {x},100,{z} cycles in {period} ticks, outside the jar's 10..=28"
        );
      }
    }
  }

  #[test]
  fn a_lamp_lights_after_its_period_of_energy_and_darkens_after_as_long_without() {
    let period = 19;
    // It starts dark, at a zero counter, and does not light before the period.
    assert_eq!(light_tick(0, period, true), (1, None));
    assert_eq!(light_tick(18, period, true), (19, Some(true)));
    assert_eq!(light_tick(19, period, false), (18, None));
    // The counter is clamped: a lit lamp asked for more stays lit, the counter held.
    assert_eq!(light_tick(19, period, true), (19, Some(true)));
    // From the lit clamp, losing energy for one period reaches zero, and two reaches the
    // other clamp — which is where the lamp goes dark.
    assert_eq!(light_tick(1, period, false), (0, None));
    assert_eq!(light_tick(-18, period, false), (-19, Some(false)));
    assert_eq!(light_tick(-19, period, false), (-19, Some(false)));
  }

  #[test]
  fn a_lamp_that_never_got_energy_goes_dark_and_stays_dark() {
    let period = 19;
    let (timer, switched) = light_tick(0, period, false);
    assert_eq!((timer, switched), (-1, None));
    assert_eq!(light_tick(-19, period, false), (-19, Some(false)));
    assert_eq!(light_tick(-19, period, true), (-18, None));
  }

  #[test]
  fn the_lamp_rig_draws_one_fe_a_tick_and_conserves_it() {
    // The e2e's rig in the rules: a source accumulator, a pull connector feeding the wire
    // network, and a lamp drawing its 1 FE a tick out of the network's pool. The pass order is
    // the code's: swap the halves, the lamp draws, the pull connector fills what is left.
    let mut source = 5_000;
    let capacity = buffer_capacity(2, LIGHT_DRAW);
    let mut pool = Pool::default();
    let mut consumed = 0;
    let mut drops = Vec::new();
    for _ in 0..20 {
      let before = source;
      pool.swap();
      consumed += pool.take(LIGHT_DRAW);
      let pulled = pull_amount(1_000, source, pool.room(capacity));
      pool.put(pulled, capacity);
      source -= pulled;
      drops.push(before - source);
      assert_eq!(
        source + pool.total() + consumed,
        5_000,
        "the lamp spends energy from the network, it never destroys more than it takes"
      );
    }
    assert_eq!(
      drops.iter().skip(2).copied().collect::<Vec<_>>(),
      vec![1; 18],
      "once the pool is full the lamp's 1 FE a tick is exactly one unit of the source a tick"
    );
    assert_eq!(consumed, 19, "the lamp drew in every tick but the first");
    assert_eq!(source + pool.total() + consumed, 5_000);
  }

  #[test]
  fn a_lamp_on_a_network_with_no_energy_draws_nothing() {
    // An unlinked lamp has no pool to draw from: the counter only falls, and no FE leaves
    // anything. This is the e2e's differential in the rules.
    let mut pool = Pool::default();
    assert_eq!(pool.take(LIGHT_DRAW), 0);
    let (timer, switched) = light_tick(0, light_period([0, 64, 0]), false);
    assert_eq!((timer, switched), (-1, None));
    assert_eq!(pool.total(), 0);
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

  #[test]
  fn the_comparator_level_is_the_jars_fraction_mapping() {
    // `ComparatorUtil.fractionToRedstoneLevel` (read from the parent's source): a 1x3 tower is
    // 6,000,000 FE, and the answer is `floor(frac * 14 + (frac > 0 ? 1 : 0))`, clamped to 15.
    let capacity = 6 * ACCUMULATOR_CAPACITY;
    assert_eq!(fraction_to_level(0, capacity), 0, "an empty tower reads 0");
    assert_eq!(
      fraction_to_level(capacity / 2, capacity),
      8,
      "half full is 8, not 7: the jar's +1 (a 0-vs-15 run would read 15)"
    );
    assert_eq!(
      fraction_to_level(1, capacity),
      1,
      "any charge at all reads 1"
    );
    assert_eq!(
      fraction_to_level(capacity - 1, capacity),
      14,
      "only a tower filled exactly to its capacity reads 15"
    );
    assert_eq!(fraction_to_level(capacity, capacity), 15, "full reads 15");
    assert_eq!(
      fraction_to_level(capacity + 5, capacity),
      15,
      "a save past its capacity is clamped, not wrapped"
    );
    // The 1/14 boundary on one block's own 2,000,000: `floor(f * 14)` steps at 142,857.
    assert_eq!(fraction_to_level(142_857, ACCUMULATOR_CAPACITY), 1);
    assert_eq!(fraction_to_level(142_858, ACCUMULATOR_CAPACITY), 2);
  }

  #[test]
  fn the_fraction_is_the_jars_single_precision_division() {
    // Upstream's `getFillState()` is a `float`: `(float) stored / capacity` is one f32 division,
    // and only its result is widened to the `double` that the parent's
    // `fractionToRedstoneLevel(double)` takes. A 3x3x5 tower is 90,000,000 FE, whose f32 ulp is
    // 8, and at 19,285,714 FE the jar's float division gives `frac * 14` just past 3 (level 4)
    // where the tidy double gives just under it (level 3). Matching the jar means the float.
    assert_eq!(fraction_to_level(19_285_714, 90_000_000), 4);
    assert_eq!(
      fraction_to_level(45_000_000, 90_000_000),
      8,
      "a half-full 45-block tower is still 8"
    );
  }

  #[test]
  fn a_towers_level_uses_the_towers_capacity_not_one_blocks() {
    let tower = Tower {
      controller: [4, 64, 9],
      width: 1,
      height: 3,
    };
    assert_eq!(
      tower_signal(tower, 3_000_000),
      8,
      "a half-full 1x3 tower reads the tower's fraction, not a block's"
    );
    assert_eq!(
      tower_signal(Tower::single([4, 64, 9]), 3_000_000),
      15,
      "the same charge in one block is past its 2,000,000 capacity"
    );
    assert_eq!(tower_signal(tower, 0), 0);
    assert_eq!(tower_signal(tower, tower.capacity()), 15);
  }

  #[test]
  fn a_comparator_is_told_only_when_its_answer_changes() {
    let mut signals = Signals::default();
    let pos = [10, 100, 0];
    assert_eq!(
      signals.value("minecraft:the_end", pos),
      None,
      "no answer before the pass has read the block"
    );
    assert!(
      signals.note("minecraft:the_end", pos, 8),
      "the first answer is a change, so the comparator is told"
    );
    assert_eq!(signals.value("minecraft:the_end", pos), Some(8));
    assert!(
      !signals.note("minecraft:the_end", pos, 8),
      "the same answer tells nobody, so a settled tower costs no redstone update"
    );
    assert!(
      signals.note("minecraft:the_end", pos, 15),
      "a new answer is a change"
    );
    assert!(
      signals.note("minecraft:overworld", pos, 0),
      "another dimension's block at the same coordinates is a first answer"
    );
    assert!(
      signals.note("minecraft:the_end", [11, 100, 0], 8),
      "another position is a first answer"
    );
    assert_eq!(signals.value("minecraft:overworld", pos), Some(0));
    assert_eq!(
      signals.value("minecraft:the_end", pos),
      Some(15),
      "the tower's own answer is kept"
    );
  }
}
