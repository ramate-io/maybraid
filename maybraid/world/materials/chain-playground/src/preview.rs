//! Two independently parameterized placeholder chains.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use chain_shaders::{
	apply_chain_hit, mythical_chain_ref_for, mythical_chain_ref_with, placeholder_chain_mesh,
	ChainHalo, ChainMaterial, MythicalChainScalars, MythicalChainState, OwnedChainHandle,
	OwnedChainMaterial, PlaceholderChainSpec, DEFAULT_GLOW_INTENSITY, DEFAULT_LINK_COUNT,
	DEFAULT_SEED,
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
	pub style: PreviewStyle,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreviewStyle {
	#[default]
	Sharp,
	Retro,
	Antediluvian,
}

impl PreviewStyle {
	pub fn label(self) -> &'static str {
		match self {
			Self::Sharp => "sharp",
			Self::Retro => "retro",
			Self::Antediluvian => "antediluvian",
		}
	}

	pub fn from_label(label: &str) -> Option<Self> {
		match label {
			"sharp" => Some(Self::Sharp),
			"retro" => Some(Self::Retro),
			"antediluvian" => Some(Self::Antediluvian),
			_ => None,
		}
	}

	pub fn resolution(self) -> (f32, f32) {
		match self {
			Self::Sharp => (1280.0, 720.0),
			Self::Retro => (540.0, 304.0),
			Self::Antediluvian => (400.0, 225.0),
		}
	}
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
			style: PreviewStyle::Sharp,
		}
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		format!(
			"preview: left {} · right {} · {} · {} · deform {} · sweep {}",
			recipe_label(&self.left),
			recipe_label(&self.right),
			self.lighting.label(),
			self.style.label(),
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
		let spec = PlaceholderChainSpec::default().bent(if config.deform { 0.22 } else { 0.0 });
		let mesh = meshes.add(placeholder_chain_mesh(spec));
		spawn_chain(
			&mut commands,
			mesh.clone(),
			config.left.clone(),
			Vec3::new(-0.55, 0.12, 0.0),
			"left",
			true,
			true,
		);
		spawn_chain(
			&mut commands,
			mesh,
			config.right.clone(),
			Vec3::new(0.55, 0.12, 0.0),
			"right",
			false,
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
	owned: bool,
) {
	let mut entity = commands.spawn((
		PreviewRoot,
		PreviewSide(left),
		Name::new(format!("chain-{name}")),
		Transform::from_translation(translation),
		Visibility::default(),
		MaterialRefRoot(material),
		Mesh3d(mesh),
		NotShadowCaster,
		ChainHalo,
	));
	if owned {
		entity.insert(OwnedChainMaterial);
	}
}

#[derive(Resource, Default)]
pub struct PendingChainHit {
	pub link: Option<u32>,
	pub amount: f32,
}

pub fn apply_pending_hit(
	time: Res<Time>,
	mut pending: ResMut<PendingChainHit>,
	mut materials: ResMut<Assets<ChainMaterial>>,
	owned: Query<&OwnedChainHandle, With<PreviewRoot>>,
) {
	let Some(link) = pending.link.take() else {
		return;
	};
	let amount = if pending.amount > 1e-4 { pending.amount } else { 0.72 };
	for handle in &owned {
		if let Some(mut material) = materials.get_mut(&handle.0) {
			apply_chain_hit(&mut material, link, amount, DEFAULT_LINK_COUNT, time.elapsed_secs());
		}
	}
}

pub fn sync_window_style(config: Res<PreviewConfig>, mut windows: Query<&mut Window>) {
	if !config.is_changed() && !config.is_added() {
		return;
	}
	let Ok(mut window) = windows.single_mut() else {
		return;
	};
	let (w, h) = config.style.resolution();
	window.resolution.set(w, h);
}
