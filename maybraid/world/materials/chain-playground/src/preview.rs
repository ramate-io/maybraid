//! Independently parameterized placeholder chains plus a continuous tube.

use bevy::light::NotShadowCaster;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use chain_shaders::{
	apply_chain_hit, continuous_tube_mesh, mythical_chain_ref_for, mythical_chain_ref_with,
	placeholder_chain_mesh, spawn_skinned_tube, ChainFlowStyle, ChainHalo, ChainMaterial,
	ContinuousTubeSpec, MythicalChainScalars, MythicalChainState, OwnedChainHandle,
	OwnedChainMaterial, PlaceholderChainSpec, DEFAULT_GLOW_INTENSITY, DEFAULT_LINK_COUNT,
	DEFAULT_SEED,
};
use material_ref::{MaterialRef, MaterialRefRoot};
use maybraid_world::{RequestVisualStyle, VisualStyle};

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

#[derive(Component)]
pub struct PreviewTube;

#[derive(Component)]
pub struct PreviewProp;

#[derive(Component)]
pub struct PreviewSkinnedJoint(pub u32);

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
	pub halo: bool,
	pub emission: bool,
	pub flow: ChainFlowStyle,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PreviewStyle {
	#[default]
	Native,
	Sharp,
	Retro,
	Antediluvian,
}

