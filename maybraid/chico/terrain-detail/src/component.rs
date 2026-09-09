//! Unit rock kits. Authored shapes live in `maybraid/art/terrain_detail/` and
//! present as [`SceneRef`] + [`MaterialRef`] ([RFC-170 §3.1.4]).

use bevy::prelude::Color;
use material_ref::MaterialRef;
use scene_ref::SceneRef;

/// Authored GLB paths relative to the Bevy asset root (`maybraid/assets`).
///
/// Source blends: `maybraid/art/terrain_detail/{rounded_rock,rock_knob,sharp_rock}.blend`.
pub mod assets {
	pub const ROUNDED_ROCK_GLB: &str = "terrain_detail/rounded_rock.glb";
	pub const ROCK_KNOB_GLB: &str = "terrain_detail/rock_knob.glb";
	pub const SHARP_ROCK_GLB: &str = "terrain_detail/sharp_rock.glb";
}

/// Named recipe resolved by the host [`material_ref::MaterialLib`] (Standard fallback).
pub const CHICO_ROCK_MATERIAL: &str = "rock";

/// Stone albedo for the Standard fallback (not the vegetation bump-out shader).
pub fn rock_material_ref() -> MaterialRef {
	MaterialRef::named(CHICO_ROCK_MATERIAL).with_palette([Color::srgb(0.42, 0.40, 0.36)])
}

/// Deterministic unit component. Scale is applied at spawn, not in the mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RockComponent {
	/// Boulder (`rounded_rock.blend`).
	RoundRock,
	/// Sit / sink knob (`rock_knob.blend`).
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

	pub fn scene_ref(self) -> SceneRef {
		SceneRef::glb(self.glb_path())
	}

	pub fn material_ref(self) -> MaterialRef {
		rock_material_ref()
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
	fn scene_refs_point_at_authored_glbs() -> Result<()> {
		assert_eq!(RockComponent::RoundRock.scene_ref(), SceneRef::glb(assets::ROUNDED_ROCK_GLB));
		assert_eq!(RockComponent::RockKnob.scene_ref(), SceneRef::glb(assets::ROCK_KNOB_GLB));
		assert_eq!(RockComponent::SharpRock.scene_ref(), SceneRef::glb(assets::SHARP_ROCK_GLB));
		Ok(())
	}

	#[test]
	fn authored_glb_paths_match_blend_stems() -> Result<()> {
		assert_eq!(RockComponent::RoundRock.glb_path(), "terrain_detail/rounded_rock.glb");
		assert_eq!(RockComponent::RockKnob.glb_path(), "terrain_detail/rock_knob.glb");
		assert_eq!(RockComponent::SharpRock.glb_path(), "terrain_detail/sharp_rock.glb");
		Ok(())
	}

	#[test]
	fn material_ref_is_named_rock() -> Result<()> {
		assert_eq!(rock_material_ref().name, material_ref::MaterialId::named(CHICO_ROCK_MATERIAL));
		assert_eq!(RockComponent::RoundRock.material_ref(), rock_material_ref());
		Ok(())
	}
}
