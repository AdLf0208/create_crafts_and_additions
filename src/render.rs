//! The client half of the wire family: what a wire looks like, as geometry and colour.
//!
//! Upstream draws every wire in `WireNodeRenderer` (`FINDINGS.md`, read from the jar's source):
//! a ribbon hung between the two nodes it links, sagging on the jar's own catenary, in the
//! `WireType`'s RGB, and — for the festive wire — a string of small lights every third point.
//! This module is that description as pure arithmetic: no `mod_api`, no `wgpu`, no world. The
//! helper the package patches into the renderer (`src/mixins/world_drawn.rs`) asks [`frame`] for
//! clip-space vertices and uploads them; [`publish`] is what the level's tick pass fills, so the
//! renderer draws the links the server actually holds.
//!
//! The rules here are the jar's constants, not invented ones:
//!
//! - a wire hangs by `sin(-part * PI) * (HANG * distance / SMALL_CONNECTOR_MAX_LENGTH)`, with
//!   `HANG = 0.5` and the 16-block small connector length — `WireNodeRenderer.hang`;
//! - its cross-section is `0.025` blocks — the `0.025F` its two `wirePart` calls draw with;
//! - every other point is drawn at 70 % brightness — `wireVert`'s `cr *= 0.7F`;
//! - a connector's node sits `3/16` of a block out of its mounting face, a relay's `5/16` along
//!   its own axis plus `-1/16` down (`getNodeOffset` of each block entity);
//! - a festive wire's lights cycle red, green and blue every third point
//!   (`WireNodeRenderer.colors`, `index % 3 == 0`).

use std::sync::Mutex;

/// A wire kind, as the four `WireType`s the jar can draw. The RGB is the jar's own
/// `getRed`/`getGreen`/`getBlue` (see `notes.md` § *The wire types*), so a festive wire is the
/// dark green 26/94/12 and not a colour of this port's choosing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireKind {
  /// `WireType.COPPER`: 256 transfer, low current.
  Copper,
  /// `WireType.GOLD`: 1024 transfer, high current.
  Gold,
  /// `WireType.ELECTRUM`: 8196 transfer, high current.
  Electrum,
  /// `WireType.FESTIVE`: 256 transfer, low current, and a string of lights.
  Festive,
}

impl WireKind {
  /// The jar's colour for this wire.
  pub fn rgb(self) -> [u8; 3] {
    match self {
      WireKind::Copper => [78, 37, 30],
      WireKind::Gold => [98, 83, 29],
      WireKind::Electrum => [88, 66, 37],
      WireKind::Festive => [26, 94, 12],
    }
  }

  /// Whether the wire carries the string of lights (`WireType.isFestive`).
  pub fn festive(self) -> bool {
    self == WireKind::Festive
  }
}

/// One wire as the renderer needs it: the world-space point each end attaches at, and its kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wire {
  /// Where the wire leaves one block, in world coordinates.
  pub from: [f64; 3],
  /// Where it arrives at the other, in world coordinates.
  pub to: [f64; 3],
  /// Which wire it is.
  pub kind: WireKind,
  /// The block/sky light at each end, as the level pass read it. The renderer interpolates this
  /// along the wire the way the jar's `wirePart` does (`LightTexture` lightmap coordinates).
  pub light: Light,
}

/// The light a wire is drawn under: block and sky light at its two block ends.
///
/// `block[i]`/`sky[i]` are the raw 0..15 levels the level pass read with `getBrightness`, for the
/// wire's own block (`i = 0`) and its peer (`i = 1`). This is the data the jar's `wireRender`
/// reads before calling `wirePart`; the per-segment interpolation happens in the renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
  pub block: [i32; 2],
  pub sky: [i32; 2],
}

impl Light {
  /// Fully lit, for callers that have no level to read (and for the wire unit tests that pin
  /// geometry and colour independently of light).
  pub const BRIGHT: Light = Light {
    block: [15, 15],
    sky: [15, 15],
  };
}

/// One vertex the helper uploads: a clip-space position and an RGBA colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
  /// Clip space, as the sampled frame's own matrix gives it.
  pub clip: [f32; 4],
  /// Linear RGBA, 0 to 1.
  pub rgba: [f32; 4],
}

/// The number of straight pieces a wire is drawn in (`WireNodeRenderer`'s `j < 24`).
pub const SEGMENTS: usize = 24;
/// The jar's sag constant (`WireNodeRenderer.HANG`).
pub const HANG: f32 = 0.5;
/// The small connector's length in blocks, the sag's divisor
/// (`CommonConfig.SMALL_CONNECTOR_MAX_LENGTH`, 16).
pub const MAX_LENGTH: f32 = 16.0;
/// Half the wire's cross-section: the jar draws `0.025F` across (`wirePart`'s `a`).
pub const HALF: f32 = 0.0125;
/// A festive light's half-size, in blocks: the jar's `SMALL_LIGHT` partial at its usual scale.
pub const LIGHT: f32 = 0.02;
/// The `y` a festive light hangs below its point (`WireNodeRenderer.LIGHT_Y_OFFSET`).
pub const LIGHT_Y_OFFSET: f32 = -0.03;
/// Beyond this many blocks a wire is not drawn: the connector's own 16-block reach is the
/// jar's limit, and a wire is never longer than the largest connector's reach.
const FAR: f32 = 64.0;

/// The wire links the client draws, and the dimension they belong to.
///
/// The server's level pass fills this once a tick ([`publish`]); the renderer only reads it. A
/// level handles its own wires, so a publish from another dimension replaces the set instead of
/// adding to it: only one level ticks in a single-player world, and the old dimension's wires
/// belong to a world the player left.
#[derive(Debug, Default)]
struct Cache {
  dimension: String,
  wires: Vec<Wire>,
}

impl Cache {
  /// Replace the set. `dimension` names the level the wires were read in (`minecraft:the_end`);
  /// a different one starts a new set.
  fn set(&mut self, dimension: &str, wires: Vec<Wire>) {
    if self.dimension != dimension {
      self.dimension.clear();
      self.dimension.push_str(dimension);
    }
    self.wires = wires;
  }
}

static CACHE: Mutex<Cache> = Mutex::new(Cache {
  dimension: String::new(),
  wires: Vec::new(),
});

/// Publish the wires the level's tick pass read, for the renderer. Called from a server hook, so
/// it may run on any thread; the lock is held only for the move.
pub fn publish(dimension: &str, wires: Vec<Wire>) {
  CACHE
    .lock()
    .expect("the wire render cache lock is held for one publish")
    .set(dimension, wires);
}

