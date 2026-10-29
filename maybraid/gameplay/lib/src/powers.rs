//! Equipped power use → Lubikan activation. Connects equipment to Lubikan
//! without either depending on world.

use bevy::prelude::*;
use lubikans::{LubikanActivationRequest, LubikanHealingEnabled, LubikanOwner, LubikanSystems};
use maybraid_equipment::EquippedPowerUse;
use maybraid_session::WorldGameplayEnabled;

/// Route addressed [`EquippedPowerUse`] to equipped Lubikan activation and
/// gate healing with the session gameplay projection.
pub fn configure(app: &mut App) {
	app.add_observer(request_lubikan_activation)
		.add_systems(Update, sync_lubikan_healing_enabled.before(LubikanSystems::Activate));
}

/// Pause gates healing. Capture (console / map overlay) is not needed here.
fn sync_lubikan_healing_enabled(
	gameplay: Res<WorldGameplayEnabled>,
	healing: Option<ResMut<LubikanHealingEnabled>>,
) {
	let Some(mut healing) = healing else {
		return;
	};
	if healing.0 != gameplay.0 {
		healing.0 = gameplay.0;
	}
}

/// Route addressed PowerUseItem to equipped Lubikan activation.
fn request_lubikan_activation(
	use_item: On<EquippedPowerUse>,
	gameplay: Res<WorldGameplayEnabled>,
	mut requests: MessageWriter<LubikanActivationRequest>,
	owners: Query<(), With<LubikanOwner>>,
) {
	if gameplay.0 && owners.contains(use_item.entity) {
		requests.write(LubikanActivationRequest::equipped(use_item.entity));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_character_controller::ActorTarget;

	#[test]
	fn power_use_item_requests_equipped_activation() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_observer(request_lubikan_activation)
			.add_message::<LubikanActivationRequest>()
			.insert_resource(WorldGameplayEnabled(true));
		let owner = app.world_mut().spawn(LubikanOwner).id();
		app.world_mut().trigger(EquippedPowerUse::new(ActorTarget::new(owner)));
		let request = app
			.world_mut()
			.run_system_once(|mut requests: MessageReader<LubikanActivationRequest>| {
				requests.read().next().copied()
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("activation request"))?;
		assert_eq!(request, LubikanActivationRequest::equipped(owner));
		Ok(())
	}

	#[test]
	fn gameplay_off_freezes_healing() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(LubikanHealingEnabled(true));
		world
			.run_system_once(sync_lubikan_healing_enabled)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<LubikanHealingEnabled>().0);
		Ok(())
	}
}
