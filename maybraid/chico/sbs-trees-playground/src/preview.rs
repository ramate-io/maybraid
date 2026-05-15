use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBallStd;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_trees::sopes_banyan::SopesBanyanStd;
use chico_stick_components::chico_stick::ChicoStickStd;
use chunk::cascade::CascadeChunk;
use render_item::DispatchRenderItem;

#[derive(Component)]
pub struct SbsPreviewRoot;

#[derive(Resource, Clone)]
pub struct PreviewConfig {
	pub tree: SopesBanyanStd,
	pub res_2: u8,
	pub transform: Transform,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { tree: SopesBanyanStd::default(), res_2: 4, transform: Transform::default() }
	}
}

/// Respawns preview whenever [`PreviewConfig`] changes.
pub fn sync_tree_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	root_q: Query<Entity, With<SbsPreviewRoot>>,
	stick_q: Query<Entity, With<ChicoStickStd>>,
	ball_q: Query<Entity, With<ChicoBallStd>>,
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
	for e in ball_q.iter() {
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
