//! Two independently parameterized placeholder chains.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use chain_shaders::{
	mythical_chain_ref_for, mythical_chain_ref_with, placeholder_chain_mesh, MythicalChainScalars,
	MythicalChainState, PlaceholderChainSpec, DEFAULT_GLOW_INTENSITY, DEFAULT_SEED,
};
use material_ref::{MaterialRef, MaterialRefRoot};

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightingMode {
	#[default]
	Day,
	Night,
}

impl LightingMode {
	pub fn label(self) -> &'static str {
		match self {
			Self::Day => "day",
			Self::Night => "night",
		}
	}

	pub fn from_label(label: &str) -> Option<Self> {
		match label {
			"day" => Some(Self::Day),
			"night" => Some(Self::Night),
			_ => None,
		}
	}
}

#[derive(Component)]
pub struct PreviewRoot;

#[derive(Component, Clone, Copy)]
pub struct PreviewSide(pub bool);

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct PreviewConfig {
	pub left: MaterialRef,
	pub right: MaterialRef,
	pub lighting: LightingMode,
	pub deform: bool,
	pub sweep: bool,
	pub glow_intensity: f32,
	pub seed: i32,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self {
			left: mythical_chain_ref_for(MythicalChainState::Intact),
			right: mythical_chain_ref_with(
				MythicalChainScalars::for_state(MythicalChainState::Damaged),
				DEFAULT_SEED + 18,
			),
			lighting: LightingMode::Day,
			deform: true,
			sweep: false,
			glow_intensity: DEFAULT_GLOW_INTENSITY,
			seed: DEFAULT_SEED,
		}
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		format!(
			"preview: left {} · right {} · {} · deform {} · sweep {}",
			recipe_label(&self.left),
			recipe_label(&self.right),
			self.lighting.label(),
			if self.deform { "on" } else { "off" },
			if self.sweep { "on" } else { "off" },
		)
	}

	pub fn restamp_glow_and_seed(&mut self) {
		self.left = restamp(&self.left, self.glow_intensity, self.seed);
		self.right = restamp(&self.right, self.glow_intensity, self.seed + 18);
	}

	pub fn set_left_state(&mut self, state: MythicalChainState) {
		let glow = self.glow_intensity;
		let seed = self.seed;
		apply_state(&mut self.left, state, glow, seed);
	}

	pub fn set_right_state(&mut self, state: MythicalChainState) {
		let glow = self.glow_intensity;
		let seed = self.seed + 18;
		apply_state(&mut self.right, state, glow, seed);
	}
}

fn recipe_label(material: &MaterialRef) -> String {
	let scalars = MythicalChainScalars::from_slice(material.scalar_values());
	if let Some(state) = state_from_scalars(&scalars) {
		return state.label().into();
	}
	format!("intact {:.2} fade {:.2}", scalars.intact, scalars.fade)
}

fn state_from_scalars(scalars: &MythicalChainScalars) -> Option<MythicalChainState> {
	[
		MythicalChainState::Intact,
		MythicalChainState::Damaged,
		MythicalChainState::NearBreaking,
		MythicalChainState::Released,
	]
	.into_iter()
	.find(|state| {
		(scalars.intact - state.intact()).abs() < 1e-3 && (scalars.fade - state.fade()).abs() < 1e-3
	})
}

fn restamp(material: &MaterialRef, glow: f32, seed: i32) -> MaterialRef {
	let scalars =
		MythicalChainScalars::from_slice(material.scalar_values()).with_glow_intensity(glow);
	mythical_chain_ref_with(scalars, seed)
}

pub fn apply_state(material: &mut MaterialRef, state: MythicalChainState, glow: f32, seed: i32) {
	*material = mythical_chain_ref_with(
		MythicalChainScalars::for_state(state).with_glow_intensity(glow),
		seed,
	);
}

pub fn apply_intact(material: &mut MaterialRef, intact: f32, glow: f32, seed: i32) {
	let fade = if intact < 0.08 {
		1.0
	} else if intact < 0.28 {
		0.15
	} else {
		0.0
	};
	*material = mythical_chain_ref_with(
		MythicalChainScalars::for_state(MythicalChainState::Intact)
			.with_intact(intact)
			.with_fade(fade)
			.with_glow_intensity(glow),
		seed,
	);
}

pub fn sync_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut meshes: ResMut<Assets<Mesh>>,
	roots: Query<(Entity, &PreviewSide, &MaterialRefRoot), With<PreviewRoot>>,
	mut last_deform: Local<Option<bool>>,
) {
	let deform_changed = last_deform.as_ref() != Some(&config.deform);
	let missing = roots.iter().count() < 2;
	if deform_changed || missing {
		for (entity, _, _) in &roots {
			commands.entity(entity).try_despawn();
		}
		let spec = PlaceholderChainSpec::default().bent(if config.deform { 0.28 } else { 0.0 });
		let mesh = meshes.add(placeholder_chain_mesh(spec));
		spawn_chain(
			&mut commands,
			mesh.clone(),
			config.left.clone(),
			Vec3::new(-0.72, 0.18, 0.0),
			"left",
			true,
		);
		spawn_chain(
			&mut commands,
			mesh,
			config.right.clone(),
			Vec3::new(0.72, 0.18, 0.0),
			"right",
			false,
		);
		*last_deform = Some(config.deform);
		return;
	}

	for (entity, side, current) in &roots {
		let wanted = if side.0 { &config.left } else { &config.right };
		if current.0 != *wanted {
			commands.entity(entity).insert(MaterialRefRoot(wanted.clone()));
		}
	}
}

pub fn tick_sweep(time: Res<Time>, mut config: ResMut<PreviewConfig>) {
	if !config.sweep {
		return;
	}
	let t = (time.elapsed_secs() * 0.22).rem_euclid(1.0);
	let intact = 1.0 - t;
	let glow = config.glow_intensity;
	let seed = config.seed;
	apply_intact(&mut config.left, intact, glow, seed);
}

fn spawn_chain(
	commands: &mut Commands,
	mesh: Handle<Mesh>,
	material: MaterialRef,
	translation: Vec3,
	name: &str,
	left: bool,
) {
	commands.spawn((
		PreviewRoot,
		PreviewSide(left),
		Name::new(format!("chain-{name}")),
		Transform::from_translation(translation),
		Visibility::default(),
		MaterialRefRoot(material),
		Mesh3d(mesh),
		NotShadowCaster,
	));
}
