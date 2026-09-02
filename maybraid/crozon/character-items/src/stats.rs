//! Rolled item attributes. Stats are integers so inventory stays [`Eq`].
//!
//! Generation is identity-hashed (same mesh / look / color → same stats) so
//! old saves without a stats blob can recover a deterministic roll. Gameplay
//! systems do not consume these yet; menus display them and
//! [`CharacterSheet`] compiles worn clothing plus queued weapon weight.

use serde::{Deserialize, Serialize};

use crate::{
	names::mix, ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryItem, ItemColor,
	ItemRng,
};

const CLOTHING_SEED: u64 = 0x51A7_0001_C10A_0001;
const FIREARM_SEED: u64 = 0xF1A4_A11A_0002_C0DE;

const BASE_HEALTH: i16 = 100;
const BASE_RUNNING: i16 = 100;
const BASE_JUMP: i16 = 100;
const BASE_AGILITY: i16 = 100;
const BASE_STRENGTH: i16 = 100;

const BOLT_SPEED_MIN: u32 = 120;
const BOLT_SPEED_MAX: u32 = 500;
const LASER_SPEED_MIN: u32 = 60;
const LASER_SPEED_MAX: u32 = 120;

const BOLT_PEN_MIN: u32 = 600;
const BOLT_PEN_MAX: u32 = 1200;
const BULLET_PEN_MIN: u32 = 250;
const BULLET_PEN_MAX: u32 = 700;
const RANGE_PEN_FLOOR: u32 = 250;
const RANGE_PEN_CEILING: u32 = 1200;

const RPM_MIN: u32 = 400;
const RPM_LOW_MAX: u32 = 700;
const RPM_MOD_MAX: u32 = 1100;
const RPM_MAX: u32 = 1500;

const LASER_DPC_CENTER: i16 = 22;
const LASER_DPC_SPREAD: u16 = 6;

/// Projectile family on a firearm. Mirrors the `firearms` crate conceptually
/// without taking a dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectileKind {
	Bolt,
	Bullet,
	Laser,
}

impl ProjectileKind {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Bolt => "Bolt",
			Self::Bullet => "Bullet",
			Self::Laser => "Laser",
		}
	}

	pub const fn is_ballistic(self) -> bool {
		!matches!(self, Self::Laser)
	}
}

/// Fire mode for bolts and bullets. Lasers have none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FireMode {
	FullAuto { rpm: u16 },
	Burst { rounds: u8, rpm: u16 },
	SemiAuto,
	Gated { recharge_tenths: u8 },
}

impl FireMode {
	pub fn label(self) -> String {
		match self {
			Self::FullAuto { rpm } => format!("Full auto {rpm} RPM"),
			Self::Burst { rounds, rpm } => format!("Burst {rounds} · {rpm} RPM"),
			Self::SemiAuto => String::from("Semi-auto"),
			Self::Gated { recharge_tenths } => {
				format!("Recharge {:.1}s", f32::from(recharge_tenths) / 10.0)
			}
		}
	}

	fn catalog_label(self) -> String {
		match self {
			Self::FullAuto { rpm } => format!("Auto {rpm}"),
			Self::Burst { rounds, .. } => format!("Burst {rounds}"),
			Self::SemiAuto => String::from("Semi"),
			Self::Gated { recharge_tenths } => {
				format!("Gate {:.1}s", f32::from(recharge_tenths) / 10.0)
			}
		}
	}
}

/// Clothing buffs plus carried weight. Most axes stay 0; one to three roll
/// in 4–16. Weight always rolls and is biased by mesh bulk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClothingStats {
	pub health: i16,
	pub running: i16,
	pub jump: i16,
	pub agility: i16,
	pub strength: i16,
	pub damage: i16,
	pub weight: u16,
}

impl ClothingStats {
	pub fn generate(mesh: ClothingMesh, material: ClothingMaterial, color: ItemColor) -> Self {
		let seed = mix(mix(mix(CLOTHING_SEED, mesh.label()), material.label()), color.label());
		generate_clothing(&mut ItemRng::from_seed(seed), mesh)
	}

