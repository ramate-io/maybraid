//! Shared `--opening` CLI parsing for shell previews.
//!
//! Formats:
//! - AABB: `id:label:minx,miny,minz:maxx,maxy,maxz`
//! - Arc ring locus: `id:label:t=0.5` (resolved with shell radius / height)

use bevy::prelude::*;
use richmond_buildings::{
	side_passage_opening, ArcFloor, Opening, OpeningId, OpeningLabel, Openings, TrazaloidSide,
};

use super::transform::parse_vec3_csv;

/// One authored opening from the playground CLI.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewOpening {
	pub id: String,
	pub label: OpeningLabel,
	pub min: Vec3,
	pub max: Vec3,
}

impl PreviewOpening {
	pub fn bounds(&self) -> bevy_math::bounding::Aabb3d {
		bevy_math::bounding::Aabb3d::from_min_max(self.min.min(self.max), self.min.max(self.max))
	}

	pub fn into_pair(self) -> (OpeningId, Opening) {
		let bounds = self.bounds();
		(OpeningId::new(self.id), Opening::new(bounds, self.label))
	}
}

/// Parse `--opening` values into a plan [`Openings`] table.
pub fn openings_from_preview(openings: &[PreviewOpening]) -> Openings {
	let mut out = Openings::new();
	for opening in openings {
		let (id, o) = opening.clone().into_pair();
		out.insert(id, o);
	}
	out
}

/// Parse one `--opening` string.
///
/// - `id:label:x0,y0,z0:x1,y1,z1`
/// - `id:label:t=0.25` (arc-ring shorthand; requires [`resolve_arc_opening`])
pub fn parse_opening_arg(s: &str) -> Result<OpeningArg, String> {
	let s = s.trim();
	let parts: Vec<_> = s.split(':').collect();
	if parts.len() < 3 {
		return Err(format!(
			"expected id:label:min:max or id:label:t=…, got {s:?}"
		));
	}
	let id = parts[0].trim();
	if id.is_empty() {
		return Err("opening id must be non-empty".into());
	}
	let label = parse_opening_label(parts[1].trim())?;
	let rest = parts[2..].join(":");
	let rest = rest.trim();
	if let Some(t_str) = rest.strip_prefix("t=") {
		let t: f32 = t_str
			.trim()
			.parse()
			.map_err(|e| format!("t=: {e}"))?;
		return Ok(OpeningArg::ArcT {
			id: id.to_string(),
			label,
			t,
		});
	}
	// min:max where each is x,y,z — rest may be "x,y,z:x,y,z"
	let aabb_parts: Vec<_> = rest.splitn(2, ':').collect();
	if aabb_parts.len() != 2 {
		return Err(format!(
			"expected minx,miny,minz:maxx,maxy,maxz after label, got {rest:?}"
		));
	}
	let min = parse_vec3_csv(aabb_parts[0].trim())?;
	let max = parse_vec3_csv(aabb_parts[1].trim())?;
	Ok(OpeningArg::Aabb {
		id: id.to_string(),
		label,
		min,
		max,
	})
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpeningArg {
	Aabb {
		id: String,
		label: OpeningLabel,
		min: Vec3,
		max: Vec3,
	},
	ArcT {
		id: String,
		label: OpeningLabel,
		t: f32,
	},
}

impl OpeningArg {
	pub fn resolve_aabb(
		self,
		arc: Option<ArcOpeningContext>,
	) -> Result<PreviewOpening, String> {
		match self {
			Self::Aabb { id, label, min, max } => Ok(PreviewOpening { id, label, min, max }),
			Self::ArcT { id, label, t } => {
				let ctx = arc.ok_or_else(|| {
					"opening t=… is only valid for arc-floor / arc-tower previews".to_string()
				})?;
				let (_id, opening) = ArcFloor::plan_opening_at_t(
					id.clone(),
					label.clone(),
					ctx.center_xz,
					ctx.radius,
					ctx.storey_height,
					t,
				);
				let min = Vec3::from(opening.bounds.min);
				let max = Vec3::from(opening.bounds.max);
				Ok(PreviewOpening { id, label, min, max })
			}
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub struct ArcOpeningContext {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
}

pub fn parse_opening_label(s: &str) -> Result<OpeningLabel, String> {
	let s = s.trim().to_ascii_lowercase();
	match s.as_str() {
		"boundary" => Ok(OpeningLabel::Boundary),
		"exclusion" => Ok(OpeningLabel::Exclusion),
		"passage" | "door" => Ok(OpeningLabel::Passage),
		"aperture" | "window" => Ok(OpeningLabel::Aperture),
		"shaft" => Ok(OpeningLabel::Shaft),
		other if other.starts_with("custom=") => {
			Ok(OpeningLabel::Custom(other["custom=".len()..].to_string()))
		}
		other => Err(format!(
			"unknown opening label {other:?}; expected boundary|exclusion|passage|aperture|shaft|custom=…"
		)),
	}
}

/// Build trazaloid plan openings from CLI args, falling back to cardinal door flags.
pub fn trazaloid_openings(
	args: &[OpeningArg],
	footprint: Vec2,
	lower_height: f32,
	door_thickness: f32,
	door_width_frac: f32,
	door_height_frac: f32,
	door_north: bool,
	door_east: bool,
	door_south: bool,
	door_west: bool,
) -> Result<Vec<PreviewOpening>, String> {
	if !args.is_empty() {
		return args
			.iter()
			.cloned()
			.map(|a| a.resolve_aabb(None))
			.collect();
	}
	let door_w = if door_thickness > 0.0 {
		door_thickness
	} else {
		footprint.x.min(footprint.y) * door_width_frac
	};
	let door_h = lower_height * door_height_frac;
	let mut out = Vec::new();
	for (enabled, side, id) in [
		(door_north, TrazaloidSide::North, "north"),
		(door_east, TrazaloidSide::East, "east"),
		(door_south, TrazaloidSide::South, "south"),
		(door_west, TrazaloidSide::West, "west"),
	] {
		if !enabled {
			continue;
		}
		let opening = side_passage_opening(side, footprint, door_w, door_h);
		out.push(PreviewOpening {
			id: id.to_string(),
			label: OpeningLabel::Passage,
			min: Vec3::from(opening.bounds.min),
			max: Vec3::from(opening.bounds.max),
		});
	}
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_aabb_opening() -> Result<(), String> {
		let arg = parse_opening_arg("south:passage:-0.6,0,-3.2:0.6,2.1,-2.8")?;
		let preview = arg.resolve_aabb(None)?;
		assert_eq!(preview.id, "south");
		assert_eq!(preview.label, OpeningLabel::Passage);
		assert!((preview.min.x - -0.6).abs() < 1e-5);
		assert!((preview.max.y - 2.1).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn parse_arc_t_opening() -> Result<(), String> {
		let arg = parse_opening_arg("door:passage:t=0.0")?;
		let preview = arg.resolve_aabb(Some(ArcOpeningContext {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
		}))?;
		assert_eq!(preview.id, "door");
		assert!(preview.max.x > 3.0, "t=0 should resolve to +X");
		Ok(())
	}
}
