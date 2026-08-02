//! `/show rect-ring-floor` — omitted rectangular ring storey shell.

use bevy::prelude::*;
use clap::Args;
use richmond_buildings::RectRingFloorSide;

use super::opening::{ortho_openings, parse_opening_arg, OpeningArg, OrthoOpeningContext};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RectRingFloor {
	#[arg(long, default_value_t = 8.0)]
	pub outer_x: f32,
	#[arg(long, default_value_t = 6.0)]
	pub outer_z: f32,
	#[arg(long, default_value_t = 4.0)]
	pub inner_x: f32,
	#[arg(long, default_value_t = 3.0)]
	pub inner_z: f32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	/// Omit interval on the outer south side: `start,end` meters from the SW corner.
	#[arg(long, value_name = "START,END")]
	pub omit_south: Option<String>,
	#[arg(long, value_name = "START,END")]
	pub omit_east: Option<String>,
	#[arg(long, value_name = "START,END")]
	pub omit_north: Option<String>,
	#[arg(long, value_name = "START,END")]
	pub omit_west: Option<String>,
	/// Opening plan entries. Repeatable. When set, overrides `--door-*` flags.
	///
	/// Formats: `id:label:minx,miny,minz:maxx,maxy,maxz` or `id:label:side=south`.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[arg(long, default_value_t = false)]
	pub door_north: bool,
	#[arg(long, default_value_t = false)]
	pub door_east: bool,
	#[arg(long, default_value_t = false)]
	pub door_south: bool,
	#[arg(long, default_value_t = false)]
	pub door_west: bool,
	#[arg(long, default_value_t = 1.2)]
	pub door_width: f32,
	#[arg(long, default_value_t = 2.1)]
	pub door_height: f32,
	#[arg(long, default_value_t = false)]
	pub floor: bool,
	#[arg(long, default_value_t = false)]
	pub ceiling: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl RectRingFloor {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let outer = Vec2::new(self.outer_x, self.outer_z);
		let ctx = OrthoOpeningContext {
			center_xz: Vec3::ZERO,
			footprint: outer,
			storey_height: self.storey_height,
			door_width: self.door_width,
			door_height: self.door_height,
		};
		let openings = ortho_openings(
			&self.openings,
			ctx,
			self.door_north,
			self.door_east,
			self.door_south,
			self.door_west,
		)?;
		let mut outer_omits = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
		if let Some(spec) = &self.omit_south {
			outer_omits[RectRingFloorSide::South.face_index()].push(parse_omit_interval(spec)?);
		}
		if let Some(spec) = &self.omit_east {
			outer_omits[RectRingFloorSide::East.face_index()].push(parse_omit_interval(spec)?);
		}
		if let Some(spec) = &self.omit_north {
			outer_omits[RectRingFloorSide::North.face_index()].push(parse_omit_interval(spec)?);
		}
		if let Some(spec) = &self.omit_west {
			outer_omits[RectRingFloorSide::West.face_index()].push(parse_omit_interval(spec)?);
		}
		Ok((
			PreviewSubject::RectRingFloor {
				outer_x: self.outer_x,
				outer_z: self.outer_z,
				inner_x: self.inner_x,
				inner_z: self.inner_z,
				storey_height: self.storey_height,
				outer_omits,
				openings,
				floor: self.floor,
				ceiling: self.ceiling,
			},
			self.transform.transform(),
		))
	}
}

fn parse_omit_interval(s: &str) -> Result<(f32, f32), String> {
	let parts: Vec<_> = s.split(',').map(str::trim).collect();
	if parts.len() != 2 {
		return Err(format!(
			"omit interval expected START,END meters, got {s:?}"
		));
	}
	let a: f32 = parts[0]
		.parse()
		.map_err(|e| format!("omit start: {e}"))?;
	let b: f32 = parts[1].parse().map_err(|e| format!("omit end: {e}"))?;
	Ok((a.min(b), a.max(b)))
}
