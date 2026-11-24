//! Create Crafts & Additions — the one `impl Hooks` at the crate root.
//!
//! The FE half only: the two connectors with their spools and wire links, the wire network that
//! moves energy between them at the end of the level's tick, the modular accumulator that
//! feeds the blocks on its UP and DOWN faces, the redstone relay that bridges one wire
//! network into another while its `powered` bit is set, the small light connector whose
//! lamp draws 1 FE a tick from its network and lights while it is paid, and the creative
//! generator that gives without limit and fills every neighbour each tick.
//!
//! The rules themselves live in [`runtime`], byte-identical to the retired bundle's `runtime.rs`
//! and tested by `cargo test` without the game. [`network`] is the glue: the block entity Part,
//! the wire links and the commands.

pub mod render;

pub mod runtime;

mod network;

use std::sync::{Arc, Mutex};

use mod_api::{
  self, ChunkUnloadEvent, Flow, Hooks, HostSide, LevelTickPostEvent, LevelUnloadEvent,
  PlayerLoggedOutEvent, RegisterCommandsEvent, Registrar, RegistrationContext, RunModCommandEvent,
  ServerHost,
};

use crate::network::{
  AccumulatorShape, ConnectorShape, Kind, LightShape, Pending, Queue, RelayShape, Signals,
  Supports, Ticker,
};

/// The mod's session state: which of its block entities ticked this tick, per level, and the
/// accumulator's last comparator answer per block.
#[derive(Default)]
struct Mod {
  queue: Arc<Mutex<Queue>>,
  signals: Arc<Mutex<Signals>>,
  pending: Arc<Mutex<Pending>>,
  supports: Arc<Mutex<Supports>>,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
  /// Declare the one block entity type the five blocks share (its tick queues the level's
  /// pass), attach the Parts, and declare the energy buffer of each block.
  fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
    let small = registrar.block_key(network::CONNECTOR);
    let large = registrar.block_key(network::LARGE_CONNECTOR);
    let accumulator = registrar.block_key(network::ACCUMULATOR);
    let relay = registrar.block_key(network::RELAY);
    let light = registrar.block_key(network::LIGHT_CONNECTOR);
    let creative = registrar.block_key(network::CREATIVE);

    let kind = registrar
      .block_entity_type("energy")
      .blocks(&[
        "connector",
        "large_connector",
        "modular_accumulator",
        "redstone_relay",
        "small_light_connector",
        "creative_energy",
      ])
      .build();
    let attached = registrar.block_entity_part(&kind, Ticker::new(Arc::clone(&self.queue)));
    debug_assert!(
      attached.is_ok(),
      "the energy part attaches once: {attached:?}"
    );

    // Every wire block's Part shares one pending-selection store: the first spool click selects a
    // node, the second lays the wire, and a spool's selection is the player's (`network::Pending`),
    // not the stack's — the SDK cannot rewrite a held stack in place. The same Parts share one
    // support queue: a neighbour change under a placed connector may have taken its support, and
    // the removal waits for this level's pass, where the exact face-sturdy query can run.
    registrar.block_part(
      small,
      ConnectorShape::new(Arc::clone(&self.pending), Arc::clone(&self.supports)),
    );
    registrar.block_part(
      large,
      ConnectorShape::new(Arc::clone(&self.pending), Arc::clone(&self.supports)),
    );
    registrar.block_part(
      relay,
      RelayShape::new(Arc::clone(&self.pending), Arc::clone(&self.supports)),
    );
    // The lamp is a connector with its own Part: the same placement and spool click, plus the
    // `light_emission` answer its `dynamic_light` declaration asks for.
    registrar.block_part(
      light,
      LightShape::new(Arc::clone(&self.pending), Arc::clone(&self.supports)),
    );
    // The accumulator answers a comparator: `has_analog_output` says it may be asked and
    // `analog_output` gives the tower's fill, which the level pass computes and records.
    registrar.block_part(
      accumulator,
      AccumulatorShape::new(Arc::clone(&self.signals)),
    );

    registrar.block_energy(&small, Kind::Small.buffer());
    registrar.block_energy(&large, Kind::Large.buffer());
    registrar.block_energy(&accumulator, Kind::Accumulator.buffer());
    registrar.block_energy(&light, Kind::Light.buffer());

    // The accumulator answers the ENERGY capability itself, ahead of the game's own one-block
    // buffer: a formed tower's pooled charge and its `blocks * 2,000,000` capacity are what a
    // neighbour sees, as upstream's controller answers for every part of the multiblock
    // (`handlerForCapability`). The connectors keep the declared buffer.
    registrar.block_capability_before_game(&mod_api::ENERGY, &accumulator, |_, at, _, entity| {
      entity.map(|_| network::accumulator_energy(at.level, at.pos))
    });

