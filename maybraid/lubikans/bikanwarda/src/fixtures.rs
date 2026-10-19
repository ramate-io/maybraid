//! Test helpers: register Bikanwarda definitions and equip Ward / Enraged.

use bevy::prelude::*;
use lubikans::{LubikanCatalog, LubikanLedger, LubikanOwner, LubikansPlugin};
use player::Player;

use crate::identity::{
	enraged_definition, is_bikanwarda_account, ward_definition, BIKANWARDA_ENRAGED_ACCOUNT,
	BIKANWARDA_WARD_ACCOUNT,
};

/// Register Ward and Enraged in the catalog and unlock both on `owner`.
pub fn unlock_bikanwarda_accounts(world: &mut World, owner: Entity) -> anyhow::Result<()> {
	let ward = ward_definition();
	let enraged = enraged_definition();
	if let Some(mut catalog) = world.get_resource_mut::<LubikanCatalog>() {
		catalog.register(ward);
		catalog.register(enraged);
	}
	let mut ledger = world
		.get_mut::<LubikanLedger>(owner)
		.ok_or_else(|| anyhow::anyhow!("owner needs a LubikanLedger"))?;
	ledger.ensure_owned(ward);
	ledger.ensure_owned(enraged);
	Ok(())
}

/// Select Ward on `owner`.
pub fn equip_ward(world: &mut World, owner: Entity) -> anyhow::Result<()> {
	unlock_bikanwarda_accounts(world, owner)?;
	world
		.get_mut::<LubikanLedger>(owner)
		.ok_or_else(|| anyhow::anyhow!("ledger"))?
		.select(Some(BIKANWARDA_WARD_ACCOUNT));
	Ok(())
}

/// Select Enraged on `owner`.
pub fn equip_enraged(world: &mut World, owner: Entity) -> anyhow::Result<()> {
	unlock_bikanwarda_accounts(world, owner)?;
	world
		.get_mut::<LubikanLedger>(owner)
		.ok_or_else(|| anyhow::anyhow!("ledger"))?
		.select(Some(BIKANWARDA_ENRAGED_ACCOUNT));
	Ok(())
}

/// Spawn a player-style owner with Bikanwarda accounts unlocked and Ward equipped.
pub fn spawn_ward_owner(app: &mut App) -> anyhow::Result<Entity> {
	if !app.is_plugin_added::<LubikansPlugin>() {
		app.add_plugins(LubikansPlugin);
	}
	let owner = app
		.world_mut()
		.spawn((Player, LubikanOwner))
		.id();
	app.update();
	equip_ward(app.world_mut(), owner)?;
	Ok(owner)
}

pub fn assert_bikanwarda_equipped(world: &World, owner: Entity) -> anyhow::Result<()> {
	let ledger = world
		.get::<LubikanLedger>(owner)
		.ok_or_else(|| anyhow::anyhow!("ledger"))?;
	anyhow::ensure!(
		is_bikanwarda_account(ledger.selection().resolved()),
		"expected Bikanwarda equipped"
	);
	Ok(())
}
