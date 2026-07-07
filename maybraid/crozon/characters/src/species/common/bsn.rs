//! Shared BSN scene builders for resolved character assemblies.
//!
//! Layering follows a compose-up pattern: each part kind (`body_rig_scene`,
//! `part_scene`, `gltf_part_scene`) is its own [`Scene`], and species modules
//! compose those into `visual_scene()` / `scene()`. Higher-order consumers can
//! compose species scenes further without reaching back down into asset paths.
//!
//! Clothing, item attachments, and shadow rigs are expressed through the same
//! semantic markers ([`CharacterPartMarker`], [`CharacterSkinTarget`],
//! [`CharacterSocket`]) so runtime systems can implement them by querying the
//! spawned hierarchy rather than by special-cased spawn code. Clothing in
//! particular is *not* part of a species' character scene: compose
//! [`clothing_scene`] over a character scene at a higher layer instead.

use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;

use crate::{
	assembly::{
		CharacterPartSlot, ResolvedCharacterAssembly, ResolvedCharacterPart, RigAsset, SkinTarget,
		SocketRig,
	},
	assets::AssetPath,
};
use crozon_character_items::ClothingMesh;

/// A material family that can accept a flat base color.
///
/// Character scenes are generic over this trait so any material pipeline
/// (standard PBR, toon, preview checkerboard, ...) can receive part colors.
pub trait WithBaseColor: Send + Sync + 'static {
	fn set_base_color(&mut self, color: Color);

	fn with_base_color(mut self, color: Color) -> Self
	where
		Self: Sized,
	{
		self.set_base_color(color);
		self
	}
}

impl WithBaseColor for StandardMaterial {
	fn set_base_color(&mut self, color: Color) {
		self.base_color = color;
	}
}

/// Marks the body rig GLTF entity in a character visual scene.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterBodyRig;

/// Marks which logical character slot an instantiated GLTF entity occupies.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterPartMarker {
	pub slot: CharacterPartSlot,
}

/// Semantic skin-remap target; runtime maps this to a spawned rig entity.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CharacterSkinTarget {
	pub target: SkinTarget,
}

/// Semantic socket placement relative to a body or head rig.
#[derive(Component, Clone, Copy, PartialEq)]
pub struct CharacterSocket {
	pub rig: SocketRig,
	pub bone: &'static str,
	pub local_transform: Transform,
}

impl Default for CharacterSocket {
	fn default() -> Self {
		Self { rig: SocketRig::default(), bone: "", local_transform: Transform::default() }
	}
}

/// Resolved base color for a part, typed by the material family that should
/// receive it. The color itself lives on the character part entity; systems
/// like [`apply_part_base_colors`] push it into `M` materials.
#[derive(Component)]
pub struct PartBaseColor<M: WithBaseColor> {
	pub color: Color,
	marker: PhantomData<fn() -> M>,
}

impl<M: WithBaseColor> PartBaseColor<M> {
	pub fn new(color: Color) -> Self {
		Self { color, marker: PhantomData }
	}
}

impl<M: WithBaseColor> Default for PartBaseColor<M> {
	fn default() -> Self {
		Self::new(Color::WHITE)
	}
}

impl<M: WithBaseColor> Clone for PartBaseColor<M> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<M: WithBaseColor> Copy for PartBaseColor<M> {}

impl<M: WithBaseColor> PartialEq for PartBaseColor<M> {
	fn eq(&self, other: &Self) -> bool {
		self.color == other.color
	}
}

/// Marks a part whose [`PartBaseColor`] has been pushed into its materials.
/// Remove it to force re-application (e.g. after a color change).
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartBaseColorApplied;

/// The body rig GLTF as its own scene, composable by species and consumers.
pub fn body_rig_scene(rig: &RigAsset, label: &'static str) -> impl Scene {
	let path = rig.path.gltf_scene_0();
	bsn! {
		#BodyRig
		Name({format!("{label}_body_rig")})
		WorldAssetRoot({path})
		CharacterBodyRig
		Transform::default()
	}
}

