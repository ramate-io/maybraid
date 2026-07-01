use bevy::prelude::*;
use crozon_characters::CharacterPartSlot;

use crate::{
	preview::PreviewAssetTarget,
	skinning::{CharacterPart, NeedsSocketPlacement},
	ui::{CreatorUiState, UiAssetTarget},
};

#[derive(Component, Clone, Copy)]
pub struct PreviewFocusBaseScale(pub Vec3);

pub fn animate_focused_preview_asset(
	mut commands: Commands,
	time: Res<Time>,
	ui_state: Res<CreatorUiState>,
	mut parts: Query<
		(
			Entity,
			&PreviewAssetTarget,
			&CharacterPart,
			&mut Transform,
			Option<&PreviewFocusBaseScale>,
		),
		Without<NeedsSocketPlacement>,
	>,
) {
	let focus = ui_state.focused_target();
	for (entity, target, part, mut transform, base) in &mut parts {
		if !should_pulse(part.slot) {
			if let Some(base) = base {
				transform.scale = base.0;
			}
			continue;
		}
		let base_scale = match base {
			Some(base) => base.0,
			None => {
				let scale = transform.scale;
				commands.entity(entity).try_insert(PreviewFocusBaseScale(scale));
				scale
			}
		};
		let pulse = focus_scale(focus, target.target, time.elapsed_secs());
		transform.scale = base_scale * pulse;
	}
}

fn should_pulse(slot: CharacterPartSlot) -> bool {
	matches!(
		slot,
		CharacterPartSlot::BodyMesh
			| CharacterPartSlot::HeadMesh
			| CharacterPartSlot::Horns
			| CharacterPartSlot::Clothing
	)
}

fn focus_scale(focus: Option<UiAssetTarget>, target: UiAssetTarget, elapsed: f32) -> f32 {
	if focus != Some(target) {
		return 1.0;
	}
	1.0 + elapsed.mul_add(7.0, 0.0).sin().abs() * 0.045
}
