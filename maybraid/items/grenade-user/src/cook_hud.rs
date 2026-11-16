//! Simple remaining-fuse bar while the local player cooks a grenade.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{Inventory, InventoryItem};
use menu_components::{
	gameplay_hud_visibility, hud_glass_fill, hud_glass_stroke, GameplayHud, HUD_GLASS_BORDER,
	HUD_GLASS_RADIUS,
};
use player::Player;

use crate::throw::GrenadeThrow;

const BAR_WIDTH: f32 = 168.0;
const BAR_HEIGHT: f32 = 8.0;

#[derive(Component)]
pub struct CookFuseHud;

#[derive(Component)]
pub(crate) struct CookFuseFill;

pub(crate) fn spawn_cook_fuse_hud(mut commands: Commands) {
	commands
		.spawn((
			Name::new("grenade-cook-hud"),
			CookFuseHud,
			GameplayHud,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Percent(50.0),
				bottom: Val::Px(92.0),
				width: Val::Px(BAR_WIDTH),
				height: Val::Px(BAR_HEIGHT),
				margin: UiRect::left(Val::Px(-BAR_WIDTH * 0.5)),
				border: UiRect::all(Val::Px(HUD_GLASS_BORDER)),
				border_radius: BorderRadius::all(Val::Px(HUD_GLASS_RADIUS)),
				overflow: Overflow::clip(),
				..default()
			},
			hud_glass_fill(),
			hud_glass_stroke(false),
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|bar| {
			bar.spawn((
				CookFuseFill,
				Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
				BackgroundColor(Color::srgb(0.12, 0.92, 1.0)),
				Pickable::IGNORE,
			));
		});
}

pub(crate) fn sync_cook_fuse_hud(
	players: Query<(&GrenadeThrow, Option<&InventoryUser>), With<Player>>,
	bags: Query<&Inventory>,
	mut roots: Query<(&mut Visibility, &mut BorderColor), With<CookFuseHud>>,
	mut fills: Query<(&mut Node, &mut BackgroundColor), With<CookFuseFill>>,
) {
	let cooking = players.iter().find_map(|(throw, carrier)| {
		let remaining = throw.live_fuse()?;
		let total = carrier
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
			.map(|stats| stats.fuse)
			.unwrap_or(remaining.max(1e-3));
		Some((remaining, total.max(1e-3)))
	});
	let shown = cooking.is_some();
	let visible = gameplay_hud_visibility(shown);
	let stroke = hud_glass_stroke(shown);
	for (mut visibility, mut border) in &mut roots {
		if *visibility != visible {
			*visibility = visible;
		}
		if *border != stroke {
			*border = stroke;
		}
	}
	let Some((remaining, total)) = cooking else {
		return;
	};
	let fraction = (remaining / total).clamp(0.0, 1.0);
	let fill = cook_fill_color(fraction);
	for (mut node, mut color) in &mut fills {
		node.width = Val::Px(BAR_WIDTH * fraction);
		if color.0 != fill {
			color.0 = fill;
		}
	}
}

fn cook_fill_color(fraction: f32) -> Color {
	Color::srgb(1.0 - fraction * 0.88, 0.18 + fraction * 0.74, 0.16 + fraction * 0.84)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fill_goes_red_as_the_fuse_runs_out() {
		let full = cook_fill_color(1.0);
		let empty = cook_fill_color(0.0);
		let Color::Srgba(full) = full else {
			panic!("expected srgb");
		};
		let Color::Srgba(empty) = empty else {
			panic!("expected srgb");
		};
		assert!(empty.red > full.red);
		assert!(full.green > empty.green);
	}
}
