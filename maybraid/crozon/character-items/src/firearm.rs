//! Firearm catalog for inventory items.
//!
//! Combat kits live in the `firearms` crate. This crate stores the bag identity:
//! body plus optional slots, per-slot length/thickness, surface look, and bolt look.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{BoltMaterial, FirearmMaterial, ItemColor, ItemRng};

pub const LENGTH_MILLI_MIN: u16 = 500;
pub const LENGTH_MILLI_MAX: u16 = 1500;
pub const THICKNESS_MILLI_MIN: u16 = 800;
pub const THICKNESS_MILLI_MAX: u16 = 1200;
pub const SCALE_MILLI_UNIT: u16 = 1000;

/// Named firearm bodies currently in `items/guns/`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum FirearmMesh {
	#[default]
	Bullpup,
	Silopup,
	Reltor,
	Samsonist,
	Snailer,
}

impl FirearmMesh {
	pub const VALUES: &'static [Self] =
		&[Self::Bullpup, Self::Silopup, Self::Reltor, Self::Samsonist, Self::Snailer];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Bullpup => "bullpup",
			Self::Silopup => "silopup",
			Self::Reltor => "reltor",
			Self::Samsonist => "samsonist",
			Self::Snailer => "snailer",
		}
	}

	/// Unfitted catalog thumbnail relative to the `maybraid/assets` root.
	pub const fn path(self) -> &'static str {
		match self {
			Self::Bullpup => "items/guns/concepts/bullpup_full_concept.glb",
			Self::Silopup => "items/guns/concepts/silopup_full_concept.glb",
			Self::Reltor => "items/guns/bodies/reltor_body.glb",
			Self::Samsonist => "items/guns/bodies/samsonist_body.glb",
			Self::Snailer => "items/guns/bodies/snailer_body.glb",
		}
	}

	/// Body GLB used when assembling a kit.
	pub const fn body_path(self) -> &'static str {
		match self {
			Self::Bullpup => "items/guns/bodies/bullpup_body.glb",
			Self::Silopup => "items/guns/bodies/silopup_body.glb",
			Self::Reltor => "items/guns/bodies/reltor_body.glb",
			Self::Samsonist => "items/guns/bodies/samsonist_body.glb",
			Self::Snailer => "items/guns/bodies/snailer_body.glb",
		}
	}

	/// Nouns a rolled item of this mesh may be called.
	pub const fn nouns(self) -> &'static [&'static str] {
		match self {
			Self::Bullpup => &["Bullpup", "Carbine", "Short Rifle", "Compact", "Issue Rifle"],
			Self::Silopup => &["Silopup", "Suppressor", "Quiet Rifle", "Hush Gun", "Whisper"],
			Self::Reltor => &["Reltor", "Receiver", "Box Gun", "Service Rifle", "Latch Gun"],
			Self::Samsonist => &["Samsonist", "Long Rifle", "Pike", "Reach Gun", "Lance"],
			Self::Snailer => &["Snailer", "Coil Gun", "Helix", "Spiral", "Shell Gun"],
		}
	}

	/// Authored concept kit for this body (used when a save only stored the mesh).
	pub const fn concept_kit(self) -> FirearmKitSpec {
		match self {
			Self::Bullpup => FirearmKitSpec {
				body: Self::Bullpup,
				barrel: FirearmBarrel::Bullpup,
				grip: FirearmGrip::BumpHandle,
				trigger_box: FirearmTriggerBox::None,
				stock: FirearmStock::None,
			},
			Self::Reltor => FirearmKitSpec {
				body: Self::Reltor,
				barrel: FirearmBarrel::None,
				grip: FirearmGrip::None,
				trigger_box: FirearmTriggerBox::Reltor,
				stock: FirearmStock::None,
			},
			body => FirearmKitSpec {
				body,
				barrel: FirearmBarrel::None,
				grip: FirearmGrip::None,
				trigger_box: FirearmTriggerBox::None,
				stock: FirearmStock::None,
			},
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum FirearmBarrel {
	#[default]
	None,
	Bullpup,
	Laznard,
}

impl FirearmBarrel {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Bullpup, Self::Laznard];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Bullpup => "bullpup",
			Self::Laznard => "laznard",
		}
	}

	pub const fn path(self) -> Option<&'static str> {
		match self {
			Self::None => None,
			Self::Bullpup => Some("items/guns/barrels/bullpup_barrel.glb"),
			Self::Laznard => Some("items/guns/barrels/laznard_barrel.glb"),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum FirearmGrip {
	#[default]
	None,
	BumpHandle,
}

impl FirearmGrip {
	pub const VALUES: &'static [Self] = &[Self::None, Self::BumpHandle];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::BumpHandle => "bump-handle",
		}
	}

	pub const fn path(self) -> Option<&'static str> {
		match self {
			Self::None => None,
			Self::BumpHandle => Some("items/guns/grips/bump_handle.glb"),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum FirearmTriggerBox {
	#[default]
	None,
	Keelripe,
	Paddle,
	Reltor,
}

impl FirearmTriggerBox {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Keelripe, Self::Paddle, Self::Reltor];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Keelripe => "keelripe",
			Self::Paddle => "paddle",
			Self::Reltor => "reltor",
		}
	}

	pub const fn path(self) -> Option<&'static str> {
		match self {
			Self::None => None,
			Self::Keelripe => Some("items/guns/trigger_boxes/keelripe_box.glb"),
			Self::Paddle => Some("items/guns/trigger_boxes/paddle_box.glb"),
			Self::Reltor => Some("items/guns/trigger_boxes/reltor_box.glb"),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum FirearmStock {
	#[default]
	None,
}

impl FirearmStock {
	pub const VALUES: &'static [Self] = &[Self::None];

	pub const fn label(self) -> &'static str {
		"none"
	}

	pub const fn path(self) -> Option<&'static str> {
		None
	}
}

