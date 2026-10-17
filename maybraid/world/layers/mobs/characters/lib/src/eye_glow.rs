//! Per-character eye warning glow on the `face_eye` material path.
//!
//! [`EyeGlowControl`] on [`CharacterRoot`] drives runtime glow. Per-character
//! isolation uses [`EyeGlowMaterialInstance`] on eye part [`MaterialRefRoot`]
//! hosts so [`FaceMaterialLib`] fulfills instanced [`FaceShaderMaterial`] handles
//! (see [`crate::material_lib::FaceShaderMaterialRefCache`]).

use bevy::prelude::*;
use character_shaders::{
	set_eye_glow_uniform, FaceShaderMaterial, GOD_BEAM_EYE_GLOW_COLOR,
	GOD_BEAM_EYE_GLOW_EMISSIVE_GAIN, GOD_BEAM_EYE_GLOW_INTENSITY,
};
use material_ref::{MaterialRefApplied, MaterialRefSystems};

use crate::assembly::CharacterPartSlot;
use crate::member::{CharacterMembers, CharacterRoot};
use crate::nodes::PartNode;

/// Default fade duration when [`EyeGlowControl::start_warning`] omits one.
pub const DEFAULT_EYE_GLOW_TRANSITION_SECS: f32 = 0.35;

/// Runtime warning glow for one character's eyes (both sockets).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct EyeGlowControl {
	pub color: Color,
	pub intensity: f32,
	pub target_intensity: f32,
	pub emissive_gain: f32,
	pub transition_secs: f32,
}

impl Default for EyeGlowControl {
	fn default() -> Self {
		Self::off()
	}
}

impl EyeGlowControl {
	pub fn off() -> Self {
		Self {
			color: GOD_BEAM_EYE_GLOW_COLOR,
			intensity: 0.0,
			target_intensity: 0.0,
			emissive_gain: GOD_BEAM_EYE_GLOW_EMISSIVE_GAIN,
			transition_secs: DEFAULT_EYE_GLOW_TRANSITION_SECS,
		}
	}

	/// Documented God Beam telegraph preset (color + full intensity).
	pub fn god_beam_warning() -> Self {
		Self {
			color: GOD_BEAM_EYE_GLOW_COLOR,
			intensity: 0.0,
			target_intensity: GOD_BEAM_EYE_GLOW_INTENSITY,
			emissive_gain: GOD_BEAM_EYE_GLOW_EMISSIVE_GAIN,
			transition_secs: DEFAULT_EYE_GLOW_TRANSITION_SECS,
		}
	}

	pub fn start_warning(
		&mut self,
		color: Color,
		intensity: f32,
		transition_secs: Option<f32>,
	) {
		self.color = color;
		self.target_intensity = intensity.max(0.0);
		if let Some(secs) = transition_secs {
			self.transition_secs = secs.max(0.0);
		}
	}

	pub fn start_god_beam_warning(&mut self) {
		*self = Self::god_beam_warning();
	}

	/// Fade glow off using the configured transition duration.
	pub fn cancel(&mut self) {
		self.target_intensity = 0.0;
	}

	/// Interrupt any fade and snap to ordinary eyes.
	pub fn reset(&mut self) {
		self.intensity = 0.0;
		self.target_intensity = 0.0;
	}
}

/// Instanced face fulfill owner (character root). Lives on the eye part host with
/// [`MaterialRefRoot`] + [`PropagateToDescendants`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EyeGlowMaterialInstance {
	pub owner: Entity,
}

pub struct EyeGlowPlugin;

impl Plugin for EyeGlowPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				sync_eye_glow_material_instances.before(MaterialRefSystems::Invalidate),
				update_eye_glow.after(MaterialRefSystems::Fulfill),
			),
		);
	}
}