	pub fn catalog_detail(self) -> String {
		let mut parts = vec![format!("W{}", self.weight)];
		for (label, value) in self.buff_shorts() {
			parts.push(format!("{:+} {label}", value));
		}
		parts.join(" · ")
	}

	pub fn stat_rows(self) -> Vec<(String, String)> {
		let mut rows = Vec::new();
		for (label, value) in [
			("Health", self.health),
			("Running", self.running),
			("Jump", self.jump),
			("Agility", self.agility),
			("Strength", self.strength),
			("Damage", self.damage),
		] {
			if value != 0 {
				rows.push((String::from(label), format!("{value:+}")));
			}
		}
		rows.push((String::from("Weight"), self.weight.to_string()));
		rows
	}

	fn buff_shorts(self) -> Vec<(&'static str, i16)> {
		[
			("HP", self.health),
			("Run", self.running),
			("Jump", self.jump),
			("Agi", self.agility),
			("Str", self.strength),
			("Dmg", self.damage),
		]
		.into_iter()
		.filter(|(_, value)| *value != 0)
		.take(2)
		.collect()
	}
}

/// Firearm combat stats. Penetration is millunits (`600` = 0.60). Recoil is
/// a small integer. Recharge is tenths of a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FirearmStats {
	pub projectile: ProjectileKind,
	pub speed: u16,
	pub penetration: u16,
	pub range: u16,
	pub fire: Option<FireMode>,
	pub recoil: u8,
	pub damage: u16,
	pub weight: u16,
}

impl FirearmStats {
	pub fn generate(mesh: FirearmMesh) -> Self {
		let seed = mix(FIREARM_SEED, mesh.label());
		generate_firearm(&mut ItemRng::from_seed(seed), mesh)
	}

	pub fn catalog_detail(self) -> String {
		let mut parts = vec![String::from(self.projectile.label())];
		if let Some(fire) = self.fire {
			parts.push(fire.catalog_label());
		}
		parts.push(format!("{} DPC", self.damage));
		parts.join(" · ")
	}

	pub fn stat_rows(self) -> Vec<(String, String)> {
		let mut rows = vec![
			(String::from("Projectile"), String::from(self.projectile.label())),
			(String::from("Speed"), format!("{}/s", self.speed)),
		];
		if self.projectile.is_ballistic() {
			rows.push((
				String::from("Penetration"),
				format!("{:.2}", f32::from(self.penetration) / 1000.0),
			));
		}
		rows.push((String::from("Range"), format!("{} m", self.range)));
		if let Some(fire) = self.fire {
			rows.push((String::from("Fire"), fire.label()));
		}
		if self.projectile.is_ballistic() {
			rows.push((String::from("Recoil"), self.recoil.to_string()));
		}
		rows.push((String::from("DPC"), self.damage.to_string()));
		rows.push((String::from("Weight"), self.weight.to_string()));
		rows
	}

	/// Inclusive DPC band implied by fire mode, recoil, and recharge.
	pub fn damage_band(self) -> Option<(u16, u16)> {
		damage_band(self.projectile, self.fire, self.recoil)
	}
}

/// Species-agnostic compiled sheet. Worn clothing adds buffs and weight;
/// queued weapons add weight only. Primary-weapon combat stats stay on the
/// firearm, not mixed into health / strength.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterSheet {
	pub health: i16,
	pub running: i16,
	pub jump: i16,
	pub agility: i16,
	pub strength: i16,
	pub damage: i16,
	pub weight: u16,
}

impl CharacterSheet {
	pub const BASE: Self = Self {
		health: BASE_HEALTH,
		running: BASE_RUNNING,
		jump: BASE_JUMP,
		agility: BASE_AGILITY,
		strength: BASE_STRENGTH,
		damage: 0,
		weight: 0,
	};