/// Assembled kit identity. Body is required; other slots may be empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FirearmKitSpec {
	pub body: FirearmMesh,
	pub barrel: FirearmBarrel,
	pub grip: FirearmGrip,
	pub trigger_box: FirearmTriggerBox,
	pub stock: FirearmStock,
}

impl FirearmKitSpec {
	pub const fn from_body(body: FirearmMesh) -> Self {
		body.concept_kit()
	}
}

impl Default for FirearmKitSpec {
	fn default() -> Self {
		FirearmMesh::Bullpup.concept_kit()
	}
}

/// Length and thickness in millunits (`1000` = rest scale 1.0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SlotScale {
	pub length_milli: u16,
	pub thickness_milli: u16,
}

impl SlotScale {
	pub const UNIT: Self =
		Self { length_milli: SCALE_MILLI_UNIT, thickness_milli: SCALE_MILLI_UNIT };

	pub const fn length(self) -> f32 {
		self.length_milli as f32 / SCALE_MILLI_UNIT as f32
	}

	pub const fn thickness(self) -> f32 {
		self.thickness_milli as f32 / SCALE_MILLI_UNIT as f32
	}

	/// Tenths over 1.0 length (`1.5` → `5.0`, `0.5` → `-5.0`).
	pub fn length_tenths(self) -> f32 {
		(self.length_milli as f32 - SCALE_MILLI_UNIT as f32) / 100.0
	}

	pub fn thickness_tenths(self) -> f32 {
		(self.thickness_milli as f32 - SCALE_MILLI_UNIT as f32) / 100.0
	}

	pub fn roll(rng: &mut ItemRng) -> Self {
		Self {
			length_milli: rng.in_range(u32::from(LENGTH_MILLI_MIN), u32::from(LENGTH_MILLI_MAX))
				as u16,
			thickness_milli: rng
				.in_range(u32::from(THICKNESS_MILLI_MIN), u32::from(THICKNESS_MILLI_MAX))
				as u16,
		}
	}
}

impl Default for SlotScale {
	fn default() -> Self {
		Self::UNIT
	}
}

/// Per-slot scales in Body → Barrel → Grip → Trigger box → Stock order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FirearmScales {
	pub body: SlotScale,
	pub barrel: SlotScale,
	pub grip: SlotScale,
	pub trigger_box: SlotScale,
	pub stock: SlotScale,
}

impl FirearmScales {
	pub const UNIT: Self = Self {
		body: SlotScale::UNIT,
		barrel: SlotScale::UNIT,
		grip: SlotScale::UNIT,
		trigger_box: SlotScale::UNIT,
		stock: SlotScale::UNIT,
	};

