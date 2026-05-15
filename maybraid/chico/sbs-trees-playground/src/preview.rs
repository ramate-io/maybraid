use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_trees::SkippedMeshMaterial;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_stick_components::chico_stick::ChicoStick;
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use chunk::cascade::CascadeChunk;
use render_item::DispatchRenderItem;

/// [`SopesBanyan`] configured for this playground: custom vegetation [`Material`] shaders on sticks vs canopy balls.
pub type PreviewSopesBanyan = SopesBanyan<
	ChicoStickMaterial,
	SkippedMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedMeshMaterial<ChicoLeafMaterial>,
>;

pub type PreviewStick = ChicoStick<ChicoStickMaterial, SkippedMeshMaterial<ChicoStickMaterial>>;
pub type PreviewJointBall =
	ChicoBall<ChicoStickMaterial, SkippedMeshMaterial<ChicoStickMaterial>>;
pub type PreviewLeafBall = ChicoBall<ChicoLeafMaterial, SkippedMeshMaterial<ChicoLeafMaterial>>;

#[derive(Component)]
pub struct SbsPreviewRoot;

#[derive(Resource, Clone)]
pub struct PreviewConfig {
	pub tree: PreviewSopesBanyan,
	pub res_2: u8,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { tree: PreviewSopesBanyan::default(), res_2: 4, transform: Transform::default() }
	}
}

/// Respawns preview whenever [`PreviewConfig`] changes.
pub fn sync_tree_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	root_q: Query<Entity, With<SbsPreviewRoot>>,
	stick_q: Query<Entity, With<PreviewStick>>,
	joint_ball_q: Query<Entity, With<PreviewJointBall>>,
	leaf_ball_q: Query<Entity, With<PreviewLeafBall>>,
	splay_q: Query<Entity, With<PlaneSplay>>,
) {
	if !config.is_changed() {
		return;
	}

	for e in root_q.iter() {
		commands.entity(e).despawn();
	}
	for e in stick_q.iter() {
		commands.entity(e).despawn();
	}
	for e in joint_ball_q.iter() {
		commands.entity(e).despawn();
	}
	for e in leaf_ball_q.iter() {
		commands.entity(e).despawn();
	}
	for e in splay_q.iter() {
		commands.entity(e).despawn();
	}

	commands.spawn((
		SbsPreviewRoot,
		CascadeChunk::unit_center_chunk().with_res_2(config.res_2),
		DispatchRenderItem::new(config.tree.clone()),
		config.transform,
	));
}