	pub fn from_inventory(inventory: &Inventory) -> Self {
		let mut sheet = Self::BASE;
		for item in inventory.worn_items() {
			if let Some(stats) = item.clothing_stats() {
				sheet.add_clothing(stats);
			}
		}
		for &index in &inventory.weapons {
			if let Some(stats) = inventory.items.get(index).and_then(InventoryItem::firearm_stats) {
				sheet.weight = sheet.weight.saturating_add(stats.weight);
			}
		}
		sheet
	}

	fn add_clothing(&mut self, stats: ClothingStats) {
		self.health = self.health.saturating_add(stats.health);
		self.running = self.running.saturating_add(stats.running);
		self.jump = self.jump.saturating_add(stats.jump);
		self.agility = self.agility.saturating_add(stats.agility);
		self.strength = self.strength.saturating_add(stats.strength);
		self.damage = self.damage.saturating_add(stats.damage);
		self.weight = self.weight.saturating_add(stats.weight);
	}

	/// Preview locomotion factor as a percent. `100` at zero weight; not
	/// applied to movement this pass.
	pub fn pace(self) -> u16 {
		(10_000 / (100 + u32::from(self.weight))).min(100) as u16
	}

	pub fn stat_rows(self) -> Vec<(String, String)> {
		vec![
			(String::from("Health"), self.health.to_string()),
			(String::from("Running"), self.running.to_string()),
			(String::from("Jump"), self.jump.to_string()),
			(String::from("Agility"), self.agility.to_string()),
			(String::from("Strength"), self.strength.to_string()),
			(String::from("Damage"), format!("{:+}", self.damage)),
			(String::from("Weight"), self.weight.to_string()),
			(String::from("Pace"), format!("{}%", self.pace())),
		]
	}
}

fn generate_clothing(rng: &mut ItemRng, mesh: ClothingMesh) -> ClothingStats {
	let (weight_min, weight_max) = clothing_weight_range(mesh);
	let weight = rng.in_range(weight_min, weight_max) as u16;
	let mut axes = [0u8, 1, 2, 3, 4, 5];
	for i in (1..axes.len()).rev() {
		let j = rng.gen_index(i + 1);
		axes.swap(i, j);
	}
	let count = rng.in_range(1, 3) as usize;
	let mut stats = ClothingStats { weight, ..ClothingStats::default() };
	for axis in axes.into_iter().take(count) {
		let value = rng.in_range(4, 16) as i16;
		match axis {
			0 => stats.health = value,
			1 => stats.running = value,
			2 => stats.jump = value,
			3 => stats.agility = value,
			4 => stats.strength = value,
			_ => stats.damage = value,
		}
	}
	stats
}

fn clothing_weight_range(mesh: ClothingMesh) -> (u32, u32) {
	match mesh {
		ClothingMesh::TankTop
		| ClothingMesh::HaremPantsUpper
		| ClothingMesh::HaremPantsLowerWrap => (2, 8),
		ClothingMesh::Tunic
		| ClothingMesh::Pants
		| ClothingMesh::HaremPants
		| ClothingMesh::ShortDress => (6, 14),
		ClothingMesh::FittedCoat | ClothingMesh::KneeHighBoots => (10, 18),
		ClothingMesh::LongDress | ClothingMesh::Robe | ClothingMesh::RobeCoat => (14, 24),
	}
}

