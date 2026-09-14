//! Debug HUD pins / gizmos for presented furniture kits.

use bevy::prelude::*;
use bevy::text::FontSize;
use furniture_assemblies::FurnitureCell;
use lod::LodLevelRootStreamed;
use richmond_building_components::FurnitureGeometry;

use crate::ui::{pin_node, place_pin, project_mob_pin, MobDebugHud};

const HUD_PIN_COUNT: usize = 12;
const HUD_PIN_WORLD_HEIGHT: f32 = 2.2;

#[derive(Component)]
pub(crate) struct FurnitureDebugPin {
	host: Entity,
	slot: usize,
}

#[derive(Bundle)]
struct FurnitureDebugPinBundle {
	name: Name,
	pin: FurnitureDebugPin,
	node: Node,
	background: BackgroundColor,
	text: Text,
	font: TextFont,
	color: TextColor,
	pickable: Pickable,
	visibility: Visibility,
}

struct RankedItem {
	host: Entity,
	slot: usize,
	geometry: FurnitureGeometry,
	at: Vec3,
	distance: f32,
	streamed: bool,
}

pub(crate) fn sync_furniture_debug_pins(
	mut commands: Commands,
	camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	hud: Query<Entity, With<MobDebugHud>>,
	cells: Query<(Entity, &FurnitureCell, Has<LodLevelRootStreamed>)>,
	mut pins: Query<(
		Entity,
		&FurnitureDebugPin,
		&mut Node,
		&mut BackgroundColor,
		&mut Text,
		&mut TextColor,
		&mut Visibility,
	)>,
) {
	let Ok(hud) = hud.single() else {
		return;
	};
	let Ok((camera, camera_transform)) = camera.single() else {
		for (_, _, _, _, _, _, mut visibility) in &mut pins {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	let wanted: Vec<_> =
		ranked_items(camera_transform, &cells).into_iter().take(HUD_PIN_COUNT).collect();
	let mut assigned = Vec::new();
	for (pin_entity, pin, mut node, mut background, mut text, mut text_color, mut visibility) in
		&mut pins
	{
		let Some(item) = wanted.iter().find(|item| item.host == pin.host && item.slot == pin.slot)
		else {
			commands.entity(pin_entity).despawn();
			continue;
		};
		let Some((screen, on_screen)) =
			project_mob_pin(camera, camera_transform, item.at + Vec3::Y * HUD_PIN_WORLD_HEIGHT)
		else {
			*visibility = Visibility::Hidden;
			continue;
		};
		place_pin(&mut node, screen);
		background.0 = item_color(item.geometry).with_alpha(pin_alpha(item.streamed, on_screen));
		text.0 = item_label(item);
		text_color.0 = if item.streamed { Color::WHITE } else { Color::srgb(0.85, 0.85, 0.88) };
		*visibility = Visibility::Visible;
		assigned.push((item.host, item.slot));
	}
	for item in wanted {
		if assigned.contains(&(item.host, item.slot)) {
			continue;
		}
		let Some((screen, on_screen)) =
			project_mob_pin(camera, camera_transform, item.at + Vec3::Y * HUD_PIN_WORLD_HEIGHT)
		else {
			continue;
		};
		commands.entity(hud).with_children(|root| {
			root.spawn(FurnitureDebugPinBundle {
				name: Name::new("furniture-debug-pin"),
				pin: FurnitureDebugPin { host: item.host, slot: item.slot },
				node: pin_node(screen),
				background: BackgroundColor(
					item_color(item.geometry).with_alpha(pin_alpha(item.streamed, on_screen)),
				),
				text: Text::new(item_label(&item)),
				font: TextFont { font_size: FontSize::Px(12.0), ..default() },
				color: TextColor(if item.streamed {
					Color::WHITE
				} else {
					Color::srgb(0.85, 0.85, 0.88)
				}),
				pickable: Pickable::IGNORE,
				visibility: Visibility::Visible,
			});
		});
	}
}

pub(crate) fn draw_furniture_debug_gizmos(
	mut gizmos: Gizmos,
	cells: Query<(&FurnitureCell, Has<LodLevelRootStreamed>)>,
) {
	for (cell, streamed) in &cells {
		for slot in &cell.slots {
			let at = slot.placement.translation;
			let color = item_color(slot.geometry).with_alpha(if streamed { 0.95 } else { 0.45 });
			gizmos.line(at, at + Vec3::Y * 1.8, color);
			gizmos.sphere(Isometry3d::from_translation(at + Vec3::Y * 1.8), 0.18, color);
		}
	}
}

fn ranked_items(
	camera: &GlobalTransform,
	cells: &Query<(Entity, &FurnitureCell, Has<LodLevelRootStreamed>)>,
) -> Vec<RankedItem> {
	let origin = camera.translation();
	let mut ranked = Vec::new();
	for (entity, cell, streamed) in cells {
		for (slot, node) in cell.slots.iter().enumerate() {
			let at = node.placement.translation;
			ranked.push(RankedItem {
				host: entity,
				slot,
				geometry: node.geometry,
				at,
				distance: at.xz().distance(origin.xz()),
				streamed,
			});
		}
	}
	ranked.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
	ranked
}

fn item_label(item: &RankedItem) -> String {
	let name = format!("{:?}", item.geometry);
	if item.streamed {
		format!("{name} {:.0}m", item.distance)
	} else {
		format!("{name} … {:.0}m", item.distance)
	}
}

fn pin_alpha(streamed: bool, on_screen: bool) -> f32 {
	match (streamed, on_screen) {
		(true, true) => 0.78,
		(true, false) => 0.94,
		(false, true) => 0.42,
		(false, false) => 0.58,
	}
}

fn item_color(geometry: FurnitureGeometry) -> Color {
	geometry.wireframe_color()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn streamed_label_drops_the_ellipsis() {
		let streamed = RankedItem {
			host: Entity::PLACEHOLDER,
			slot: 0,
			geometry: FurnitureGeometry::Chair,
			at: Vec3::ZERO,
			distance: 12.0,
			streamed: true,
		};
		assert_eq!(item_label(&streamed), "Chair 12m");
		let pending = RankedItem { streamed: false, ..streamed };
		assert_eq!(item_label(&pending), "Chair … 12m");
	}
}
