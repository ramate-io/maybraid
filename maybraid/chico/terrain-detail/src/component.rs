//! Unit rock meshes. Authored shapes live in `maybraid/art/terrain_detail/`;
//! v1 builds Bevy unit meshes and scales them at spawn ([RFC-170 §3.1.4]).

use bevy::prelude::*;

/// Authored GLB paths relative to the Bevy asset root (`maybraid/assets`).
///
/// Source blends: `maybraid/art/terrain_detail/{rounded_rock,rock_knob,sharp_rock}.blend`.
pub mod assets {
	pub const ROUNDED_ROCK_GLB: &str = "terrain_detail/rounded_rock.glb";
	pub const ROCK_KNOB_GLB: &str = "terrain_detail/rock_knob.glb";
	pub const SHARP_ROCK_GLB: &str = "terrain_detail/sharp_rock.glb";
}

/// Deterministic unit component. Scale is applied at spawn, not in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RockComponent {
	/// Icosphere boulder (`rounded_rock.blend`).
	RoundRock,
	/// Icosphere plus a bottom flange so the rock can sit / sink (`rock_knob.blend`).
	RockKnob,
	/// Pointy crag piece (`sharp_rock.blend`).
	SharpRock,
}

impl RockComponent {
	pub const ALL: [Self; 3] = [Self::RoundRock, Self::RockKnob, Self::SharpRock];

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::RoundRock => "round-rock",
			Self::RockKnob => "rock-knob",
			Self::SharpRock => "sharp-rock",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		let key = name.trim().to_ascii_lowercase();
		Self::ALL.iter().copied().find(|kind| kind.as_kebab() == key)
	}

	pub fn glb_path(self) -> &'static str {
		match self {
			Self::RoundRock => assets::ROUNDED_ROCK_GLB,
			Self::RockKnob => assets::ROCK_KNOB_GLB,
			Self::SharpRock => assets::SHARP_ROCK_GLB,
		}
	}

	/// Unit mesh with the sit plane at y = 0. Scale at spawn for world metres.
	pub fn unit_mesh(self) -> Mesh {
		match self {
			Self::RoundRock => round_rock_mesh(),
			Self::RockKnob => rock_knob_mesh(),
			Self::SharpRock => sharp_rock_mesh(),
		}
	}
}

fn unit_icosphere(radius: f32) -> Mesh {
	Sphere::new(radius)
		.mesh()
		.ico(2)
		.unwrap_or_else(|_| Mesh::from(Sphere::new(radius)))
}

fn round_rock_mesh() -> Mesh {
	// Radius 0.5, lifted so the sit plane is the south pole.
	unit_icosphere(0.5).translated_by(Vec3::new(0.0, 0.5, 0.0))
}

fn rock_knob_mesh() -> Mesh {
	let mut body = unit_icosphere(0.42).translated_by(Vec3::new(0.0, 0.52, 0.0));
	let flange = Mesh::from(Cylinder::new(0.55, 0.16)).translated_by(Vec3::new(0.0, 0.08, 0.0));
	let _ = body.merge(&flange);
	body
}

fn sharp_rock_mesh() -> Mesh {
	// Cone sits on its base; tip points +Y.
	Mesh::from(Cone::new(0.32, 1.0)).translated_by(Vec3::new(0.0, 0.5, 0.0))
}

/// Shared rock albedo for v1 (`StandardMaterial`; not the vegetation bump-out shader).
pub fn rock_material() -> StandardMaterial {
	StandardMaterial {
		base_color: Color::srgb(0.42, 0.40, 0.36),
		perceptual_roughness: 0.92,
		metallic: 0.02,
		..Default::default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn kebab_round_trips_every_component() -> Result<()> {
		for kind in RockComponent::ALL {
			assert_eq!(RockComponent::from_kebab(kind.as_kebab()), Some(kind));
		}
		assert!(RockComponent::from_kebab("not-a-rock").is_none());
		Ok(())
	}

	#[test]
	fn unit_meshes_have_triangles() -> Result<()> {
		for kind in RockComponent::ALL {
			let mesh = kind.unit_mesh();
			let verts = mesh.count_vertices();
			assert!(verts > 8, "{:?} too few verts: {verts}", kind);
		}
		Ok(())
	}

	#[test]
	fn authored_glb_paths_match_blend_stems() -> Result<()> {
		assert_eq!(RockComponent::RoundRock.glb_path(), "terrain_detail/rounded_rock.glb");
		assert_eq!(RockComponent::RockKnob.glb_path(), "terrain_detail/rock_knob.glb");
		assert_eq!(RockComponent::SharpRock.glb_path(), "terrain_detail/sharp_rock.glb");
		Ok(())
	}
}