fn generate_firearm(rng: &mut ItemRng, mesh: FirearmMesh) -> FirearmStats {
	let projectile = pick_projectile(rng, mesh);
	let (speed, penetration, range, fire, recoil, damage) = match projectile {
		ProjectileKind::Laser => {
			let speed = rng.in_range(LASER_SPEED_MIN, LASER_SPEED_MAX) as u16;
			let range = laser_range(speed);
			let damage = roll_dpc(rng, LASER_DPC_CENTER, LASER_DPC_SPREAD, 0, 0, 0);
			(speed, 0, range, None, 0, damage)
		}
		ProjectileKind::Bolt | ProjectileKind::Bullet => {
			let speed = rng.in_range(BOLT_SPEED_MIN, BOLT_SPEED_MAX) as u16;
			let (pen_min, pen_max) = if projectile == ProjectileKind::Bolt {
				(BOLT_PEN_MIN, BOLT_PEN_MAX)
			} else {
				(BULLET_PEN_MIN, BULLET_PEN_MAX)
			};
			let penetration = rng.in_range(pen_min, pen_max) as u16;
			let range = ballistic_range(penetration, speed);
			let fire = pick_fire_mode(rng, mesh);
			let recoil = rng.in_range(0, 8) as u8;
			let damage = ballistic_dpc(rng, fire, recoil);
			(speed, penetration, range, Some(fire), recoil, damage)
		}
	};
	let weight = firearm_weight(rng, mesh, fire);
	FirearmStats { projectile, speed, penetration, range, fire, recoil, damage, weight }
}

fn pick_projectile(rng: &mut ItemRng, mesh: FirearmMesh) -> ProjectileKind {
	let weights: &[(ProjectileKind, u32)] = match mesh {
		FirearmMesh::Bullpup => {
			&[(ProjectileKind::Bolt, 50), (ProjectileKind::Bullet, 40), (ProjectileKind::Laser, 10)]
		}
		FirearmMesh::Silopup => {
			&[(ProjectileKind::Bolt, 30), (ProjectileKind::Bullet, 60), (ProjectileKind::Laser, 10)]
		}
		FirearmMesh::Reltor => {
			&[(ProjectileKind::Bolt, 40), (ProjectileKind::Bullet, 50), (ProjectileKind::Laser, 10)]
		}
		FirearmMesh::Samsonist => {
			&[(ProjectileKind::Bolt, 55), (ProjectileKind::Bullet, 35), (ProjectileKind::Laser, 10)]
		}
		FirearmMesh::Snailer => {
			&[(ProjectileKind::Bolt, 15), (ProjectileKind::Bullet, 15), (ProjectileKind::Laser, 70)]
		}
	};
	pick_weighted(rng, weights)
}

fn pick_fire_mode(rng: &mut ItemRng, mesh: FirearmMesh) -> FireMode {
	let weights: &[(u8, u32)] = match mesh {
		FirearmMesh::Bullpup => &[(0, 50), (1, 25), (2, 20), (3, 5)],
		FirearmMesh::Silopup => &[(0, 40), (1, 20), (2, 35), (3, 5)],
		FirearmMesh::Reltor => &[(0, 35), (1, 30), (2, 25), (3, 10)],
		FirearmMesh::Samsonist => &[(0, 15), (1, 20), (2, 25), (3, 40)],
		FirearmMesh::Snailer => &[(0, 20), (1, 20), (2, 20), (3, 40)],
	};
	match pick_weighted(rng, weights) {
		0 => FireMode::FullAuto { rpm: roll_full_auto_rpm(rng) },
		1 => {
			let rounds = [3u8, 4, 5][rng.gen_index(3)];
			FireMode::Burst { rounds, rpm: rng.in_range(RPM_MIN, RPM_MAX) as u16 }
		}
		2 => FireMode::SemiAuto,
		_ => FireMode::Gated { recharge_tenths: rng.in_range(8, 35) as u8 },
	}
}

fn roll_full_auto_rpm(rng: &mut ItemRng) -> u16 {
	match rng.in_range(0, 2) {
		0 => rng.in_range(RPM_MIN, RPM_LOW_MAX) as u16,
		1 => rng.in_range(RPM_LOW_MAX + 1, RPM_MOD_MAX) as u16,
		_ => rng.in_range(RPM_MOD_MAX + 1, RPM_MAX) as u16,
	}
}

