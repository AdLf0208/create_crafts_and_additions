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
    let rgba = [base[0] * shade, base[1] * shade, base[2] * shade, 1.0];
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
      push_light(out, m, start, delta, distance, step);
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
) {
  const COLOURS: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
  if step == 0 || step == SEGMENTS || !step.is_multiple_of(3) {
    return;
  }
  let main = delta[0] + delta[1] + delta[2] > 0.0;
  let cycle = (step / 3) % 3;
  let colour = COLOURS[if main { 2 - cycle } else { cycle }];
  let rgba = [colour[0], colour[1], colour[2], 1.0];
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
    Wire { from, to, kind }
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
    for vertex in &vertices {
      let shade = vertex.rgba[0] / base[0];
      assert!(
        (shade - 1.0).abs() < 1e-6 || (shade - 0.7).abs() < 1e-6,
        "a copper wire is drawn in copper's colour, shaded only by the jar's 0.7: {vertex:?}"
      );
      assert!((vertex.rgba[1] / base[1] - shade).abs() < 1e-6);
      assert!((vertex.rgba[2] / base[2] - shade).abs() < 1e-6);
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
    // The lights are the jar's red, green and blue, at full brightness.
    for wanted in [
      [1.0, 0.0, 0.0, 1.0],
      [0.0, 1.0, 0.0, 1.0],
      [0.0, 0.0, 1.0, 1.0],
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
}