fn sync_eye_glow_material_instances(
	mut commands: Commands,
	roots: Query<(Entity, &CharacterMembers, Has<EyeGlowControl>), With<CharacterRoot>>,
	parts: Query<&PartNode>,
	children: Query<&Children>,
	instances: Query<(), With<EyeGlowMaterialInstance>>,
	applied: Query<(), With<MaterialRefApplied>>,
) {
	for (root, members, has_glow) in &roots {
		for member in members.iter() {
			let Ok(part) = parts.get(member) else {
				continue;
			};
			if !is_eye_slot(part.slot) {
				continue;
			}
			let wants_instance = has_glow;
			let has_instance = instances.contains(member);
			if wants_instance == has_instance {
				continue;
			}
			if wants_instance {
				commands.entity(member).insert(EyeGlowMaterialInstance { owner: root });
			} else {
				commands.entity(member).remove::<EyeGlowMaterialInstance>();
			}
			invalidate_material_ref_tree(&mut commands, member, &children, &applied);
		}
	}
}

fn invalidate_material_ref_tree(
	commands: &mut Commands,
	root: Entity,
	children: &Query<&Children>,
	applied: &Query<(), With<MaterialRefApplied>>,
) {
	commands.entity(root).try_remove::<MaterialRefApplied>();
	let mut stack: Vec<Entity> =
		children.get(root).map(|c| c.iter().collect()).unwrap_or_default();
	while let Some(child) = stack.pop() {
		if applied.contains(child) {
			commands.entity(child).try_remove::<MaterialRefApplied>();
		}
		if let Ok(kids) = children.get(child) {
			stack.extend(kids.iter());
		}
	}
}

fn update_eye_glow(
	time: Res<Time>,
	mut roots: Query<(&mut EyeGlowControl, &CharacterMembers), With<CharacterRoot>>,
	parts: Query<&PartNode>,
	children: Query<&Children>,
	meshes: Query<&MeshMaterial3d<FaceShaderMaterial>>,
	mut face_materials: ResMut<Assets<FaceShaderMaterial>>,
) {
	let dt = time.delta_secs();
	for (mut control, members) in &mut roots {
		tick_eye_glow_intensity(&mut control, dt);
		apply_eye_glow_to_members(
			&control,
			members,
			&parts,
			&children,
			&meshes,
			&mut face_materials,
		);
	}
}

fn tick_eye_glow_intensity(control: &mut EyeGlowControl, dt: f32) {
	let target = control.target_intensity.max(0.0);
	if (control.intensity - target).abs() < 1e-4 {
		control.intensity = target;
		return;
	}
	if control.transition_secs <= 1e-4 {
		control.intensity = target;
		return;
	}
	let step = dt / control.transition_secs;
	if control.intensity < target {
		control.intensity = (control.intensity + step).min(target);
	} else {
		control.intensity = (control.intensity - step).max(target);
	}
}

fn apply_eye_glow_to_members(
	control: &EyeGlowControl,
	members: &CharacterMembers,
	parts: &Query<&PartNode>,
	children: &Query<&Children>,
	meshes: &Query<&MeshMaterial3d<FaceShaderMaterial>>,
	face_materials: &mut Assets<FaceShaderMaterial>,
) {
	for member in members.iter() {
		let Ok(part) = parts.get(member) else {
			continue;
		};
		if !is_eye_slot(part.slot) {
			continue;
		}
		let mut stack: Vec<Entity> =
			children.get(member).map(|c| c.iter().collect()).unwrap_or_default();
		while let Some(entity) = stack.pop() {
			if let Ok(material) = meshes.get(entity) {
				if let Some(mut shader) = face_materials.get_mut(&material.0) {
					set_eye_glow_uniform(
						&mut shader.params,
						control.color,
						control.intensity,
						control.emissive_gain,
					);
				}
			}
			if let Ok(kids) = children.get(entity) {
				stack.extend(kids.iter());
			}
		}
	}
}