fn ballistic_dpc(rng: &mut ItemRng, fire: FireMode, recoil: u8) -> u16 {
	match fire {
		FireMode::FullAuto { rpm } => {
			let (center, spread, recoil_per) = full_auto_dpc_params(rpm);
			roll_dpc(rng, center, spread, recoil, recoil_per, 0)
		}
		FireMode::Burst { rounds, .. } => {
			let (center, spread) = match rounds {
				3 => (30, 6),
				4 => (24, 3),
				_ => (20, 3),
			};
			roll_dpc(rng, center, spread, recoil, 1, 0)
		}
		FireMode::SemiAuto => roll_dpc(rng, 35, 5, recoil, 2, 0),
		FireMode::Gated { recharge_tenths } => {
			roll_dpc(rng, 60, 15, recoil, 2, i16::from(recharge_tenths))
		}
	}
}

fn full_auto_dpc_params(rpm: u16) -> (i16, u16, i16) {
	if rpm <= RPM_LOW_MAX as u16 {
		(30, 4, 1)
	} else if rpm <= RPM_MOD_MAX as u16 {
		(24, 2, 1)
	} else {
		(18, 2, 1)
	}
}

fn roll_dpc(
	rng: &mut ItemRng,
	center: i16,
	spread: u16,
	recoil: u8,
	recoil_per: i16,
	extra: i16,
) -> u16 {
	let delta = rng.in_range_i16(-(spread as i16), spread as i16);
	let value = i32::from(center)
		+ i32::from(delta)
		+ i32::from(recoil) * i32::from(recoil_per)
		+ i32::from(extra);
	value.clamp(1, 240) as u16
}

fn damage_band(
	projectile: ProjectileKind,
	fire: Option<FireMode>,
	recoil: u8,
) -> Option<(u16, u16)> {
	let (center, spread, recoil_per, extra) = match (projectile, fire) {
		(ProjectileKind::Laser, _) => (LASER_DPC_CENTER, LASER_DPC_SPREAD, 0, 0),
		(_, Some(FireMode::FullAuto { rpm })) => {
			let (center, spread, recoil_per) = full_auto_dpc_params(rpm);
			(center, spread, recoil_per, 0)
		}
		(_, Some(FireMode::Burst { rounds, .. })) => {
			let (center, spread) = match rounds {
				3 => (30, 6),
				4 => (24, 3),
				_ => (20, 3),
			};
			(center, spread, 1, 0)
		}
		(_, Some(FireMode::SemiAuto)) => (35, 5, 2, 0),
		(_, Some(FireMode::Gated { recharge_tenths })) => (60, 15, 2, i16::from(recharge_tenths)),
		_ => return None,
	};
	let bonus = i32::from(recoil) * i32::from(recoil_per) + i32::from(extra);
	let min = (i32::from(center) - i32::from(spread) + bonus).clamp(1, 240) as u16;
	let max = (i32::from(center) + i32::from(spread) + bonus).clamp(1, 240) as u16;
	Some((min, max))
}

fn ballistic_range(penetration: u16, speed: u16) -> u16 {
	let pen_span = RANGE_PEN_CEILING - RANGE_PEN_FLOOR;
	let pen_t = f32::from(penetration.saturating_sub(RANGE_PEN_FLOOR as u16)) / pen_span as f32;
	let speed_t = f32::from(speed.saturating_sub(BOLT_SPEED_MIN as u16))
		/ (BOLT_SPEED_MAX - BOLT_SPEED_MIN) as f32;
	let t = (0.75 * pen_t + 0.25 * speed_t).clamp(0.0, 1.0);
	(50.0 + t * 950.0).round().clamp(50.0, 1000.0) as u16
}

fn laser_range(speed: u16) -> u16 {
	let t = f32::from(speed.saturating_sub(LASER_SPEED_MIN as u16))
		/ (LASER_SPEED_MAX - LASER_SPEED_MIN) as f32;
	(90.0 + t * 10.0).round().clamp(80.0, 100.0) as u16
}

