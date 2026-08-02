//! Pitch-face openings: passages / apertures clip the nearest half and map contact.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use crate::openings::{
	MappedOpening, MappedOpeningQuad, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings,
};

use super::geometry::RoofHalf;
use super::{PitchedRoof, PitchedRoofParams};

impl PitchedRoof {
	/// Authoring helper: thin AABB on half `half` centered in face UV.
	///
	/// `u` / `v` are normalized face coords (`u` along eave, `v` eave→ridge).
	/// `width` / `height` are meters in the face (along eave / along generator).
	pub fn pitch_opening(
		half: &RoofHalf,
		u: f32,
		v: f32,
		width: f32,
		height: f32,
		label: OpeningLabel,
	) -> Opening {
		let width = width.max(1e-3);
		let height = height.max(1e-3);
		let depth = 0.35;
		let center = half.pitch_point(u.clamp(0.05, 0.95), v.clamp(0.05, 0.95));
		let (eave_x, eave_z) = RoofHalf::eave_frame(half.eave_line);
		// Outward (away from ridge) for a shallow AABB through the pitch.
		let outward = -eave_z;
		let half_w = width * 0.5;
		let half_h = height * 0.5;
		// Approximate face-up along the generator at `u`.
		let eave_u = half.eave_line.0.lerp(half.eave_line.1, u.clamp(0.0, 1.0));
		let ridge_u = half.ridge_line.0.lerp(half.ridge_line.1, u.clamp(0.0, 1.0));
		let up = (ridge_u - eave_u).normalize_or_zero();
		let min = center - eave_x * half_w - up * half_h - outward * depth;
		let max = center + eave_x * half_w + up * half_h + outward * depth * 0.25;
		Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label)
	}
}

/// Per-half pitch clip + mapped opening after resolution.
pub(super) struct PitchOpeningMap {
	pub half: usize,
	pub clip: Vec<Vec3>,
	pub id: OpeningId,
	pub opening: Opening,
	pub mapped: MappedOpening,
}

impl PitchedRoofParams {
	/// Map passages / apertures onto pitch halves: largest area wins per half.
	pub(super) fn resolve_pitch_openings(&self) -> [Option<PitchOpeningMap>; 2] {
		let mut best: [Option<(f32, OpeningId, Opening)>; 2] = [None, None];

		for (id, opening) in self.openings.iter() {
			if !matches!(
				opening.label,
				OpeningLabel::Passage | OpeningLabel::Aperture
			) {
				continue;
			}
			let Some(half) = best_half_for_bounds(&opening.bounds, &self.halves) else {
				continue;
			};
			let score = pitch_extent_score(&opening.bounds, &self.halves[half]);
			let replace = match &best[half] {
				None => true,
				Some((prev, ..)) => score > *prev,
			};
			if replace {
				best[half] = Some((score, id.clone(), opening.clone()));
			}
		}

		let mut out: [Option<PitchOpeningMap>; 2] = [None, None];
		for half in 0..2 {
			let Some((_, id, opening)) = best[half].take() else {
				continue;
			};
			let roof_half = &self.halves[half];
			let (face_width, face_height) = pitch_face_extents(roof_half);
			let (width, height) =
				opening_dims_on_pitch(&opening.bounds, roof_half, face_width, face_height);
			let (u, v) = opening_uv_on_pitch(&opening.bounds, roof_half);
			let clip = centered_pitch_clip(roof_half, u, v, width, height);
			let orientation = RoofHalf::outward_orientation(roof_half.eave_line);
			let mapped = mapped_from_outside_clip(&clip, orientation);
			out[half] = Some(PitchOpeningMap {
				half,
				clip,
				id,
				opening,
				mapped,
			});
		}
		out
	}
}

impl MapsOpenings for PitchedRoof {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}