	pub fn roll(rng: &mut ItemRng) -> Self {
		Self {
			body: SlotScale::roll(rng),
			barrel: SlotScale::roll(rng),
			grip: SlotScale::roll(rng),
			trigger_box: SlotScale::roll(rng),
			stock: SlotScale::roll(rng),
		}
	}

	pub fn slots(self) -> [(bool, SlotScale); 5] {
		[
			(false, self.body),
			(true, self.barrel),
			(false, self.grip),
			(false, self.trigger_box),
			(false, self.stock),
		]
	}
}

impl Default for FirearmScales {
	fn default() -> Self {
		Self::UNIT
	}
}

/// Authored kit bones are 1m. Handheld rest is this fraction of authored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotRest {
	pub length: f32,
	pub thickness: f32,
}

impl SlotRest {
	pub const BODY: Self = Self { length: 0.30, thickness: 0.16 };
	pub const BARREL: Self = Self { length: 0.38, thickness: 0.07 };
	pub const GRIP: Self = Self { length: 0.12, thickness: 0.09 };
	pub const TRIGGER_BOX: Self = Self { length: 0.10, thickness: 0.09 };
	pub const STOCK: Self = Self { length: 0.22, thickness: 0.11 };

	pub const fn applied(self, scale: SlotScale) -> (f32, f32) {
		(self.length * scale.length(), self.thickness * scale.thickness())
	}
}

impl SlotScale {
	pub const fn applied_length(self, rest: SlotRest) -> f32 {
		rest.length * self.length()
	}

	pub const fn applied_thickness(self, rest: SlotRest) -> f32 {
		rest.thickness * self.thickness()
	}
}

impl FirearmScales {
	/// Length / thickness to stamp on kit bones (rest × rolled multiplier).
	pub fn bone_fits(self) -> [(&'static str, f32, f32); 5] {
		let body = SlotRest::BODY.applied(self.body);
		let barrel = SlotRest::BARREL.applied(self.barrel);
		let grip = SlotRest::GRIP.applied(self.grip);
		let trigger = SlotRest::TRIGGER_BOX.applied(self.trigger_box);
		let stock = SlotRest::STOCK.applied(self.stock);
		[
			("body", body.0, body.1),
			("barrel", barrel.0, barrel.1),
			("grip", grip.0, grip.1),
			("trigger_box", trigger.0, trigger.1),
			("stock", stock.0, stock.1),
		]
	}
}

/// One kit slot's surface recipe and palette color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SlotLook {
	pub material: FirearmMaterial,
	pub color: ItemColor,
}

impl SlotLook {
	pub const fn new(material: FirearmMaterial, color: ItemColor) -> Self {
		Self { material, color }
	}

	pub fn roll(rng: &mut ItemRng) -> Self {
		Self {
			material: *rng
				.choose(FirearmMaterial::VALUES)
				.unwrap_or(&FirearmMaterial::BrushedMetal),
			color: *rng.choose(ItemColor::VALUES).unwrap_or(&ItemColor::Natural),
		}
	}
}

impl Default for SlotLook {
	fn default() -> Self {
		Self::new(FirearmMaterial::BrushedMetal, ItemColor::Natural)
	}
}

/// Per-slot looks in Body → Barrel → Grip → Trigger box → Stock order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FirearmLooks {
	pub body: SlotLook,
	pub barrel: SlotLook,
	pub grip: SlotLook,
	pub trigger_box: SlotLook,
	pub stock: SlotLook,
}

impl FirearmLooks {
	pub const fn uniform(material: FirearmMaterial, color: ItemColor) -> Self {
		let look = SlotLook::new(material, color);
		Self { body: look, barrel: look, grip: look, trigger_box: look, stock: look }
	}

	pub fn roll(rng: &mut ItemRng) -> Self {
		Self {
			body: SlotLook::roll(rng),
			barrel: SlotLook::roll(rng),
			grip: SlotLook::roll(rng),
			trigger_box: SlotLook::roll(rng),
			stock: SlotLook::roll(rng),
		}
	}
}

impl Default for FirearmLooks {
	fn default() -> Self {
		Self::uniform(FirearmMaterial::BrushedMetal, ItemColor::Natural)
	}
}

/// Full firearm identity used to hash stats and rebuild the kit at spawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FirearmSpec {
	pub kit: FirearmKitSpec,
	pub scales: FirearmScales,
	pub looks: FirearmLooks,
	pub bolt: BoltMaterial,
}