fn firearm_weight(rng: &mut ItemRng, mesh: FirearmMesh, fire: Option<FireMode>) -> u16 {
	let (min, max) = match mesh {
		FirearmMesh::Snailer => (6, 16),
		FirearmMesh::Bullpup | FirearmMesh::Silopup => (8, 18),
		FirearmMesh::Reltor => (10, 22),
		FirearmMesh::Samsonist => (14, 28),
	};
	let mut weight = rng.in_range(min, max);
	if matches!(fire, Some(FireMode::Gated { .. })) {
		weight = weight.saturating_add(rng.in_range(2, 6));
	}
	weight as u16
}

fn pick_weighted<T: Copy>(rng: &mut ItemRng, weights: &[(T, u32)]) -> T {
	let total: u32 = weights.iter().map(|(_, weight)| *weight).sum();
	let mut pick = rng.in_range(0, total.saturating_sub(1));
	for (value, weight) in weights {
		if pick < *weight {
			return *value;
		}
		pick -= *weight;
	}
	weights[0].0
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clothing_identity_is_stable_and_has_weight() {
		let a = ClothingStats::generate(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let b = ClothingStats::generate(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert_eq!(a, b);
		assert!(a.weight >= 2);
		let nonzero = [a.health, a.running, a.jump, a.agility, a.strength, a.damage]
			.into_iter()
			.filter(|value| *value != 0)
			.count();
		assert!((1..=3).contains(&nonzero));
		assert!(a.weight <= 8);
	}

	#[test]
	fn robe_is_heavier_than_tank() {
		let tank = ClothingStats::generate(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let robe = ClothingStats::generate(
			ClothingMesh::Robe,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert!(robe.weight >= tank.weight);
		assert!(robe.weight >= 14);
	}

	#[test]
	fn firearm_dpc_stays_in_formula_band() {
		for mesh in FirearmMesh::VALUES {
			let stats = FirearmStats::generate(*mesh);
			let (min, max) = stats.damage_band().expect("band");
			assert!(
				(min..=max).contains(&stats.damage),
				"{:?} dpc {} not in {min}..={max}",
				mesh,
				stats.damage
			);
			match stats.projectile {
				ProjectileKind::Laser => {
					assert!(stats.fire.is_none());
					assert_eq!(stats.penetration, 0);
					assert!(
						(LASER_SPEED_MIN as u16..=LASER_SPEED_MAX as u16).contains(&stats.speed)
					);
					assert!(stats.range <= 100);
				}
				ProjectileKind::Bolt => {
					assert!(stats.fire.is_some());
					assert!(
						(BOLT_PEN_MIN as u16..=BOLT_PEN_MAX as u16).contains(&stats.penetration)
					);
					assert!((BOLT_SPEED_MIN as u16..=BOLT_SPEED_MAX as u16).contains(&stats.speed));
					assert!((50..=1000).contains(&stats.range));
				}
				ProjectileKind::Bullet => {
					assert!(stats.fire.is_some());
					assert!((BULLET_PEN_MIN as u16..=BULLET_PEN_MAX as u16)
						.contains(&stats.penetration));
					assert!(stats.penetration < BOLT_PEN_MAX as u16);
					assert!((50..=1000).contains(&stats.range));
				}
			}
		}
	}

	#[test]
	fn character_sheet_sums_worn_clothing_and_weapon_weight() {
		let clothing = InventoryItem::clothing(
			ClothingMesh::Pants,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let gun = InventoryItem::firearm(FirearmMesh::Bullpup);
		let clothing_stats = clothing.clothing_stats().unwrap();
		let gun_stats = gun.firearm_stats().unwrap();
		let inventory =
			Inventory { items: vec![clothing, gun], clothing: vec![0], weapons: vec![1] };
		let sheet = CharacterSheet::from_inventory(&inventory);
		assert_eq!(sheet.health, BASE_HEALTH + clothing_stats.health);
		assert_eq!(sheet.weight, clothing_stats.weight + gun_stats.weight);
		assert!(sheet.pace() <= 100);
	}
}
