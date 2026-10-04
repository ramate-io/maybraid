//! Use activation, windup, release marker, and follow-through.

use avian3d::prelude::{LinearVelocity, SpatialQuery};
use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use firearm_user::WeaponSwap;
use grenades::{spawn_thrown_grenade, GrenadeEffect};
use maybraid_character_controller::CharacterIntent;
use player::{Player, PlayerLook};

use crate::hold::HeldGrenade;
use crate::HeldByGrenade;

/// 1:1 user → held visual. Relationship rows are immutable; throw clocks live on [`GrenadeThrow`].
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = HeldByGrenade)]
pub struct GrenadeUser {
	#[relationship]
	pub held: Entity,
	pub settings: GrenadeUserSettings,
}

impl GrenadeUser {
	pub fn holding(held: Entity) -> Self {
		Self { held, settings: GrenadeUserSettings::default() }
	}
}

/// Mutable throw / latch state. Kept off [`GrenadeUser`] because relationships are immutable.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GrenadeThrow {
	pub phase: GrenadePhase,
	/// True while use stays held after a throw so recharge cannot auto-fire.
	pub use_latched: bool,
}

impl GrenadeThrow {
	pub fn busy(&self) -> bool {
		!matches!(self.phase, GrenadePhase::Ready)
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrenadeUserSettings {
	pub hold_forward: f32,
	pub hold_right: f32,
	pub hold_up: f32,
}

impl Default for GrenadeUserSettings {
	fn default() -> Self {
		Self { hold_forward: 0.22, hold_right: 0.16, hold_up: 0.08 }
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GrenadePhase {
	#[default]
	Ready,
	Windup {
		age: f32,
	},
	Recovery {
		age: f32,
	},
}

pub fn apply_throw_intents(
	mouse: Res<ButtonInput<MouseButton>>,
	mut intents: MessageReader<CharacterIntent>,
	bags: Query<&Inventory>,
	carriers: Query<&InventoryUser>,
	mut users: Query<(Entity, &mut GrenadeThrow, Has<WeaponSwap>), (With<Player>, With<GrenadeUser>)>,
) {
	let mut use_held = mouse.pressed(MouseButton::Left);
	for intent in intents.read() {
		if let CharacterIntent::UseItem(_) = *intent {
			use_held = true;
		}
	}
	for (entity, mut throw, swapping) in &mut users {
		if !use_held {
			throw.use_latched = false;
		}
		if swapping || throw.busy() || throw.use_latched {
			continue;
		}
		if !use_held {
			continue;
		}
		let ready = carriers
			.get(entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(Inventory::primary_weapon)
			.is_some_and(|item| match item {
				InventoryItem::Grenade { recharge, .. } => recharge.ready(),
				_ => false,
			});
		if !ready {
			continue;
		}
		throw.phase = GrenadePhase::Windup { age: 0.0 };
		throw.use_latched = true;
	}
}

pub fn advance_throw(
	time: Res<Time>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	spatial: SpatialQuery,
	looks: Query<&PlayerLook>,
	velocities: Query<&LinearVelocity>,
	carriers: Query<&InventoryUser>,
	mut bags: Query<&mut Inventory>,
	mut users: Query<(Entity, &GrenadeUser, &mut GrenadeThrow)>,
	mut held: Query<(&Transform, &mut Visibility), With<HeldGrenade>>,
) {
	let dt = time.delta_secs();
	for (user_entity, user, mut throw) in &mut users {
		let stats = carriers
			.get(user_entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats));
		match throw.phase {
			GrenadePhase::Ready => {}
			GrenadePhase::Windup { age } => {
				let Some(stats) = stats else {
					throw.phase = GrenadePhase::Ready;
					continue;
				};
				let next = age + dt;
				if age < stats.release_at && next >= stats.release_at {
					release_grenade(
						&mut commands,
						&mut meshes,
						&mut materials,
						&spatial,
						&looks,
						&velocities,
						&carriers,
						&mut bags,
						&mut held,
						user_entity,
						user.held,
						stats,
					);
				}
				if next >= stats.throw_secs {
					throw.phase = GrenadePhase::Ready;
				} else if next >= stats.release_at {
					throw.phase = GrenadePhase::Recovery { age: next };
				} else {
					throw.phase = GrenadePhase::Windup { age: next };
				}
			}
			GrenadePhase::Recovery { age } => {
				let throw_secs = stats.map(|stats| stats.throw_secs).unwrap_or(0.6);
				let next = age + dt;
				if next >= throw_secs {
					throw.phase = GrenadePhase::Ready;
				} else {
					throw.phase = GrenadePhase::Recovery { age: next };
				}
			}
		}
	}
}

fn release_grenade(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	spatial: &SpatialQuery,
	looks: &Query<&PlayerLook>,
	velocities: &Query<&LinearVelocity>,
	carriers: &Query<&InventoryUser>,
	bags: &mut Query<&mut Inventory>,
	held: &mut Query<(&Transform, &mut Visibility), With<HeldGrenade>>,
	user_entity: Entity,
	held_entity: Entity,
	stats: GrenadeStats,
) {
	let look = looks.get(user_entity).copied().unwrap_or_default();
	let inherit = velocities.get(user_entity).map(|v| v.0).unwrap_or(Vec3::ZERO);
	let Ok((pose, mut visibility)) = held.get_mut(held_entity) else {
		return;
	};
	*visibility = Visibility::Hidden;
	let origin = pose.translation;
	let spec = carriers
		.get(user_entity)
		.ok()
		.and_then(|carrier| bags.get(carrier.bag).ok())
		.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_spec))
		.unwrap_or_default();
	if let Ok(carrier) = carriers.get(user_entity) {
		if let Ok(mut bag) = bags.get_mut(carrier.bag) {
			if let Some(InventoryItem::Grenade { recharge, stats, .. }) = bag.primary_weapon_mut() {
				recharge.start(stats.recharge);
			}
		}
	}
	spawn_thrown_grenade(
		commands,
		meshes,
		materials,
		spatial,
		origin,
		launch_velocity(&look, &stats, inherit),
		spec,
		stats,
		GrenadeEffect::from_stats(&stats),
		user_entity,
		[user_entity, held_entity],
	);
}

pub fn launch_velocity(look: &PlayerLook, stats: &GrenadeStats, inherit: Vec3) -> Vec3 {
	let yaw = Quat::from_rotation_y(look.yaw);
	let pitch = Quat::from_rotation_x(-look.pitch);
	let aim = (yaw * pitch) * -Vec3::Z;
	let tossed = (aim + Vec3::Y * stats.upward_bias).normalize_or(Vec3::Y);
	tossed * stats.launch_speed + inherit * stats.inherit_velocity
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn launch_includes_upward_bias() {
		let look = PlayerLook { yaw: 0.0, pitch: 0.0, ..default() };
		let velocity = launch_velocity(&look, &GrenadeStats::standard(), Vec3::ZERO);
		assert!(velocity.y > 0.0);
		assert!(velocity.z < 0.0);
	}

	#[test]
	fn windup_crosses_release_once() {
		let stats = GrenadeStats::standard();
		let prev = stats.release_at - 0.2;
		let next = prev + 0.5;
		assert!(prev < stats.release_at && next >= stats.release_at);
	}
}