/// The wires to draw this frame.
pub fn wires() -> Vec<Wire> {
  CACHE
    .lock()
    .expect("the wire render cache lock is held for one read")
    .wires
    .clone()
}

/// The centre of a block, the point a node offsets from.
pub fn centre(pos: [i32; 3]) -> [f64; 3] {
  [
    f64::from(pos[0]) + 0.5,
    f64::from(pos[1]) + 0.5,
    f64::from(pos[2]) + 0.5,
  ]
}

// --- the accumulator's gauge -------------------------------------------------
//
// The jar draws the accumulator's fill as a needle on the `gauge.png` face of every horizontal
// side (`ModularAccumulatorRenderer.renderDial`, read from the audited jar in `probe-gauge.log`):
// `rotateXDegrees(-180 * fill)` about the dial's pivot, measured from the model's `dial.json`.
// This port's block models carry no gauge partial, so the fill was invisible. The needle is drawn
// by the **same** pipeline as the wires — one more vertex buffer in the existing `world_drawn`
// mixin — with a colour taken from the jar's gauge texture so a screenshot can count it instead
// of measuring a grey panel's brightness.

/// The needle's colour: the jar's `gauge.png` cyan (123, 213, 214), as a solid so a frame can
/// count the needle rather than the grey dial it replaces.
pub const GAUGE_RGBA: [f32; 4] = [123.0 / 255.0, 213.0 / 255.0, 214.0 / 255.0, 1.0];
/// Half the needle's width: `dial.json`'s y 5.5..6.5 is 1/16 of a block tall.
pub const NEEDLE_HALF: f64 = 0.5 / 16.0;
/// The needle's length: `dial.json`'s z 8..11.25 from the pivot at z 8.
pub const NEEDLE_LENGTH: f64 = 3.25 / 16.0;

/// The jar's needle angle, `rotateXDegrees(-180 * fill)`.
pub fn needle_angle_deg(fill: f32) -> f32 {
  -180.0 * fill
}

/// The fill a gauge answers: stored over capacity, clamped to the jar's 0..1 sweep (a docked
/// charge cannot turn the needle past full).
pub fn gauge_fill(stored: i32, capacity: i32) -> f32 {
  if capacity <= 0 {
    return 0.0;
  }
  (stored.clamp(0, capacity) as f32) / (capacity as f32)
}

/// One accumulator's gauge as the renderer needs it: its block position and the tower's fill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gauge {
  pub pos: [i32; 3],
  pub fill: f32,
}

/// The gauges this client draws, and the dimension they belong to. Published by the level pass,
/// exactly as the wire set is; a different dimension replaces the set.
#[derive(Debug, Default)]
struct GaugeCache {
  dimension: String,
  gauges: Vec<Gauge>,
}

impl GaugeCache {
  fn set(&mut self, dimension: &str, gauges: Vec<Gauge>) {
    if self.dimension != dimension {
      self.dimension.clear();
      self.dimension.push_str(dimension);
    }
    self.gauges = gauges;
  }
}

static GAUGE_CACHE: Mutex<GaugeCache> = Mutex::new(GaugeCache {
  dimension: String::new(),
  gauges: Vec::new(),
});

/// Publish the gauges the level's pass read. Called from a server hook, so it may run on any
/// thread; the lock is held only for the move.
pub fn publish_gauges(dimension: &str, gauges: Vec<Gauge>) {
  GAUGE_CACHE
    .lock()
    .expect("the createaddition gauge cache lock is held for one publish")
    .set(dimension, gauges);
}

/// The gauges to draw this frame.
pub fn gauges() -> Vec<Gauge> {
  GAUGE_CACHE
    .lock()
    .expect("the createaddition gauge cache lock is held for one read")
    .gauges
    .clone()
}

// --- a wire being laid (the held-spool preview) --------------------------------
//
// Upstream's `WireNodeRenderer.render` draws a second wire when a loaded spool is held: from the
// selected connector's node to the player, in the spool's wire colour (`clientRenderHeldWire`).
// `WorldDrawn` carries the camera but no player, so the server level pass publishes the selection
// into this cache the way it publishes laid wires and gauges. `held_frame` reuses `push_wire`, so
// the preview gets the jar's catenary and the 0.2.14 lightmap for free, and the existing wire
// vertex buffer and draw call need no second `[[native.mixin]]` row.

/// A wire being laid: from the player's feet anchor to the selected connector's node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldWire {
  /// Start: the player's feet plus half a block (upstream's `stack.translate(tx + .5, …)`).
  pub from: [f64; 3],
  /// End: the selected node's world position (`te.getNodeOffset(nodeIndex)`).
  pub to: [f64; 3],
  /// The wire kind of the spool that made the selection (`WireType` from its custom data).
  pub kind: WireKind,
  /// Block/sky light at the player's block (0) and the selected connector (1).
  pub light: Light,
}

#[derive(Debug, Default)]
struct HeldCache {
  dimension: String,
  held: Vec<HeldWire>,
}

impl HeldCache {
  fn set(&mut self, dimension: &str, held: Vec<HeldWire>) {
    if self.dimension != dimension {
      self.dimension.clear();
      self.dimension.push_str(dimension);
    }
    self.held = held;
  }
}

static HELD_CACHE: Mutex<HeldCache> = Mutex::new(HeldCache {
  dimension: String::new(),
  held: Vec::new(),
});

/// Publish the wire in progress the level's pass saw. Called from a server hook, so it may run on
/// any thread; the lock is held only for the move.
pub fn publish_held(dimension: &str, held: Vec<HeldWire>) {
  HELD_CACHE
    .lock()
    .expect("the createaddition held-wire cache lock is held for one publish")
    .set(dimension, held);
}

/// The wire being laid this frame, if any.
pub fn held() -> Vec<HeldWire> {
  HELD_CACHE
    .lock()
    .expect("the createaddition held-wire cache lock is held for one read")
    .held
    .clone()
}

/// The vertices of the held spool's preview: the same `push_wire` as a laid wire, so the catenary,
/// the 0.7 alternating shade and the 0.2.14 lightmap are the jar's, and the festive wire keeps its
/// string of lights. The caller appends these to the laid wires' own vertex buffer and draw call.
pub fn held_frame(held: &[HeldWire], camera: [f64; 3], view_proj: &[f32; 16]) -> Vec<Vertex> {
  let mut vertices = Vec::new();
  for preview in held {
    push_wire(
      &mut vertices,
      &Wire {
        from: preview.from,
        to: preview.to,
        kind: preview.kind,
        light: preview.light,
      },
      camera,
      view_proj,
    );
  }
  vertices
}