fn is_eye_slot(slot: CharacterPartSlot) -> bool {
	matches!(slot, CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::material_lib::init_character_material_caches;
	use character_shaders::{eye_glow_intensity, FaceShaderMaterialPlugin};
	use character_shaders::RECIPE_FACE_EYE;
	use material_ref::{
		MaterialRef, MaterialRefPlugin, MaterialRefRoot, PropagateToDescendants,
	};

	use crate::material_lib::FaceMaterialLib;

	fn base_face_ref() -> MaterialRef {
		MaterialRef::named(RECIPE_FACE_EYE).with_palette([Color::srgb(0.2, 0.5, 0.9)])
	}

	#[test]
	fn reset_snaps_off_and_cancel_targets_zero() {
		let mut control = EyeGlowControl::god_beam_warning();
		control.intensity = 0.8;
		control.cancel();
		assert_eq!(control.target_intensity, 0.0);
		control.reset();
		assert_eq!(control.intensity, 0.0);
		assert_eq!(control.target_intensity, 0.0);
	}

	#[test]
	fn tick_reaches_target_over_transition() {
		let mut control = EyeGlowControl::god_beam_warning();
		control.intensity = 0.0;
		control.transition_secs = 1.0;
		tick_eye_glow_intensity(&mut control, 0.5);
		assert!((control.intensity - 0.5).abs() < 1e-4);
		tick_eye_glow_intensity(&mut control, 0.5);
		assert!((control.intensity - 1.0).abs() < 1e-4);
	}

	#[test]
	fn restamp_after_glow_keeps_other_characters_isolated() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.init_asset::<FaceShaderMaterial>()
			.add_plugins(FaceShaderMaterialPlugin);
		init_character_material_caches(&mut app);
		app.add_plugins(MaterialRefPlugin::<FaceMaterialLib<'_, '_>>::default());

		let mesh = app
			.world_mut()
			.resource_mut::<Assets<Mesh>>()
			.add(Mesh::from(Cuboid::from_length(0.1)));
		let base = base_face_ref();
		let owner_a = app.world_mut().spawn_empty().id();
		let host_a = app
			.world_mut()
			.spawn((
				MaterialRefRoot(base.clone()),
				PropagateToDescendants,
				EyeGlowMaterialInstance { owner: owner_a },
			))
			.id();
		let host_b = app
			.world_mut()
			.spawn((MaterialRefRoot(base.clone()), PropagateToDescendants))
			.id();
		let mesh_a = app.world_mut().spawn((Mesh3d(mesh.clone()), ChildOf(host_a))).id();
		let mesh_b = app.world_mut().spawn((Mesh3d(mesh), ChildOf(host_b))).id();
		app.update();

		let handle_a =
			app.world().get::<MeshMaterial3d<FaceShaderMaterial>>(mesh_a).unwrap().0.clone();
		let handle_b =
			app.world().get::<MeshMaterial3d<FaceShaderMaterial>>(mesh_b).unwrap().0.clone();
		assert_ne!(handle_a, handle_b, "instanced glow owner should not share handles");

		{
			let mut materials = app.world_mut().resource_mut::<Assets<FaceShaderMaterial>>();
			let mut shader = materials.get_mut(&handle_a).expect("instanced material");
			set_eye_glow_uniform(&mut shader.params, GOD_BEAM_EYE_GLOW_COLOR, 1.0, 22.0);
		}
		let materials = app.world().resource::<Assets<FaceShaderMaterial>>();
		assert!(eye_glow_intensity(&materials.get(&handle_a).unwrap().params) > 0.9);
		assert!(eye_glow_intensity(&materials.get(&handle_b).unwrap().params) < 1e-4);

		let restamped = base_face_ref().with_scalars([0.5]);
		app.world_mut().entity_mut(host_a).insert(MaterialRefRoot(restamped));
		app.update();

		let handle_a2 =
			app.world().get::<MeshMaterial3d<FaceShaderMaterial>>(mesh_a).unwrap().0.clone();
		assert_ne!(handle_a, handle_a2, "restamp should replace mesh material handle");
		let materials = app.world().resource::<Assets<FaceShaderMaterial>>();
		assert!(
			eye_glow_intensity(&materials.get(&handle_b).unwrap().params) < 1e-4,
			"character B must stay off after A restamps"
		);
		Ok(())
	}
}
