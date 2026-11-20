//! Opt-in buffered investigation. Absent from ordinary applications; no quality changes.
use bevy::prelude::*;
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
	#[default]
	Normal,
	NoRebuild,
	NoUpload,
}
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Stage {
	Tick,
	Cache,
	Flow,
	Authored,
	Advect,
	Response,
	Ground,
	Broad,
	Deform,
	ScalarSetup,
	Scalar,
	Gradient,
	Tetra,
	Package,
	Insert,
	Bindings,
	NoiseInit,
}
pub const STAGES: [&str; 17] = [
	"tick",
	"cache",
	"flow",
	"authored",
	"advect",
	"response",
	"ground",
	"broad",
	"deform",
	"scalar_setup",
	"scalar",
	"gradient",
	"tetra",
	"package",
	"insert",
	"bindings",
	"noise_init",
];
#[derive(Default, Clone)]
pub struct Frame {
	pub ns: [u64; 17],
	pub ticks: u32,
	pub flow_steps: u32,
	pub broad_checks: u32,
	pub rebuilds: u32,
	pub replacements: u32,
	pub vertices: usize,
	pub triangles: usize,
	pub upload_bytes: usize,
	pub age: f32,
	pub tick: u32,
	pub debt: u32,
	pub bodies: usize,
	pub affected: usize,
	pub near_solid: usize,
	pub mesh_events: usize,
	pub mesh_components: usize,
	pub material_events: usize,
}
#[derive(Resource)]
pub struct Probe {
	pub frame: Frame,
	pub mode: Mode,
	pub affected: [bool; 48],
	pub near_solid: [bool; 48],
}
macro_rules! timed {
 ($p:expr, $stage:ident, $e:expr) => {{
  let stamp = $p.as_ref().map(|_| std::time::Instant::now());
  let value = $e;
  if let (Some(p), Some(t)) = ($p.as_deref_mut(), stamp) { p.frame.ns[$crate::parts::cartoon_flow::diagnostics::Stage::$stage as usize] += t.elapsed().as_nanos() as u64; }
  value
 }};
}
pub(crate) use timed;

impl Default for Probe {
	fn default() -> Self {
		Self {
			frame: Frame::default(),
			mode: Mode::Normal,
			affected: [false; 48],
			near_solid: [false; 48],
		}
	}
}
