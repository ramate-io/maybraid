//! Per-character eye warning glow on the `face_eye` material path.
//!
//! [`EyeGlowControl`] lives on [`CharacterRoot`]. Each eye part host forks its
//! [`FaceShaderMaterial`] handle once so glow does not leak across characters
//! that share a material-ref cache entry.

use bevy::prelude::*;
use character_shaders::{
	set_eye_glow_uniform, FaceShaderMaterial, GOD_BEAM_EYE_GLOW_COLOR,
	GOD_BEAM_EYE_GLOW_EMISSIVE_GAIN, GOD_BEAM_EYE_GLOW_INTENSITY,
};
use material_ref::MaterialRefSystems;

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

/// Marks an eye part whose mesh materials were forked for per-character glow.
#[derive(Component, Debug, Default)]
pub struct EyeGlowMaterialForked;

pub struct EyeGlowPlugin;

type EyeGlowPrepareQuery<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static CharacterMembers),
	(With<CharacterRoot>, With<EyeGlowControl>),
>;

impl Plugin for EyeGlowPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				prepare_eye_glow_materials.after(MaterialRefSystems::Fulfill),
				update_eye_glow.after(prepare_eye_glow_materials),
			),
		);
	}
}

fn prepare_eye_glow_materials(
	mut commands: Commands,
	roots: EyeGlowPrepareQuery<'_, '_>,
	parts: Query<&PartNode>,
	children: Query<&Children>,
	meshes: Query<(Entity, &MeshMaterial3d<FaceShaderMaterial>)>,
	forked: Query<(), With<EyeGlowMaterialForked>>,
	mut face_materials: ResMut<Assets<FaceShaderMaterial>>,
) {
	for (_root, members) in &roots {
		for member in members.iter() {
			let Ok(part) = parts.get(member) else {
				continue;
			};
			if !is_eye_slot(part.slot) {
				continue;
			}
			if forked.contains(member) {
				continue;
			}
			fork_eye_part_materials(
				&mut commands,
				member,
				&children,
				&meshes,
				&mut face_materials,
			);
		}
	}
}

fn fork_eye_part_materials(
	commands: &mut Commands,
	part_host: Entity,
	children: &Query<&Children>,
	meshes: &Query<(Entity, &MeshMaterial3d<FaceShaderMaterial>)>,
	face_materials: &mut Assets<FaceShaderMaterial>,
) {
	let mut stack: Vec<Entity> =
		children.get(part_host).map(|c| c.iter().collect()).unwrap_or_default();
	let mut forked_any = false;
	while let Some(entity) = stack.pop() {
		if let Ok((mesh_entity, material)) = meshes.get(entity) {
			if let Some(clone) = face_materials.get(&material.0).cloned() {
				let handle = face_materials.add(clone);
				commands.entity(mesh_entity).insert(MeshMaterial3d(handle));
				forked_any = true;
			}
		}
		if let Ok(kids) = children.get(entity) {
			stack.extend(kids.iter());
		}
	}
	if forked_any {
		commands.entity(part_host).insert(EyeGlowMaterialForked);
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
}