/// One resolved part as its own scene: GLTF root, slot marker, transform,
/// typed base color, and optional skin/socket metadata composed as layers.
pub fn part_scene<M: WithBaseColor>(
	part: &ResolvedCharacterPart,
	transform: Transform,
	color: Color,
) -> impl Scene {
	let path = part.asset.path.gltf_scene_0();
	let slot = part.slot;
	let name = format!("character_{slot:?}_{}", part.asset.label);
	let base_color = PartBaseColor::<M>::new(color);

	let base = bsn! {
		Name({name})
		WorldAssetRoot({path})
		CharacterPartMarker { slot: {slot} }
		template_value(transform)
		template_value(base_color)
	};

	let skin_target = part.skin_target;
	let skin = matches!(skin_target, SkinTarget::BodyRig | SkinTarget::HeadRig)
		.then(|| bsn! { CharacterSkinTarget { target: {skin_target} } });

	let socket = part.socket.map(|socket| {
		bsn! {
			CharacterSocket {
				rig: {socket.rig},
				bone: {socket.bone},
				local_transform: {socket.local_transform},
			}
		}
	});

	(base, skin, socket)
}

/// A standalone GLTF part scene (e.g. an item attachment) outside a full assembly.
pub fn gltf_part_scene<M: WithBaseColor>(
	path: AssetPath,
	slot: CharacterPartSlot,
	transform: Transform,
	color: Color,
) -> impl Scene {
	let asset = path.gltf_scene_0();
	let base_color = PartBaseColor::<M>::new(color);
	bsn! {
		WorldAssetRoot({asset})
		CharacterPartMarker { slot: {slot} }
		template_value(transform)
		template_value(base_color)
	}
}

/// Composes the body rig scene and every resolved part scene into one visual root.
///
/// Clothing parts are skipped: clothing is a higher-order layer composed over
/// the character scene via [`clothing_scene`], not part of the character itself.
pub fn assembly_visual_scene<M: WithBaseColor>(
	assembly: &ResolvedCharacterAssembly,
	part_transform: impl Fn(&ResolvedCharacterPart) -> Transform,
	part_color: impl Fn(&ResolvedCharacterPart) -> Color,
) -> impl Scene {
	let mut children: Vec<Box<dyn Scene>> = Vec::with_capacity(assembly.parts.len() + 1);
	children.push(Box::new(body_rig_scene(&assembly.body_rig, assembly.label)));
	for part in assembly.parts.iter().filter(|part| part.slot != CharacterPartSlot::Clothing) {
		children.push(Box::new(part_scene::<M>(part, part_transform(part), part_color(part))));
	}

	bsn! {
		#VisualRoot
		Transform::default()
		Children [ {children} ]
	}
}

/// A clothing layer as its own scene, composed over a character scene:
///
/// ```ignore
/// let character = config.scene::<StandardMaterial>();
/// let tunic = clothing_scene::<StandardMaterial>(ClothingMesh::Tunic, color);
/// commands.queue_spawn_scene(bsn! {
///     {character}
///     Children [ ({tunic}) ]
/// });
/// ```
///
/// The clothing entity carries the same skin-target metadata as body parts, so
/// the same remap systems fit it onto the body rig wherever it sits in the tree.
pub fn clothing_scene<M: WithBaseColor>(clothing: ClothingMesh, color: Color) -> impl Scene {
	let part = ResolvedCharacterPart::clothing(clothing);
	let transform = part.asset.normalization.transform();
	part_scene::<M>(&part, transform, color)
}

/// Pushes [`PartBaseColor<M>`] into every descendant `M` material once the GLTF
/// instance has spawned meshes. Marks the part with [`PartBaseColorApplied`];
/// remove the marker to re-apply after a color change.
pub fn apply_part_base_colors<M: Material + WithBaseColor>(
	mut commands: Commands,
	mut materials: ResMut<Assets<M>>,
	parts: Query<(Entity, &PartBaseColor<M>), Without<PartBaseColorApplied>>,
	children: Query<&Children>,
	mesh_materials: Query<&MeshMaterial3d<M>>,
) {
	for (entity, base) in &parts {
		let mut applied = false;
		let mut stack: Vec<Entity> = vec![entity];
		while let Some(current) = stack.pop() {
			if let Ok(mesh_material) = mesh_materials.get(current) {
				if let Some(material) = materials.get(&mesh_material.0).cloned() {
					let handle = materials.add(material.with_base_color(base.color));
					commands.entity(current).insert(MeshMaterial3d(handle));
					applied = true;
				}
			}
			if let Ok(kids) = children.get(current) {
				stack.extend(kids.iter());
			}
		}
		if applied {
			commands.entity(entity).insert(PartBaseColorApplied);
		}
	}
}
