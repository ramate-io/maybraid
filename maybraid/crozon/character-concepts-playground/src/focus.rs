use bevy::prelude::*;

use crate::{
	preview::PreviewAssetTarget,
	skinning::NeedsSocketPlacement,
	ui::{CreatorUiState, UiAssetTarget},
};

#[derive(Component, Clone, Copy)]
pub struct PreviewFocusBaseScale(pub Vec3);

pub fn animate_focused_preview_asset(
	mut commands: Commands,
	time: Res<Time>,
	ui_state: Res<CreatorUiState>,
	mut parts: Query<
		(Entity, &PreviewAssetTarget, &mut Transform, Option<&PreviewFocusBaseScale>),
		Without<NeedsSocketPlacement>,
	>,
) {
	let focus = ui_state.focused_target();
	for (entity, target, mut transform, base) in &mut parts {
		let base_scale = match base {
			Some(base) => base.0,
			None => {
				let scale = transform.scale;
				commands.entity(entity).insert(PreviewFocusBaseScale(scale));
				scale
			}
		};
		let pulse = focus_scale(focus, target.target, time.elapsed_secs());
		transform.scale = base_scale * pulse;
	}
}

fn focus_scale(focus: Option<UiAssetTarget>, target: UiAssetTarget, elapsed: f32) -> f32 {
	if focus != Some(target) {
		return 1.0;
	}
	1.0 + elapsed.mul_add(7.0, 0.0).sin().abs() * 0.045
}