/// The world position of a gauge needle's pivot on its east face (the jar's base direction,
/// rotated to the other three horizontal faces by the renderer).
pub fn needle_pivot(pos: [i32; 3]) -> [f64; 3] {
  let middle = centre(pos);
  // dial.json's `(16, 6, 8)` in sixteenths, plus the renderer's `width/2 - 0.375` face offset,
  // and `height - 0.5`: `(1.125, 0.375, 0.5)` relative to the block's lower north-west corner.
  [middle[0] + 0.625, middle[1] - 0.125, middle[2]]
}

/// The world position of the needle's tip on its east face at `fill`, before the four horizontal
/// rotations. The jar turns it `-180 * fill` degrees about the pivot, so empty points one way
/// along the face and full the opposite way — the flip a screenshot can measure.
pub fn needle_tip(pos: [i32; 3], fill: f32) -> [f64; 3] {
  let pivot = needle_pivot(pos);
  let along = rotate_x([0.0, 0.0, NEEDLE_LENGTH], needle_angle_deg(fill) as f64);
  add_f64(pivot, along)
}

/// A right-handed rotation about the block's vertical axis, in degrees.
fn rotate_y(point: [f64; 3], degrees: f64) -> [f64; 3] {
  let angle = degrees.to_radians();
  let (sin, cos) = angle.sin_cos();
  [
    point[0] * cos + point[2] * sin,
    point[1],
    -point[0] * sin + point[2] * cos,
  ]
}

/// A rotation about a horizontal face's normal (the model's x axis), in degrees — the jar's
/// `rotateXDegrees`.
fn rotate_x(point: [f64; 3], degrees: f64) -> [f64; 3] {
  let angle = degrees.to_radians();
  let (sin, cos) = angle.sin_cos();
  [
    point[0],
    point[1] * cos - point[2] * sin,
    point[1] * sin + point[2] * cos,
  ]
}

/// The needle quads one frame draws: four horizontal faces per gauge, rotated from the jar's
/// pivot by `-180 * fill`. `camera` and `view_proj` are the frame's, as for [`frame`].
pub fn gauge_frame(gauges: &[Gauge], camera: [f64; 3], view_proj: &[f32; 16]) -> Vec<Vertex> {
  let mut vertices = Vec::with_capacity(gauges.len() * 4 * 6);
  for gauge in gauges {
    let middle = centre(gauge.pos);
    let angle = needle_angle_deg(gauge.fill) as f64;
    let pivot = needle_pivot(gauge.pos);
    let tip = needle_tip(gauge.pos, gauge.fill);
    let across = rotate_y(rotate_x([0.0, NEEDLE_HALF, 0.0], angle), 0.0);
    for face in [0.0, 90.0, 180.0, 270.0] {
      // The jar draws a gauge on all four horizontal faces and the block hides the two on the far
      // side. This pass has no block model to occlude them, so skip a face whose normal points
      // away from the camera: drawing both a near and a far needle would cancel the fill's flip.
      let toward = [
        camera[0] - middle[0],
        camera[1] - middle[1],
        camera[2] - middle[2],
      ];
      let normal = rotate_y([1.0, 0.0, 0.0], face);
      if toward[0] * normal[0] + toward[1] * normal[1] + toward[2] * normal[2] <= 1e-9 {
        continue;
      }
      let at = add_f64(middle, rotate_y(sub_f64(pivot, middle), face));
      let along = rotate_y(sub_f64(tip, pivot), face);
      let side = rotate_y(across, face);
      let corners = [
        sub_f64(at, side),
        add_f64(at, side),
        add_f64(add_f64(at, along), side),
        add_f64(sub_f64(at, along), side),
      ];
      let clips = corners.map(|corner| {
        clip(
          [
            (corner[0] - camera[0]) as f32,
            (corner[1] - camera[1]) as f32,
            (corner[2] - camera[2]) as f32,
          ],
          view_proj,
        )
      });
      // A needle behind the camera is dropped whole, as a wire's quad is.
      if clips.iter().any(|c| c[3] <= 0.001) {
        continue;
      }
      for (index, colour) in [
        (0, GAUGE_RGBA),
        (1, GAUGE_RGBA),
        (2, GAUGE_RGBA),
        (0, GAUGE_RGBA),
        (2, GAUGE_RGBA),
        (3, GAUGE_RGBA),
      ] {
        vertices.push(Vertex {
          clip: clips[index],
          rgba: colour,
        });
      }
    }
  }
  vertices
}