impl FirearmSpec {
	/// Concept kit, rest scale, brushed metal, natural, plain laser.
	pub fn from_mesh(mesh: FirearmMesh) -> Self {
		Self {
			kit: mesh.concept_kit(),
			scales: FirearmScales::UNIT,
			looks: FirearmLooks::uniform(FirearmMaterial::BrushedMetal, ItemColor::Natural),
			bolt: BoltMaterial::PlainLaser,
		}
	}

	pub fn roll(rng: &mut ItemRng, body: FirearmMesh) -> Self {
		Self {
			kit: FirearmKitSpec {
				body,
				barrel: *rng.choose(FirearmBarrel::VALUES).unwrap_or(&FirearmBarrel::None),
				grip: *rng.choose(FirearmGrip::VALUES).unwrap_or(&FirearmGrip::None),
				trigger_box: *rng
					.choose(FirearmTriggerBox::VALUES)
					.unwrap_or(&FirearmTriggerBox::None),
				stock: FirearmStock::None,
			},
			scales: FirearmScales::roll(rng),
			looks: FirearmLooks::roll(rng),
			bolt: *rng.choose(BoltMaterial::VALUES).unwrap_or(&BoltMaterial::PlainLaser),
		}
	}

	pub fn identity_label(self) -> String {
		format!(
			"body={} barrel={} grip={} trigger-box={} stock={} looks={}/{}/{}/{}/{} bolt={} scales={}:{}/{}:{}/{}:{}/{}:{}/{}:{}",
			self.kit.body.label(),
			self.kit.barrel.label(),
			self.kit.grip.label(),
			self.kit.trigger_box.label(),
			self.kit.stock.label(),
			look_label(self.looks.body),
			look_label(self.looks.barrel),
			look_label(self.looks.grip),
			look_label(self.looks.trigger_box),
			look_label(self.looks.stock),
			self.bolt.label(),
			self.scales.body.length_milli,
			self.scales.body.thickness_milli,
			self.scales.barrel.length_milli,
			self.scales.barrel.thickness_milli,
			self.scales.grip.length_milli,
			self.scales.grip.thickness_milli,
			self.scales.trigger_box.length_milli,
			self.scales.trigger_box.thickness_milli,
			self.scales.stock.length_milli,
			self.scales.stock.thickness_milli,
		)
	}
}

fn look_label(look: SlotLook) -> String {
	format!("{}:{}", look.material.label(), look.color.label())
}

impl Default for FirearmSpec {
	fn default() -> Self {
		Self::from_mesh(FirearmMesh::Bullpup)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn labels_are_kebab_case() {
		assert_eq!(FirearmMesh::Bullpup.label(), "bullpup");
		assert_eq!(FirearmMesh::Snailer.label(), "snailer");
		assert_eq!(FirearmBarrel::Laznard.label(), "laznard");
		assert_eq!(FirearmGrip::BumpHandle.label(), "bump-handle");
	}

	#[test]
	fn bullpup_concept_has_barrel_and_grip() {
		let kit = FirearmMesh::Bullpup.concept_kit();
		assert_eq!(kit.barrel, FirearmBarrel::Bullpup);
		assert_eq!(kit.grip, FirearmGrip::BumpHandle);
	}

	#[test]
	fn unit_scale_is_handheld_rest_not_one_meter() {
		let fits = FirearmScales::UNIT.bone_fits();
		assert!((fits[0].1 - SlotRest::BODY.length).abs() < 1e-5);
		assert!((fits[1].1 - SlotRest::BARREL.length).abs() < 1e-5);
		assert!(fits[0].1 < 0.5);
	}

	#[test]
	fn millunit_multiplies_handheld_rest() {
		let scale = SlotScale { length_milli: 1500, thickness_milli: 800 };
		let (length, thickness) = SlotRest::BODY.applied(scale);
		assert!((length - SlotRest::BODY.length * 1.5).abs() < 1e-5);
		assert!((thickness - SlotRest::BODY.thickness * 0.8).abs() < 1e-5);
	}

	#[test]
	fn rolled_looks_are_independent_per_slot() {
		let items = crate::random_gallery_firearms(&mut ItemRng::from_seed(99), 40);
		assert!(items.iter().any(|item| {
			let looks = item.firearm_spec().unwrap().looks;
			looks.body != looks.barrel || looks.body != looks.grip
		}));
	}
}
