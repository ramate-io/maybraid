use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_ball_components::tuft::ChicoTuft;
use chico_sbs_trees::liams_conifer::LiamsConifer;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use chico_stick_components::chico_stick::ChicoStick;
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use chunk::cascade::CascadeChunk;
use render_item::DispatchRenderItem;

/// [`SopesBanyan`] configured for this playground.
pub type PreviewSopesBanyan = SopesBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`LiamsConifer`] configured for this playground.
pub type PreviewLiamsConifer = LiamsConifer<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

pub type PreviewStick = ChicoStick<ChicoStickMaterial, SkippedStickMeshMaterial<ChicoStickMaterial>>;
pub type PreviewJointBall =
	ChicoBall<ChicoStickMaterial, SkippedStickMeshMaterial<ChicoStickMaterial>>;
pub type PreviewLeafBall = ChicoBall<ChicoLeafMaterial, SkippedLeafMeshMaterial<ChicoLeafMaterial>>;
pub type PreviewPlaneSplay =
	PlaneSplay<ChicoLeafMaterial, SkippedLeafMeshMaterial<ChicoLeafMaterial>>;
pub type PreviewTuft = ChicoTuft<ChicoLeafMaterial, SkippedLeafMeshMaterial<ChicoLeafMaterial>>;

#[derive(Clone)]
pub enum PreviewTree {
	SopesBanyan(PreviewSopesBanyan),
	LiamsConifer(PreviewLiamsConifer),
}

impl PreviewTree {
	pub fn label(&self) -> &'static str {
		match self {
			Self::SopesBanyan(_) => "SopesBanyan",
			Self::LiamsConifer(_) => "LiamsConifer",
		}
	}
}

#[derive(Component)]
pub struct SbsPreviewRoot;

#[derive(Resource, Clone)]
pub struct PreviewConfig {
	pub tree: PreviewTree,
	pub res_2: u8,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self {
			tree: PreviewTree::LiamsConifer(PreviewLiamsConifer::default()),
			res_2: 4,
			transform: Transform::default(),
		}
	}
}

fn preview_sync_key(config: &PreviewConfig) -> String {
	format!(
		"{}|res_2={}|t={:?}|s={:?}",
		config.tree.label(),
		config.res_2,
		config.transform.translation,
		config.transform.scale
	)
}

/// Respawns preview whenever [`PreviewConfig`] changes.
pub fn sync_tree_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut synced: Local<Option<String>>,
	root_q: Query<Entity, With<SbsPreviewRoot>>,
	stick_q: Query<Entity, With<PreviewStick>>,
	joint_ball_q: Query<Entity, With<PreviewJointBall>>,
	leaf_ball_q: Query<Entity, With<PreviewLeafBall>>,
	splay_q: Query<Entity, With<PreviewPlaneSplay>>,
	tuft_q: Query<Entity, With<PreviewTuft>>,
) {
	let key = preview_sync_key(&config);
	if synced.as_deref() == Some(&key) {
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
	for e in tuft_q.iter() {
		commands.entity(e).despawn();
	}

	let bundle = (
		SbsPreviewRoot,
		CascadeChunk::unit_center_chunk().with_res_2(config.res_2),
		config.transform,
	);

	match &config.tree {
		PreviewTree::SopesBanyan(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree.clone())));
		}
		PreviewTree::LiamsConifer(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree.clone())));
		}
	}
	*synced = Some(key);
}