    // The creative generator is an unlimited source: its capability answers infinity, gives
    // whatever is asked and takes nothing (`CreativeEnergyStorage`), registered ahead of any
    // declaration because there is no amount to keep.
    registrar.block_capability_before_game(&mod_api::ENERGY, &creative, |_, _, _, entity| {
      entity.map(|_| network::creative_supply())
    });
  }

  /// After the level's tick: one pass over the blocks whose tick ran, so a network of many
  /// connectors moves energy once per tick and not once per block.
  fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
    let HostSide::Server(host) = host else {
      return;
    };
    // The pass reads blocks and moves energy through this level's capability, and
    // `LevelRef::capability` answers only for the level the server ticks now. Outside that
    // window the queue is left alone and the next tick retries: the queue is bounded by the
    // number of our blocks, so waiting costs nothing.
    if !event.level.in_place(host) {
      return;
    }
    let queued = self
      .queue
      .lock()
      .expect("the createaddition tick queue lock is held for one drain")
      .take(event.level);
    network::level_ticked(
      host,
      event.level,
      &queued,
      &self.signals,
      &self.supports,
      &self.pending,
    );
  }

  /// `/createaddition info|energy|link|unlink|mode|signals`: the harness's readout and the controls
  /// the GUI and Create's wrench would give a player (neither is in this slice).
  fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
    event.commands.extend(network::command_declarations());
  }

  /// A chunk left the world: the comparator answers of the accumulators in it go with it. The
  /// event names the dimension and the chunk (`[x, z]`), so the clear needs no block read — a
  /// chunk that leaves the server has already been saved and its handles read nothing.
  ///
  /// `chunk_unload`, not `on_chunk_unloaded`: that one is per block entity and carries no
  /// dimension (only the block entity's handle, whose reads the hook's own doc says find nothing
  /// on a server by then), and this map is keyed by the dimension's name.
  fn chunk_unload(&self, _host: &mut dyn ServerHost, event: &mut ChunkUnloadEvent<'_>) {
    let _ = self
      .signals
      .lock()
      .expect("the createaddition comparator answers lock is held for one chunk unload")
      .forget_chunk(event.dimension, event.chunk);
  }

  /// A level closed, after its last save: drop everything the mod kept for it. That is the
  /// session's own clear as well — every open level closes this way when the server stops, before
  /// `WorldCloseFinal`, and `chunk_unload` is not told at a world close.
  fn level_unload(&self, _host: &mut dyn ServerHost, event: &mut LevelUnloadEvent<'_>) {
    let _ = self
      .signals
      .lock()
      .expect("the createaddition comparator answers lock is held for one level unload")
      .forget_dimension(event.dimension);
    let _ = self
      .queue
      .lock()
      .expect("the createaddition tick queue lock is held for one level unload")
      .forget_dimension(event.dimension);
    // A support check queued for that level can never run now.
    let _ = self
      .supports
      .lock()
      .expect("the createaddition support queue lock is held for one level unload")
      .forget_dimension(event.dimension);
    // A spool's first click is a click in progress: no half-made link survives the level it was
    // made in. The store is per player, so a close ends every player's (declared in `notes.md`).
    let _ = self
      .pending
      .lock()
      .expect("the createaddition pending-selection lock is held for one level unload")
      .clear();
  }

  /// A player left: their spool selection was a click in progress, so it goes with them instead
  /// of staying in the mod for the rest of the session.
  fn player_logged_out(&self, _host: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
    self
      .pending
      .lock()
      .expect("the createaddition pending-selection lock is held for one logout")
      .forget(event.player);
  }

  /// Run the six usages [`Hooks::register_commands`] declares, after the game parsed and
  /// permission-checked them.
  fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
    let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
    match literals.as_slice() {
      ["createaddition", "info"] => network::command_info(host, event.command),
      ["createaddition", "energy"] => network::command_energy(host, event.command),
      ["createaddition", "link"] => network::command_link(host, event.command),
      ["createaddition", "unlink"] => network::command_unlink(host, event.command),
      ["createaddition", "mode"] => network::command_mode(host, event.command),
      ["createaddition", "signals"] => {
        network::command_signals(host, event.command, &self.signals, &self.pending)
      }
      _ => return Flow::Continue,
    }
    Flow::Cancel
  }
}