fn add_f64(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
  [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub_f64(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
  [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// A connector's node: `3/16` of a block out of the face it is mounted on
/// (`SmallConnectorBlockEntity.OFFSET_HEIGHT`, which the large one and the lamp share).
///
/// `facing` is the side of the block the connector is mounted against (the port's `facing`
/// property), so the node is on that side's face.
pub fn attach_connector(centre: [f64; 3], facing: [i32; 3]) -> [f64; 3] {
  const OUT: f64 = 3.0 / 16.0;
  [
    centre[0] + f64::from(facing[0]) * OUT,
    centre[1] + f64::from(facing[1]) * OUT,
    centre[2] + f64::from(facing[2]) * OUT,
  ]
}

/// A relay's node, as `RedstoneRelayBlockEntity.getNodeOffset` places its eight: `5/16` along
/// the relay's horizontal facing, `1/16` across it, and `1/16` below the block's centre (or the
/// vertical variant, which lies flat).
///
/// `facing` is the relay's `facing` property's normal, `vertical` its `vertical` bit, and
/// `input` which of its two ports (0–3 in, 4–7 out) this node belongs to.
pub fn attach_relay(centre: [f64; 3], facing: [i32; 3], vertical: bool, input: bool) -> [f64; 3] {
  let [x, _, z] = [i32_to_f64(facing[0]), 0.0, i32_to_f64(facing[2])];
  let (dx, dy, dz) = if vertical {
    // The jar's four vertical pairs, as `getNodeOffset` reads them:
    // north (out -5,-1 / in 5,-1), west (-1,5 / -1,-5), south (5,1 / -5,1), east (1,-5 / 1,5).
    let out = [(z * 5.0 + x) / 16.0, 0.0, (z - x * 5.0) / 16.0];
    let into = [(-z * 5.0 + x) / 16.0, 0.0, (z + x * 5.0) / 16.0];
    let pick = if input { into } else { out };
    (pick[0], pick[1], pick[2])
  } else {
    let side = if input { -1.0 } else { 1.0 };
    (x * 5.0 / 16.0 * side, -1.0 / 16.0, z * 5.0 / 16.0 * side)
  };
  [centre[0] + dx, centre[1] + dy, centre[2] + dz]
}

fn i32_to_f64(value: i32) -> f64 {
  f64::from(value)
}

/// The jar's sag at `part` of the way along a wire `distance` blocks long
/// (`WireNodeRenderer.hang`).
pub fn hang(part: f32, distance: f32) -> f32 {
  (-part * std::f32::consts::PI).sin() * (HANG * distance / MAX_LENGTH)
}

/// The point `part` of the way along `delta` from its start, sagging as the jar's does: the
/// catenary term grows from the lower of the two ends, as `wireVert`'s `catenaryScalar` has it.
fn along(delta: [f32; 3], part: f32, distance: f32) -> [f32; 3] {
  let up = delta[1];
  let catenary = if up > 0.0 {
    up * part * part
  } else {
    up - up * (1.0 - part) * (1.0 - part)
  };
  [
    delta[0] * part,
    catenary + hang(part, distance),
    delta[2] * part,
  ]
}

/// The vertices one frame draws: every wire of `wires` as a tube of four sides, plus a festive
/// wire's lights. `camera` is the frame camera's world position (the matrix is
/// camera-relative) and `view_proj` the frame's matrix as `Mat4::to_cols_array` gives it.
pub fn frame(wires: &[Wire], camera: [f64; 3], view_proj: &[f32; 16]) -> Vec<Vertex> {
  let mut vertices = Vec::new();
  for wire in wires {
    push_wire(&mut vertices, wire, camera, view_proj);
  }
  vertices
}

fn push_wire(out: &mut Vec<Vertex>, wire: &Wire, camera: [f64; 3], m: &[f32; 16]) {
  let start = [
    (wire.from[0] - camera[0]) as f32,
    (wire.from[1] - camera[1]) as f32,
    (wire.from[2] - camera[2]) as f32,
  ];
  let delta = [
    (wire.to[0] - wire.from[0]) as f32,
    (wire.to[1] - wire.from[1]) as f32,
    (wire.to[2] - wire.from[2]) as f32,
  ];
  let distance = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
  if !(0.01..=FAR).contains(&distance) {
    return;
  }
  // A tube's cross-section: across the wire horizontally, and up. A vertical wire has no
  // horizontal direction, so its cross-section is taken in the x/z plane instead (the jar
  // branches on the same condition).
  let horizontal = (delta[0] * delta[0] + delta[2] * delta[2]).sqrt();
  let side = if horizontal > 1e-4 {
    let scale = HALF / horizontal;
    [-delta[2] * scale, 0.0, delta[0] * scale]
  } else {
    [HALF, 0.0, 0.0]
  };
  let up = [0.0, HALF, 0.0];
  let outline = [side, up, neg(side), neg(up)];
  let base = colour(wire.kind.rgb());
  let mut previous: Option<([f32; 3], [f32; 4])> = None;
  for step in 0..=SEGMENTS {
    let part = step as f32 / SEGMENTS as f32;
    let point = add(start, along(delta, part, distance));
    let shade = if step % 2 == 0 { 0.7 } else { 1.0 };
    // The jar's `wirePart` reads the interpolated lightmap coordinate at every piece and gives it
    // to `wireVert`/`setLight`. Here the same per-piece light scales the vertex colour: a wire in
    // the dark keeps the jar's hue but loses brightness.
    let lit = segment_light(wire.light, part);
    let rgba = [
      base[0] * shade * lit,
      base[1] * shade * lit,
      base[2] * shade * lit,
      1.0,
    ];
    if let Some((last, last_rgba)) = previous {
      for index in 0..4 {
        let a = add(last, outline[index]);
        let b = add(last, outline[(index + 1) % 4]);
        let c = add(point, outline[(index + 1) % 4]);
        let d = add(point, outline[index]);
        push_quad(out, m, [a, b, c, d], last_rgba, rgba);
      }
    }
    previous = Some((point, rgba));
    if wire.kind.festive() {
      push_light(out, m, start, delta, distance, step, lit);
    }
  }
}

/// A festive wire's light at `step`, every third point as `WireNodeRenderer.wirePart` draws them
/// (`index % 3 == 0`, the last point excepted), in the jar's red/green/blue cycle.
fn push_light(
  out: &mut Vec<Vertex>,
  m: &[f32; 16],
  start: [f32; 3],
  delta: [f32; 3],
  distance: f32,
  step: usize,
  lit: f32,
) {
  const COLOURS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
  if step == 0 || step == SEGMENTS || !step.is_multiple_of(3) {
    return;
  }
  let main = delta[0] + delta[1] + delta[2] > 0.0;
  let cycle = (step / 3) % 3;
  let colour = COLOURS[if main { 2 - cycle } else { cycle }];
  // The jar's festive `lights()` path is handed the same endpoint light values (`javap` 1059–1061),
  // so a light string dims with the wire it hangs on.
  let rgba = [colour[0] * lit, colour[1] * lit, colour[2] * lit, 1.0];
  let part = step as f32 / SEGMENTS as f32;
  let mut point = add(start, along(delta, part, distance));
  point[1] += LIGHT_Y_OFFSET;
  // Two crossed quads: a dot that shows from either side of the wire, as a small partial does.
  let planes = [
    ([LIGHT, 0.0, 0.0], [0.0, LIGHT, 0.0]),
    ([0.0, 0.0, LIGHT], [0.0, LIGHT, 0.0]),
  ];
  for (first, second) in planes {
    let corners = [
      add(point, add(first, second)),
      add(point, sub(first, second)),
      add(point, neg(add(first, second))),
      add(point, sub(second, first)),
    ];
    push_quad(out, m, corners, rgba, rgba);
  }
}

fn push_quad(
  out: &mut Vec<Vertex>,
  m: &[f32; 16],
  corners: [[f32; 3]; 4],
  a: [f32; 4],
  b: [f32; 4],
) {
  let clips = [
    clip(corners[0], m),
    clip(corners[1], m),
    clip(corners[2], m),
    clip(corners[3], m),
  ];
  // Behind the camera: a quad is dropped whole rather than clipped, as the wire's own span is
  // never split (the camera stands beside the rig, not inside it).
  if clips.iter().any(|c| c[3] <= 0.001) {
    return;
  }
  for (index, colour) in [(0, a), (1, a), (2, b), (0, a), (2, b), (3, b)] {
    out.push(Vertex {
      clip: clips[index],
      rgba: colour,
    });
  }
}

fn clip(point: [f32; 3], m: &[f32; 16]) -> [f32; 4] {
  // `Mat4` is column-major: column `j` holds the coefficients of input `j`.
  let mut out = [0.0f32; 4];
  for (row, slot) in out.iter_mut().enumerate() {
    *slot = m[row] * point[0] + m[4 + row] * point[1] + m[8 + row] * point[2] + m[12 + row];
  }
  out
}

/// The bytes the helper uploads for `vertices`: 32 bytes each, little-endian, position then
/// colour — the layout the helper's vertex buffer declares.
pub fn vertex_bytes(vertices: &[Vertex]) -> Vec<u8> {
  let mut bytes = Vec::with_capacity(vertices.len() * 32);
  for vertex in vertices {
    for value in vertex.clip.into_iter().chain(vertex.rgba) {
      bytes.extend_from_slice(&value.to_le_bytes());
    }
  }
  bytes
}

/// A light level's denominator scalar: the jar's `getBrightness` uses `f = level / 15`.
const LIGHT_LEVELS: f32 = 15.0;

/// The jar's `Mth.lerp(part, from, to)` narrowed to an `int` by `f2i` — exactly what
/// `WireNodeRenderer.wirePart` does for both light layers at each of its 24 pieces.
pub fn light_lerp(part: f32, from: i32, to: i32) -> i32 {
  (from as f32 + part * (to as f32 - from as f32)) as i32
}

/// The jar's `LightTexture.pack(block, sky)`: block light shifted 4, sky light shifted 20
/// (the 1.21.1 client's `LightTexture.pack`, read from its bytecode as `a(int, int)`).
pub fn light_pack(block: i32, sky: i32) -> u32 {
  ((block as u32) << 4) | ((sky as u32) << 20)
}

/// Vanilla's `LightTexture.getBrightness(dimensionType, level)` for a dimension with ambient
/// light 0 (the End, where the wire rig lives): `f = level / 15`, `f / (4 - 3f)`, lerped to 1.
/// This is the curve the lightmap texture samples; it is not a screenshot-guessed dim factor.
pub fn light_brightness(level: i32) -> f32 {
  let f = (level.clamp(0, 15) as f32) / LIGHT_LEVELS;
  f / (4.0 - 3.0 * f)
}

/// The brightness of one lightmap texel, from the packed block/sky light: block and sky
/// contributions added as `LightTexture` fills the texture, clamped, then the texture's gamma
/// (`notGamma = 1 - (1-v)^3`) and its 0.04 lerp to the 0.75 grey floor. A stride toward the
/// jar's own lightmap, expressed as a scalar the wire shader can multiply into its colour.
pub fn lightmap_factor(packed: u32) -> f32 {
  let block = ((packed >> 4) & 0xffff) as i32;
  let sky = ((packed >> 20) & 0xffff) as i32;
  // `LightTexture` fills the texel with the block-light colour plus the sky-light colour, then
  // clamps it to one. Both curves come from `getBrightness` above. No-sky dimensions (End/Nether)
  // have sky factor 0, which the level pass expresses by publishing a zero sky level.
  let combined = (light_brightness(block) + light_brightness(sky)).min(1.0);
  // The texture's `notGamma` (`1 - (1-v)^3`), then its 0.04 lerp toward the 0.75 grey floor: even
  // at zero light a texel keeps a faint value, which is why a dark wire is dim rather than absent.
  let gamma = 1.0 - (1.0 - combined).powi(3);
  (gamma * 0.96 + 0.03).min(1.0)
}

/// The dimming a wire vertex at `part` of its span gets: interpolate each light layer exactly as
/// the jar does, pack it as the jar does, then read its lightmap brightness.
pub fn segment_light(light: Light, part: f32) -> f32 {
  let block = light_lerp(part, light.block[0], light.block[1]);
  let sky = light_lerp(part, light.sky[0], light.sky[1]);
  lightmap_factor(light_pack(block, sky))
}

fn colour(rgb: [u8; 3]) -> [f32; 3] {
  [
    f32::from(rgb[0]) / 255.0,
    f32::from(rgb[1]) / 255.0,
    f32::from(rgb[2]) / 255.0,
  ]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
  [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn neg(a: [f32; 3]) -> [f32; 3] {
  [-a[0], -a[1], -a[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
  [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A camera looking down `-z` from the origin (OpenGL's convention): `w` is `-z`, so a point
  /// at the origin has `w = 0` and everything in front of the camera has `w > 0`.
  fn looking_forward() -> [f32; 16] {
    [
      1.0, 0.0, 0.0, 0.0, //
      0.0, 1.0, 0.0, 0.0, //
      0.0, 0.0, 1.0, -1.0, //
      0.0, 0.0, 0.0, 0.0,
    ]
  }

  /// A straight projection: clip coordinates are the camera-relative ones, with `w = 1`.
  fn straight() -> [f32; 16] {
    [
      1.0, 0.0, 0.0, 0.0, //
      0.0, 1.0, 0.0, 0.0, //
      0.0, 0.0, 1.0, 0.0, //
      0.0, 0.0, 0.0, 1.0,
    ]
  }

  fn wire(from: [f64; 3], to: [f64; 3], kind: WireKind) -> Wire {
    Wire {
      from,
      to,
      kind,
      light: Light::BRIGHT,
    }
  }

  #[test]
  fn every_wire_kind_draws_in_its_own_wire_type_colour() {
    // `WireType.java`: COPPER(…, 78, 37, 30), GOLD(…, 98, 83, 29),
    // ELECTRUM(…, 88, 66, 37), FESTIVE(…, 26, 94, 12).
    assert_eq!(WireKind::Copper.rgb(), [78, 37, 30]);
    assert_eq!(WireKind::Gold.rgb(), [98, 83, 29]);
    assert_eq!(WireKind::Electrum.rgb(), [88, 66, 37]);
    assert_eq!(WireKind::Festive.rgb(), [26, 94, 12]);
    let vertices = frame(
      &[wire([0.0, 0.0, 0.0], [4.0, 0.0, 0.0], WireKind::Copper)],
      [0.0; 3],
      &straight(),
    );
    let base = colour([78, 37, 30]);
    let lit = lightmap_factor(light_pack(15, 15));
    for vertex in &vertices {
      // The wire keeps the jar's hue; the only variation is the 0.7 alternating shade and the
      // (uniform) lightmap factor a fully lit wire carries.
      let shade = vertex.rgba[0] / (base[0] * lit);
      assert!(
        (shade - 1.0).abs() < 1e-5 || (shade - 0.7).abs() < 1e-5,
        "a copper wire is drawn in copper's colour, shaded only by the jar's 0.7: {vertex:?}"
      );
      assert!((vertex.rgba[1] / (base[1] * lit) - shade).abs() < 1e-5);
      assert!((vertex.rgba[2] / (base[2] * lit) - shade).abs() < 1e-5);
      assert_eq!(vertex.rgba[3], 1.0);
    }
  }

  #[test]
  fn a_wire_hangs_below_the_straight_line_by_the_jars_catenary() {
    // A horizontal four-block wire: the midpoint's midline is the jar's `hang(0.5, 4)` =
    // sin(-PI/2) * (0.5 * 4 / 16) = -0.0625, below the straight line (y = 0).
    let vertices = frame(
      &[wire([0.0, 0.0, 0.0], [4.0, 0.0, 0.0], WireKind::Gold)],
      [0.0; 3],
      &straight(),
    );
    let lowest = vertices
      .iter()
      .map(|v| v.clip[1])
      .fold(f32::INFINITY, f32::min);
    // The lowest vertex of the mid ring is the midline minus the cross-section's half.
    let midline = lowest + HALF;
    let jar = hang(0.5, 4.0);
    assert!(
      (midline - jar).abs() < 0.01,
      "the wire's midline sags to {midline}, the jar's catenary to {jar}"
    );
    assert!(
      midline < -0.05,
      "a four-block wire sags by about 0.06, not {midline}"
    );
  }

  #[test]
  fn a_wire_starts_and_ends_at_the_points_it_links() {
    let from = [1.0, 2.0, 3.0];
    let to = [5.0, 2.0, 3.0];
    let vertices = frame(&[wire(from, to, WireKind::Copper)], [0.0; 3], &straight());
    let first = vertices.first().expect("a wire draws");
    let last = vertices.last().expect("a wire draws");
    // The first ring is the `from` point plus a cross-section offset, the last the `to` point.
    assert!((first.clip[0] - (from[0] as f32)).abs() < 0.02);
    assert!((first.clip[2] - (from[2] as f32)).abs() < 0.02);
    assert!((last.clip[0] - (to[0] as f32)).abs() < 0.02);
  }

  #[test]
  fn a_vertical_wire_is_drawn_without_a_division_by_zero() {
    // `WireNodeRenderer.wireRender` divides by `fastInvSqrt(x*x + z*z)`; a wire straight up has
    // none, and the jar branches on `|x| + |z| < |y|`. Every coordinate must stay finite.
    let vertices = frame(
      &[wire([0.0, 0.0, 0.0], [0.0, 4.0, 0.0], WireKind::Electrum)],
      [0.0; 3],
      &straight(),
    );
    assert!(!vertices.is_empty(), "a vertical wire draws");
    for vertex in &vertices {
      assert!(vertex.clip.iter().all(|c| c.is_finite()), "{vertex:?}");
    }
  }

  #[test]
  fn a_festive_wire_carries_its_string_of_lights_and_the_others_do_not() {
    let festive = frame(
      &[wire([0.0, 0.0, 0.0], [12.0, 0.0, 0.0], WireKind::Festive)],
      [0.0; 3],
      &straight(),
    );
    let copper = frame(
      &[wire([0.0, 0.0, 0.0], [12.0, 0.0, 0.0], WireKind::Copper)],
      [0.0; 3],
      &straight(),
    );
    assert!(
      festive.len() > copper.len() + 60,
      "a festive wire draws its lights ({} vertices against copper's {})",
      festive.len(),
      copper.len()
    );
    // The lights are the jar's red, green and blue, at the fully lit wire's brightness.
    let lit = lightmap_factor(light_pack(15, 15));
    for wanted in [
      [lit, 0.0, 0.0, 1.0],
      [0.0, lit, 0.0, 1.0],
      [0.0, 0.0, lit, 1.0],
    ] {
      assert!(
        festive.iter().any(|v| v.rgba == wanted),
        "the string of lights has a {wanted:?} one"
      );
      assert!(
        !copper.iter().any(|v| v.rgba == wanted),
        "a copper wire carries no lights"
      );
    }
  }

  #[test]
  fn a_connectors_node_is_three_sixteenths_out_of_its_mounting_face() {
    // `SmallConnectorBlockEntity.OFFSET_HEIGHT = 3` and its four horizontal offsets.
    let centre = centre([10, 64, 0]);
    assert_eq!(
      attach_connector(centre, [0, 0, -1]),
      [10.5, 64.5, -3.0 / 16.0 + 0.5]
    );
    assert_eq!(
      attach_connector(centre, [0, 0, 1]),
      [10.5, 64.5, 3.0 / 16.0 + 0.5]
    );
    assert_eq!(
      attach_connector(centre, [-1, 0, 0]),
      [10.5 - 3.0 / 16.0, 64.5, 0.5]
    );
    assert_eq!(
      attach_connector(centre, [1, 0, 0]),
      [10.5 + 3.0 / 16.0, 64.5, 0.5]
    );
    assert_eq!(
      attach_connector(centre, [0, -1, 0]),
      [10.5, 64.5 - 3.0 / 16.0, 0.5]
    );
    assert_eq!(
      attach_connector(centre, [0, 1, 0]),
      [10.5, 64.5 + 3.0 / 16.0, 0.5]
    );
  }

  #[test]
  fn a_relays_eight_nodes_are_where_redstone_relay_block_entity_puts_them() {
    // The jar's table, verbatim: `OFFSET_*` (horizontal), `IN_VERTICAL_OFFSET_*`,
    // `OUT_VERTICAL_OFFSET_*`, all in sixteenths.
    let centre = [0.0, 0.0, 0.0];
    type Case = ([i32; 3], bool, [f64; 3], [f64; 3]);
    let cases: [Case; 4] = [
      // facing, vertical, input, output
      ([0, 0, -1], false, [0.0, -1.0, 5.0], [0.0, -1.0, -5.0]),
      ([1, 0, 0], false, [-5.0, -1.0, 0.0], [5.0, -1.0, 0.0]),
      ([0, 0, 1], false, [0.0, -1.0, -5.0], [0.0, -1.0, 5.0]),
      ([-1, 0, 0], false, [5.0, -1.0, 0.0], [-5.0, -1.0, 0.0]),
    ];
    for (facing, vertical, input, output) in cases {
      let got_in = attach_relay(centre, facing, vertical, true);
      let got_out = attach_relay(centre, facing, vertical, false);
      assert_eq!(
        got_in,
        [input[0] / 16.0, input[1] / 16.0, input[2] / 16.0],
        "{facing:?} in"
      );
      assert_eq!(
        got_out,
        [output[0] / 16.0, output[1] / 16.0, output[2] / 16.0],
        "{facing:?} out"
      );
    }
    let verticals: [([i32; 3], [f64; 3], [f64; 3]); 4] = [
      ([0, 0, -1], [5.0, 0.0, -1.0], [-5.0, 0.0, -1.0]),
      ([-1, 0, 0], [-1.0, 0.0, -5.0], [-1.0, 0.0, 5.0]),
      ([0, 0, 1], [-5.0, 0.0, 1.0], [5.0, 0.0, 1.0]),
      ([1, 0, 0], [1.0, 0.0, 5.0], [1.0, 0.0, -5.0]),
    ];
    for (facing, input, output) in verticals {
      let got_in = attach_relay(centre, facing, true, true);
      let got_out = attach_relay(centre, facing, true, false);
      assert_eq!(
        got_in,
        [input[0] / 16.0, input[1] / 16.0, input[2] / 16.0],
        "vertical {facing:?} in"
      );
      assert_eq!(
        got_out,
        [output[0] / 16.0, output[1] / 16.0, output[2] / 16.0],
        "vertical {facing:?} out"
      );
    }
  }

  #[test]
  fn a_wire_leaves_the_client_cache_when_the_level_it_belongs_to_changes() {
    let mut cache = Cache::default();
    let end = wire([0.0; 3], [1.0, 0.0, 0.0], WireKind::Copper);
    cache.set("minecraft:overworld", vec![end]);
    assert_eq!(cache.wires.len(), 1);
    // The same level replaces its set (a wire taken out does not linger).
    cache.set("minecraft:overworld", vec![]);
    assert!(cache.wires.is_empty());
    assert_eq!(cache.dimension, "minecraft:overworld");
    // Another level starts its own set, so the left dimension's wires are not drawn in it.
    let other = wire([0.0; 3], [0.0, 0.0, 1.0], WireKind::Gold);
    cache.set("minecraft:the_end", vec![other]);
    assert_eq!(cache.wires, vec![other]);
    assert_eq!(cache.dimension, "minecraft:the_end");
  }

  #[test]
  fn the_uploaded_bytes_are_the_layout_the_pipeline_declares() {
    let vertices = vec![Vertex {
      clip: [1.0, 2.0, 3.0, 4.0],
      rgba: [0.5, 0.25, 0.125, 1.0],
    }];
    let bytes = vertex_bytes(&vertices);
    assert_eq!(bytes.len(), 32, "32 bytes a vertex: two vec4s");
    assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
    assert_eq!(&bytes[12..16], &4.0f32.to_le_bytes());
    assert_eq!(&bytes[16..20], &0.5f32.to_le_bytes());
    assert_eq!(&bytes[28..32], &1.0f32.to_le_bytes());
  }

  #[test]
  fn a_wire_behind_the_camera_is_not_drawn() {
    let in_front = frame(
      &[wire([0.0, 0.0, -12.0], [4.0, 0.0, -12.0], WireKind::Copper)],
      [0.0; 3],
      &looking_forward(),
    );
    assert!(!in_front.is_empty(), "a wire in front of the camera draws");
    let behind = frame(
      &[wire([0.0, 0.0, 12.0], [4.0, 0.0, 12.0], WireKind::Copper)],
      [0.0; 3],
      &looking_forward(),
    );
    assert!(
      behind.is_empty(),
      "a quad behind the camera is dropped whole: {}",
      behind.len()
    );
  }

  #[test]
  fn the_needle_angle_is_the_jars_minus_one_eighty_times_the_fill() {
    // `ModularAccumulatorRenderer.renderDial`: `rotateXDegrees(-180.0F * gauge.getValue(partial))`.
    assert!((needle_angle_deg(0.0) - 0.0).abs() < 1e-5);
    assert!(
      (needle_angle_deg(0.5) + 90.0).abs() < 1e-5,
      "half full turns the needle ninety degrees: {}",
      needle_angle_deg(0.5)
    );
    assert!(
      (needle_angle_deg(1.0) + 180.0).abs() < 1e-5,
      "full turns it the whole half circle: {}",
      needle_angle_deg(1.0)
    );
  }

  #[test]
  fn a_gauges_fill_is_stored_over_capacity_and_clamped() {
    assert_eq!(gauge_fill(0, 2_000_000), 0.0);
    assert_eq!(gauge_fill(1_000_000, 2_000_000), 0.5);
    assert_eq!(gauge_fill(2_000_000, 2_000_000), 1.0);
    assert_eq!(
      gauge_fill(3_000_000, 2_000_000),
      1.0,
      "a docked charge is still full"
    );
    assert_eq!(
      gauge_fill(7, 0),
      0.0,
      "a zero-capacity gauge does not divide by zero"
    );
  }

  #[test]
  fn a_gauge_needle_points_the_opposite_way_when_full() {
    // On the jar's base (east) face the needle reaches z 8..11.25 from its pivot at z 8, so an
    // empty gauge's tip is 3.25/16 of a block to one side of the pivot and a full one's is that
    // far to the other — the flip the screenshot leg counts. (Averaging all four faces would
    // cancel, since the jar draws one on each side.)
    let pivot = needle_pivot([0, 0, 0]);
    let empty = needle_tip([0, 0, 0], 0.0);
    let full = needle_tip([0, 0, 0], 1.0);
    assert!(
      (empty[2] - pivot[2] - NEEDLE_LENGTH).abs() < 1e-9,
      "empty points along +z: {empty:?} from {pivot:?}"
    );
    assert!(
      (pivot[2] - full[2] - NEEDLE_LENGTH).abs() < 1e-9,
      "full points along -z: {full:?} from {pivot:?}"
    );
    assert!(empty[2] - full[2] > 2.0 * NEEDLE_LENGTH - 1e-9);
    // And the frame really draws the cyan needle on all four faces, not a grey panel.
    let frame = gauge_frame(
      &[
        Gauge {
          pos: [0, 0, 0],
          fill: 0.0,
        },
        Gauge {
          pos: [0, 0, 0],
          fill: 1.0,
        },
      ],
      [4.5, 1.5, 0.5],
      &straight(),
    );
    assert_eq!(
      frame.len(),
      12,
      "two needles, one face toward this camera, six vertices a quad"
    );
    for vertex in &frame {
      assert_eq!(
        vertex.rgba, GAUGE_RGBA,
        "the needle is the gauge's cyan, not a grey panel"
      );
    }
  }

  #[test]
  fn a_gauge_draws_only_the_faces_that_look_at_the_camera() {
    let middle = centre([4, 64, 9]);
    let camera = [middle[0] + 4.0, middle[1] + 1.0, middle[2] + 1.0];
    let vertices = gauge_frame(
      &[Gauge {
        pos: [4, 64, 9],
        fill: 0.25,
      }],
      camera,
      &straight(),
    );
    assert_eq!(
      vertices.len(),
      2 * 6,
      "the east and south faces look at this camera; the far two are hidden by the block: {}",
      vertices.len()
    );
  }

  /// The mean brightness of a draw's vertices — what a screenshot would measure on the wire's
  /// own coloured pixels, before the frame's tone mapping.
  fn mean_luma(vertices: &[Vertex]) -> f32 {
    vertices
      .iter()
      .map(|v| (v.rgba[0] + v.rgba[1] + v.rgba[2]) / 3.0)
      .sum::<f32>()
      / vertices.len() as f32
  }

  #[test]
  fn the_jars_lightmap_packs_and_interpolates_block_and_sky_light() {
    // `WireNodeRenderer.wirePart`: `Mth.lerp(part, start, end)` narrowed by `f2i`, then
    // `LightTexture.pack(block, sky) = (block << 4) | (sky << 20)`.
    assert_eq!(light_lerp(0.0, 0, 15), 0);
    assert_eq!(light_lerp(1.0, 0, 15), 15);
    assert_eq!(
      light_lerp(0.5, 0, 15),
      7,
      "Mth.lerp(0.5, 0, 15) is 7.5, floored"
    );
    assert_eq!(light_pack(7, 3), (7 << 4) | (3 << 20));
  }

  #[test]
  fn the_lightmap_brightness_follows_the_jars_get_brightness_curve() {
    // `LightTexture.getBrightness`: f = level/15, f/(4-3f), lerped from ambient 0 to 1.
    assert_eq!(light_brightness(0), 0.0);
    assert_eq!(light_brightness(15), 1.0);
    let mid = light_brightness(7);
    assert!(
      (mid - (7.0 / 15.0) / (4.0 - 3.0 * 7.0 / 15.0)).abs() < 1e-6,
      "light 7 must read the jar's curve, got {mid}"
    );
    assert!(
      (0..15).all(|l| light_brightness(l) < light_brightness(l + 1)),
      "the curve is monotonic in the light level"
    );
  }

  #[test]
  fn a_wire_in_the_dark_is_dimmer_than_one_in_daylight() {
    // The gap line: "a wire in a dark room is as bright as one in daylight". A carried lightmap
    // must make the dark frame measurably dimmer while keeping the wire's own colour.
    let lit = frame(
      &[wire([0.0, 0.0, 0.0], [4.0, 0.0, 0.0], WireKind::Festive)],
      [0.0; 3],
      &straight(),
    );
    let mut dark_wire = wire([0.0, 0.0, 0.0], [4.0, 0.0, 0.0], WireKind::Festive);
    dark_wire.light = Light {
      block: [0, 0],
      sky: [0, 0],
    };
    let dark = frame(&[dark_wire], [0.0; 3], &straight());
    let lit_mean = mean_luma(&lit);
    let dark_mean = mean_luma(&dark);
    assert!(
      dark_mean < lit_mean * 0.5,
      "a wire in the dark must be much dimmer: dark mean {dark_mean}, lit mean {lit_mean}"
    );
    assert!(
      (0.001..=0.5).contains(&dark_mean),
      "a lightless wire keeps the lightmap's faint grey floor, not zero and not full: {dark_mean}"
    );
    assert!(
      lit_mean > 0.1,
      "a fully lit wire keeps the jar's own colour brightness: {lit_mean}"
    );
  }

  #[test]
  fn a_gauge_cache_replaces_the_previous_levels_gauges() {
    let mut cache = GaugeCache::default();
    cache.set(
      "minecraft:overworld",
      vec![Gauge {
        pos: [0, 0, 0],
        fill: 0.5,
      }],
    );
    assert_eq!(cache.gauges.len(), 1);
    cache.set("minecraft:overworld", vec![]);
    assert!(cache.gauges.is_empty());
    cache.set(
      "minecraft:the_end",
      vec![Gauge {
        pos: [1, 2, 3],
        fill: 1.0,
      }],
    );
    assert_eq!(
      cache.gauges,
      vec![Gauge {
        pos: [1, 2, 3],
        fill: 1.0
      }]
    );
    assert_eq!(cache.dimension, "minecraft:the_end");
  }

  /// The held-spool preview: a selection must draw a wire to the player's hand, and no selection
  /// must draw nothing. These were written against the stubbed `held_frame` (which returned no
  /// vertices) and watched fail before the geometry was implemented.
  fn held_rig() -> HeldWire {
    HeldWire {
      from: [10.0, 1.5, 3.0],
      to: [8.0, 1.5, 0.0],
      kind: WireKind::Festive,
      light: Light {
        block: [15, 15],
        sky: [0, 0],
      },
    }
  }

  #[test]
  fn a_pending_selection_draws_a_wire_to_the_hand() {
    let vertices = held_frame(&[held_rig()], [10.0, 1.5, 4.0], &straight());
    assert!(
      !vertices.is_empty(),
      "a held spool's selection must draw a preview wire to the player"
    );
  }

  #[test]
  fn no_pending_selection_draws_no_preview() {
    assert!(
      held_frame(&[], [10.0, 1.5, 4.0], &straight()).is_empty(),
      "with no selection there is no held wire to draw"
    );
  }

  #[test]
  fn the_preview_keeps_the_wires_own_green_colour() {
    let vertices = held_frame(&[held_rig()], [10.0, 1.5, 4.0], &straight());
    // The tube is the wire's own festive green; the string of lights on every third point is
    // deliberately red/green/blue, so the tube must be the majority, not every vertex.
    let green = vertices
      .iter()
      .filter(|vertex| {
        vertex.rgba[1] > vertex.rgba[0] + 0.15 && vertex.rgba[1] > vertex.rgba[2] + 0.15
      })
      .count();
    assert!(
      !vertices.is_empty() && green * 2 > vertices.len(),
      "the preview must keep the festive wire's own green hue, not draw a grey panel: {green} green of {} vertices",
      vertices.len()
    );
  }
}