impl PreviewStyle {
	pub fn label(self) -> &'static str {
		match self {
			Self::Native => "native",
			Self::Sharp => "sharp",
			Self::Retro => "retro",
			Self::Antediluvian => "antediluvian",
		}
	}

	pub fn from_label(label: &str) -> Option<Self> {
		match label {
			"native" => Some(Self::Native),
			"sharp" => Some(Self::Sharp),
			"retro" => Some(Self::Retro),
			"antediluvian" => Some(Self::Antediluvian),
			_ => None,
		}
	}

	pub fn visual_style(self) -> VisualStyle {
		match self {
			Self::Native => VisualStyle::Native,
			Self::Sharp => VisualStyle::Sharp,
			Self::Retro => VisualStyle::Retro,
			Self::Antediluvian => VisualStyle::Antediluvian,
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
			style: PreviewStyle::Native,
			halo: true,
			emission: true,
			flow: ChainFlowStyle::default(),
		}
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		format!(
			"preview: left {} · right {} · {} · {} · deform {} · sweep {} · halo {} · emit {}",
			recipe_label(&self.left),
			recipe_label(&self.right),
			self.lighting.label(),
			self.style.label(),
			if self.deform { "on" } else { "off" },
			if self.sweep { "on" } else { "off" },
			if self.halo { "on" } else { "off" },
			if self.emission { "on" } else { "off" },
		)
	}

	pub fn restamp_glow_and_seed(&mut self) {
		self.left = restamp(&self.left, self.glow_intensity, self.seed, self.flow, self.emission);
		self.right =
			restamp(&self.right, self.glow_intensity, self.seed + 18, self.flow, self.emission);
	}

	pub fn set_left_state(&mut self, state: MythicalChainState) {
		apply_state(
			&mut self.left,
			state,
			self.glow_intensity,
			self.seed,
			self.flow,
			self.emission,
		);
	}

	pub fn set_right_state(&mut self, state: MythicalChainState) {
		apply_state(
			&mut self.right,
			state,
			self.glow_intensity,
			self.seed + 18,
			self.flow,
			self.emission,
		);
	}

	pub fn set_left_damage(&mut self, damage: f32) {
		self.left = mythical_chain_ref_with(
			MythicalChainScalars::for_state(MythicalChainState::Intact)
				.with_damage(damage)
				.with_intact(1.0 - damage)
				.with_glow_intensity(if self.emission { self.glow_intensity } else { 0.0 })
				.with_flow(self.flow),
			self.seed,
		);
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

fn restamp(
	material: &MaterialRef,
	glow: f32,
	seed: i32,
	flow: ChainFlowStyle,
	emission: bool,
) -> MaterialRef {
	let scalars = MythicalChainScalars::from_slice(material.scalar_values())
		.with_glow_intensity(if emission { glow } else { 0.0 })
		.with_flow(flow);
	mythical_chain_ref_with(scalars, seed)
}

pub fn apply_state(
	material: &mut MaterialRef,
	state: MythicalChainState,
	glow: f32,
	seed: i32,
	flow: ChainFlowStyle,
	emission: bool,
) {
	*material = mythical_chain_ref_with(
		MythicalChainScalars::for_state(state)
			.with_glow_intensity(if emission { glow } else { 0.0 })
			.with_flow(flow),
		seed,
	);
}

pub fn apply_intact(
	material: &mut MaterialRef,
	intact: f32,
	glow: f32,
	seed: i32,
	flow: ChainFlowStyle,
	emission: bool,
) {
	let fade = if intact < 0.02 { 1.0 } else { 0.0 };
	*material = mythical_chain_ref_with(
		MythicalChainScalars::for_state(MythicalChainState::Intact)
			.with_intact(intact)
			.with_fade(fade)
			.with_glow_intensity(if emission { glow } else { 0.0 })
			.with_flow(flow),
		seed,
	);
}

#[allow(clippy::too_many_arguments)]
pub fn sync_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
	mut standards: ResMut<Assets<StandardMaterial>>,
	roots: Query<(Entity, &PreviewSide, &MaterialRefRoot), With<PreviewRoot>>,
	tubes: Query<Entity, With<PreviewTube>>,
	props: Query<Entity, With<PreviewProp>>,
	mut last_deform: Local<Option<bool>>,
) {
	let deform_changed = last_deform.as_ref() != Some(&config.deform);
	let missing = roots.iter().count() < 2 || tubes.iter().count() < 2 || props.iter().count() < 1;
	if deform_changed || missing {
		for (entity, _, _) in &roots {
			commands.entity(entity).try_despawn();
		}
		for entity in &tubes {
			commands.entity(entity).try_despawn();
		}
		for entity in &props {
			commands.entity(entity).try_despawn();
		}
		let spec = PlaceholderChainSpec::default().bent(if config.deform { 0.22 } else { 0.0 });
		let mesh = meshes.add(placeholder_chain_mesh(spec));
		spawn_chain(
			&mut commands,
			mesh.clone(),
			config.left.clone(),
			Vec3::new(-0.70, 0.12, 0.0),
			"left",
			true,
			true,
			config.halo,
		);
		spawn_chain(
			&mut commands,
			mesh,
			config.right.clone(),
			Vec3::new(0.20, 0.12, 0.0),
			"right",
			false,
			false,
			config.halo,
		);
		let tube = ContinuousTubeSpec::default().bent(if config.deform { 0.28 } else { 0.0 });
		let tube_mesh = meshes.add(continuous_tube_mesh(tube));
		spawn_tube(
			&mut commands,
			tube_mesh,
			config.left.clone(),
			Vec3::new(1.05, 0.12, 0.15),
			"tube",
			config.halo,
		);
		let (skinned, joints) = spawn_skinned_tube(
			&mut commands,
			&mut meshes,
			&mut bindposes,
			tube,
			Transform::from_xyz(1.55, 0.12, -0.35),
			"skinned-tube",
		);
		commands.entity(skinned).insert((
			PreviewTube,
			MaterialRefRoot(config.left.clone()),
			NotShadowCaster,
		));
		if config.halo {
			commands.entity(skinned).insert(ChainHalo);
		}
		for (i, joint) in joints.into_iter().enumerate() {
			commands.entity(joint).insert(PreviewSkinnedJoint(i as u32));
		}
		let cube = meshes.add(Cuboid::new(0.28, 0.28, 0.28));
		let wood = standards.add(StandardMaterial {
			base_color: Color::srgb(0.55, 0.38, 0.22),
			perceptual_roughness: 0.72,
			metallic: 0.04,
			..default()
		});
		commands.spawn((
			PreviewProp,
			Name::new("world-cube"),
			Mesh3d(cube),
			MeshMaterial3d(wood),
			Transform::from_xyz(-1.25, 0.22, 0.35),
		));
		*last_deform = Some(config.deform);
		return;
	}

	for (entity, side, current) in &roots {
		let wanted = if side.0 { &config.left } else { &config.right };
		if current.0 != *wanted {
			commands.entity(entity).insert(MaterialRefRoot(wanted.clone()));
		}
		if config.halo {
			commands.entity(entity).insert(ChainHalo);
		} else {
			commands.entity(entity).remove::<ChainHalo>();
		}
	}
	for entity in &tubes {
		if config.halo {
			commands.entity(entity).insert(ChainHalo);
		} else {
			commands.entity(entity).remove::<ChainHalo>();
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
	let flow = config.flow;
	let emission = config.emission;
	apply_intact(&mut config.left, intact, glow, seed, flow, emission);
}

pub fn pose_skinned_tube(
	time: Res<Time>,
	mut joints: Query<(&PreviewSkinnedJoint, &mut Transform)>,
) {
	let t = time.elapsed_secs();
	for (joint, mut transform) in &mut joints {
		let wave = (t * 0.85 + joint.0 as f32 * 0.55).sin() * 0.18;
		transform.rotation = Quat::from_rotation_z(wave);
	}
}

#[allow(clippy::too_many_arguments)]
fn spawn_chain(
	commands: &mut Commands,
	mesh: Handle<Mesh>,
	material: MaterialRef,
	translation: Vec3,
	name: &str,
	left: bool,
	owned: bool,
	halo: bool,
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
	));
	if halo {
		entity.insert(ChainHalo);
	}
	if owned {
		entity.insert(OwnedChainMaterial);
	}
}

fn spawn_tube(
	commands: &mut Commands,
	mesh: Handle<Mesh>,
	material: MaterialRef,
	translation: Vec3,
	name: &str,
	halo: bool,
) {
	let mut entity = commands.spawn((
		PreviewTube,
		Name::new(format!("tube-{name}")),
		Transform::from_translation(translation),
		Visibility::default(),
		MaterialRefRoot(material),
		Mesh3d(mesh),
		NotShadowCaster,
	));
	if halo {
		entity.insert(ChainHalo);
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
	let amount = if pending.amount > 1e-4 { pending.amount } else { 0.25 };
	for handle in &owned {
		if let Some(mut material) = materials.get_mut(&handle.0) {
			apply_chain_hit(&mut material, link, amount, DEFAULT_LINK_COUNT, time.elapsed_secs());
		}
	}
}

pub fn sync_visual_style(
	config: Res<PreviewConfig>,
	mut requests: MessageWriter<RequestVisualStyle>,
	mut last: Local<Option<PreviewStyle>>,
) {
	if last.as_ref() == Some(&config.style) && !config.is_added() {
		return;
	}
	*last = Some(config.style);
	requests.write(RequestVisualStyle(config.style.visual_style()));
}