fn best_half_for_bounds(bounds: &Aabb3d, halves: &[RoofHalf; 2]) -> Option<usize> {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let mut best: Option<(usize, f32)> = None;
	for (i, half) in halves.iter().enumerate() {
		let d = mid.distance_squared(half.pitch_centroid());
		let replace = match best {
			None => true,
			Some((_, prev)) => d < prev,
		};
		if replace {
			best = Some((i, d));
		}
	}
	best.map(|(i, _)| i)
}

fn pitch_extent_score(bounds: &Aabb3d, half: &RoofHalf) -> f32 {
	let e = Vec3::from(bounds.max - bounds.min);
	let (eave_x, _) = RoofHalf::eave_frame(half.eave_line);
	let along = (e.x * eave_x.x.abs() + e.z * eave_x.z.abs()).max(e.x.max(e.z) * 0.5);
	along.max(0.0) * e.y.max(0.0)
}

fn pitch_face_extents(half: &RoofHalf) -> (f32, f32) {
	let (e0, e1) = half.eave_line;
	let (r0, _) = half.ridge_line;
	let face_width = e0.distance(e1).max(1e-4);
	let face_height = e0.distance(r0).max(1e-4);
	(face_width, face_height)
}

fn opening_uv_on_pitch(bounds: &Aabb3d, half: &RoofHalf) -> (f32, f32) {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let (e0, e1) = half.eave_line;
	let (r0, r1) = half.ridge_line;
	let eave_dir = (e1 - e0).normalize_or_zero();
	let eave_len = e0.distance(e1).max(1e-4);
	let u = ((mid - e0).dot(eave_dir) / eave_len).clamp(0.05, 0.95);
	let eave_u = e0.lerp(e1, u);
	let ridge_u = r0.lerp(r1, u);
	let gen = ridge_u - eave_u;
	let gen_len = gen.length().max(1e-4);
	let v = ((mid - eave_u).dot(gen / gen_len) / gen_len).clamp(0.05, 0.95);
	(u, v)
}

fn opening_dims_on_pitch(
	bounds: &Aabb3d,
	half: &RoofHalf,
	face_width: f32,
	face_height: f32,
) -> (f32, f32) {
	let e = Vec3::from(bounds.max - bounds.min);
	let (eave_x, _) = RoofHalf::eave_frame(half.eave_line);
	let width = (e.x * eave_x.x.abs() + e.z * eave_x.z.abs())
		.max(e.x.max(e.z) * 0.5)
		.clamp(face_width * 0.05, face_width * 0.95);
	let height = e.y.clamp(face_height * 0.05, face_height * 0.95);
	(width, height)
}

/// Centered rectangular clip on the pitch face (`[BL, BR, TR, TL]` looking in).
pub(crate) fn centered_pitch_clip(
	half: &RoofHalf,
	u: f32,
	v: f32,
	width: f32,
	height: f32,
) -> Vec<Vec3> {
	let (face_width, face_height) = pitch_face_extents(half);
	let width_frac = (width / face_width).clamp(0.05, 0.95);
	let height_frac = (height / face_height).clamp(0.05, 0.95);
	let u0 = (u - width_frac * 0.5).clamp(0.0, 1.0 - width_frac);
	let u1 = u0 + width_frac;
	let v0 = (v - height_frac * 0.5).clamp(0.0, 1.0 - height_frac);
	let v1 = v0 + height_frac;
	vec![
		half.pitch_point(u0, v0),
		half.pitch_point(u1, v0),
		half.pitch_point(u1, v1),
		half.pitch_point(u0, v1),
	]
}

/// `clip` is `[BL, BR, TR, TL]` looking **in** from outside; map wants looking **out**.
fn mapped_from_outside_clip(clip: &[Vec3], orientation: bevy_math::Vec2) -> MappedOpening {
	debug_assert!(clip.len() >= 4);
	MappedOpening::new(
		MappedOpeningQuad::new(clip[1], clip[0], clip[2], clip[3]),
		orientation,
	)
}
