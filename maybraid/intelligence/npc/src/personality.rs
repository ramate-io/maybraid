use bevy::prelude::*;
use combat_targeting::CombatTargeting;
use damage::Health;
use evasion_intelligence::{EvasionIntelligenceUser, EvasionSettings};
use firearm_intelligence::{
	FirearmEngagement, FirearmIntelligence, FirearmIntelligenceSettings,
	FirearmMovementIntelligence, FirearmMovementIntelligenceSettings, FirearmTargeting,
};
use fleeing_intelligence::{FleeingSettings, FleeingUser};
use hiding_intelligence::{HidingSettings, HidingUser};
use meandering_intelligence::MeanderingIntelligenceUser;
use movement_intelligence::{
	MovementIntelligence, MovementLocation, MovementObjective, VantageStandoffs,
};
use poi_intelligence::{
	PoiIntelligenceUser, PoiInterests, PoiKnowledge, PoiLearningPolicy, PoiVisitPolicy,
	PoiVisitState,
};
use spotting_intelligence::{InterestLayers, SpotDirective, SpottingSettings, SpottingUser};
use tether_intelligence::{install_tether, StalkRadii, TetherIntelligenceUser, TetherObjective};
use threat_intelligence::{ThreatDiscoveryPolicy, ThreatIntelligenceUser, ThreatKnowledge};
use threat_management_intelligence::{ThreatManagementElement, ThreatManagementIntelligence};

use crate::NpcIntelligence;

/// Named NPC constructors. Mobs override fields after build.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Personality {
	Grazer,
	Predator,
	Civilian,
	Brawler,
	Assassin,
}

/// Capsule-derived sizes the installer needs without depending on character crates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NpcBody {
	pub agent_radius: f32,
	pub feet_below_origin: f32,
	pub eye_height: f32,
}

impl Default for NpcBody {
	fn default() -> Self {
		Self { agent_radius: 0.4, feet_below_origin: 0.9, eye_height: 1.45 }
	}
}

/// Optional combat actuator stamped by a personality.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombatPersonality {
	pub firearm: FirearmIntelligenceSettings,
	pub movement: FirearmMovementIntelligenceSettings,
}

/// Optional evade actuator stamped by a personality.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvadePersonality {
	pub flee_distance: f32,
	pub flee_radius: f32,
	pub hide: bool,
}

/// Idle leash or stalking ring, bound to a tether subject at install.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PersonalityTether {
	Leash { radius: f32 },
	Stalk { without: f32, within: f32 },
}

impl PersonalityTether {
	pub fn objective(self, subject: Entity) -> TetherObjective {
		match self {
			Self::Leash { radius } => TetherObjective::Tether(subject, radius),
			Self::Stalk { without, within } => {
				TetherObjective::Stalk(subject, StalkRadii::new(without, within))
			}
		}
	}
}

/// Data a personality stamps. Does not classify threats or pick POI kinds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PersonalitySpec {
	pub kind: Personality,
	pub threat: ThreatManagementIntelligence,
	pub combat: Option<CombatPersonality>,
	pub evade: Option<EvadePersonality>,
	pub meander_radius: f32,
	pub linger_secs: f32,
	pub visit_policy: PoiVisitPolicy,
	pub tether: PersonalityTether,
	pub tether_added_radius: f32,
	pub engaged_tether_radius: Option<f32>,
	pub keep_tether_in_combat: bool,
	pub spotting_range: f32,
	pub discovery_radius: f32,
}

/// Mob / playground context for [`PersonalitySpec::install`].
#[derive(Clone, Debug)]
pub struct NpcInstall {
	pub at: Vec3,
	pub body: NpcBody,
	pub health: Health,
	pub tether: Option<Entity>,
	pub poi_interests: PoiInterests,
	pub engagement: Option<FirearmEngagement>,
	pub threat_override: Option<ThreatManagementIntelligence>,
	pub discovery_radius: Option<f32>,
	pub spotting_range: Option<f32>,
	/// When false, skip combat brains even if the spec has them (unarmed civilian).
	pub armed: bool,
	/// Mob override. `None` keeps the personality default (Hunt flips this on).
	pub keep_tether_in_combat: Option<bool>,
}

impl Default for NpcInstall {
	fn default() -> Self {
		Self {
			at: Vec3::ZERO,
			body: NpcBody::default(),
			health: Health::default(),
			tether: None,
			poi_interests: PoiInterests::default(),
			engagement: None,
			threat_override: None,
			discovery_radius: None,
			spotting_range: None,
			armed: true,
			keep_tether_in_combat: None,
		}
	}
}

impl Personality {
	pub fn spec(self) -> PersonalitySpec {
		match self {
			Self::Grazer => PersonalitySpec::grazer(),
			Self::Predator => PersonalitySpec::predator(),
			Self::Civilian => PersonalitySpec::civilian(),
			Self::Brawler => PersonalitySpec::brawler(),
			Self::Assassin => PersonalitySpec::assassin(),
		}
	}

	pub fn install(self, commands: &mut Commands, entity: Entity, install: NpcInstall) {
		self.spec().install(commands, entity, install);
	}
}

impl PersonalitySpec {
	pub fn grazer() -> Self {
		let mut threat = ThreatManagementIntelligence::default();
		threat.ignore = ThreatManagementElement::new(0.5, 0.0);
		threat.evade = ThreatManagementElement::new(0.0, 1.2);
		threat.combat = ThreatManagementElement::ZERO;
		threat.commitment = (1.1, 1.0);
		threat.proximity_horizon = 18.0;
		Self {
			kind: Personality::Grazer,
			threat,
			combat: None,
			evade: Some(EvadePersonality { flee_distance: 14.0, flee_radius: 20.0, hide: true }),
			meander_radius: 36.0,
			linger_secs: 6.0,
			visit_policy: PoiVisitPolicy::default(),
			tether: PersonalityTether::Leash { radius: 24.0 },
			tether_added_radius: 16.0,
			engaged_tether_radius: None,
			keep_tether_in_combat: false,
			spotting_range: 40.0,
			discovery_radius: 24.0,
		}
	}

	pub fn civilian() -> Self {
		let mut threat = ThreatManagementIntelligence::default();
		threat.ignore = ThreatManagementElement::new(0.4, 0.0);
		threat.evade = ThreatManagementElement::new(0.0, 1.0);
		threat.combat = ThreatManagementElement::new(0.2, 0.4);
		threat.commitment = (1.3, 1.0);
		threat.proximity_horizon = 22.0;
		Self {
			kind: Personality::Civilian,
			threat,
			combat: Some(CombatPersonality {
				firearm: FirearmIntelligenceSettings {
					accuracy: 0.45,
					trigger_happiness: 0.4,
					headshots: 0.08,
					vision: 4,
					..FirearmIntelligenceSettings::default()
				},
				movement: FirearmMovementIntelligenceSettings {
					range: (10.0, 0.8),
					cover: 0.7,
					flee: (2.0, 10.0),
					..FirearmMovementIntelligenceSettings::default()
				},
			}),
			evade: Some(EvadePersonality { flee_distance: 10.0, flee_radius: 16.0, hide: true }),
			meander_radius: 28.0,
			linger_secs: 5.0,
			visit_policy: PoiVisitPolicy::default(),
			tether: PersonalityTether::Leash { radius: 16.0 },
			tether_added_radius: 10.0,
			engaged_tether_radius: None,
			keep_tether_in_combat: false,
			spotting_range: 48.0,
			discovery_radius: 28.0,
		}
	}

	pub fn predator() -> Self {
		let mut threat = ThreatManagementIntelligence::default();
		threat.ignore = ThreatManagementElement::ZERO;
		threat.evade = ThreatManagementElement::new(-1.5, 0.4);
		threat.combat = ThreatManagementElement::new(1.0, 1.0);
		threat.commitment = (1.2, 1.0);
		threat.proximity_horizon = 40.0;
		Self {
			kind: Personality::Predator,
			threat,
			combat: Some(CombatPersonality {
				firearm: FirearmIntelligenceSettings {
					accuracy: 0.72,
					motion_tracking: 0.7,
					trigger_happiness: 0.55,
					headshots: 0.25,
					vision: 6,
					..FirearmIntelligenceSettings::default()
				},
				movement: FirearmMovementIntelligenceSettings {
					range: (12.0, 1.0),
					cover: 0.4,
					flee: (1.5, 8.0),
					..FirearmMovementIntelligenceSettings::default()
				},
			}),
			evade: Some(EvadePersonality { flee_distance: 6.0, flee_radius: 14.0, hide: true }),
			meander_radius: 32.0,
			linger_secs: 2.5,
			visit_policy: PoiVisitPolicy::default(),
			tether: PersonalityTether::Stalk { without: 8.0, within: 22.0 },
			tether_added_radius: 4.0,
			engaged_tether_radius: Some(6.0),
			keep_tether_in_combat: false,
			spotting_range: 64.0,
			discovery_radius: 48.0,
		}
	}

	pub fn brawler() -> Self {
		let mut threat = ThreatManagementIntelligence::default();
		threat.ignore = ThreatManagementElement::ZERO;
		threat.evade = ThreatManagementElement::new(-0.8, 0.2);
		threat.combat = ThreatManagementElement::new(1.0, 0.8);
		threat.commitment = (1.4, 1.0);
		threat.proximity_horizon = 28.0;
		Self {
			kind: Personality::Brawler,
			threat,
			combat: Some(CombatPersonality {
				firearm: FirearmIntelligenceSettings {
					accuracy: 0.55,
					motion_tracking: 0.45,
					counter_recoil: 0.45,
					trigger_happiness: 0.85,
					headshots: 0.05,
					wall_firing: 0.2,
					vision: 4,
					focus: 0.45,
					..FirearmIntelligenceSettings::default()
				},
				movement: FirearmMovementIntelligenceSettings {
					range: (8.0, 1.0),
					cover: 0.35,
					flee: (0.0, 8.0),
					..FirearmMovementIntelligenceSettings::default()
				},
			}),
			evade: Some(EvadePersonality { flee_distance: 8.0, flee_radius: 12.0, hide: false }),
			meander_radius: 20.0,
			linger_secs: 2.0,
			visit_policy: PoiVisitPolicy::default(),
			tether: PersonalityTether::Leash { radius: 8.0 },
			tether_added_radius: 2.0,
			engaged_tether_radius: Some(6.0),
			keep_tether_in_combat: false,
			spotting_range: 56.0,
			discovery_radius: 36.0,
		}
	}

	pub fn assassin() -> Self {
		let mut threat = ThreatManagementIntelligence::default();
		threat.ignore = ThreatManagementElement::new(0.2, 0.0);
		threat.evade = ThreatManagementElement::new(-0.6, 0.6);
		threat.combat = ThreatManagementElement::new(0.6, 1.2);
		threat.commitment = (1.3, 1.0);
		threat.proximity_horizon = 20.0;
		Self {
			kind: Personality::Assassin,
			threat,
			combat: Some(CombatPersonality {
				firearm: FirearmIntelligenceSettings {
					accuracy: 0.84,
					motion_tracking: 0.75,
					counter_recoil: 0.7,
					trigger_happiness: 0.35,
					headshots: 0.55,
					wall_firing: 0.0,
					vision: 6,
					focus: 0.75,
					..FirearmIntelligenceSettings::default()
				},
				movement: FirearmMovementIntelligenceSettings {
					range: (14.0, 1.0),
					cover: 0.85,
					flee: (3.0, 10.0),
					..FirearmMovementIntelligenceSettings::default()
				},
			}),
			evade: Some(EvadePersonality { flee_distance: 3.0, flee_radius: 12.0, hide: true }),
			meander_radius: 24.0,
			linger_secs: 3.0,
			visit_policy: PoiVisitPolicy::default(),
			tether: PersonalityTether::Stalk { without: 12.0, within: 28.0 },
			tether_added_radius: 3.0,
			engaged_tether_radius: Some(8.0),
			keep_tether_in_combat: false,
			spotting_range: 72.0,
			discovery_radius: 40.0,
		}
	}

	/// Insert the HOB stack and personality actuators. Does not spawn a body.
	pub fn install(self, commands: &mut Commands, entity: Entity, install: NpcInstall) {
		let mut movement = MovementIntelligence::new(MovementObjective::Reach(
			MovementLocation::new(install.at, install.body.agent_radius),
		));
		movement.ability.agent_radius = install.body.agent_radius;
		movement.ability.feet_below_origin = install.body.feet_below_origin;
		movement.ability.eye_height = install.body.eye_height;
		if self.combat.is_some() && install.armed {
			movement.ability.candidate_budget.max_candidates = 8;
			movement.ability.candidate_budget.horizon = 30.0;
			movement.ability.vantage_standoffs = VantageStandoffs::from_radii(&[6.0, 10.0]);
			movement.ability.vantage_azimuths = 4;
		} else {
			movement.ability.candidate_budget.max_candidates = 6;
			movement.ability.candidate_budget.horizon = 24.0;
		}

		let spotting_range = install.spotting_range.unwrap_or(self.spotting_range);
		let discovery_radius = install.discovery_radius.unwrap_or(self.discovery_radius);
		let eye_offset =
			Vec3::Y * (install.body.eye_height - install.body.feet_below_origin).max(0.0);
		let desired_count = if self.combat.is_some() && install.armed { 8 } else { 4 };
		let vision =
			self.combat.map(|combat| usize::from(combat.firearm.vision.max(1))).unwrap_or(4);
		let freshness = self
			.combat
			.map(|combat| combat.firearm.fire_spotting_freshness.max(0.125))
			.unwrap_or(0.5);
		let memory = self.combat.map(|combat| combat.firearm.target_spotting_memory).unwrap_or(4.0);
		let directive = SpotDirective {
			layers: InterestLayers::CHARACTER,
			range: spotting_range,
			priority: 1,
			desired_count,
			freshness_secs: freshness,
			discovery_interval_secs: 0.125,
			respot_interval_secs: 0.125,
			max_samples_per_subject: vision,
		};
		let spotting = SpottingUser::new(eye_offset, [directive])
			.with_settings(SpottingSettings::new(desired_count, desired_count.max(vision), memory));

		let threat = install.threat_override.unwrap_or(self.threat);
		let idle_tether = install.tether.map(|subject| self.tether.objective(subject));
		let engaged_tether = install
			.tether
			.zip(self.engaged_tether_radius)
			.map(|(subject, radius)| TetherObjective::Tether(subject, radius));
		let npc = NpcIntelligence {
			idle_tether,
			engaged_tether,
			keep_tether_in_combat: install
				.keep_tether_in_combat
				.unwrap_or(self.keep_tether_in_combat),
		};

		let mut learning = PoiIntelligenceUser::new(install.poi_interests);
		learning.policy = PoiLearningPolicy {
			local_radius: self.meander_radius.max(learning.policy.local_radius),
			..learning.policy
		};

		let mut meandering = MeanderingIntelligenceUser::new(self.meander_radius);
		meandering.visit_policy = self.visit_policy;
		meandering.linger_secs = self.linger_secs.max(0.0);

		commands.entity(entity).insert((
			npc,
			movement,
			spotting,
			threat,
			ThreatKnowledge::default(),
			ThreatIntelligenceUser::new(ThreatDiscoveryPolicy {
				radius: discovery_radius,
				scan_interval_secs: 0.125,
				retained_scan_interval_secs: 2.0,
				retention_secs: 8.0,
				desired_threats: 8,
				candidates_per_scan: 16,
				max_known: 32,
				threat_threshold: 0.2,
			}),
			meandering,
			learning,
			PoiKnowledge::default(),
			PoiVisitState::default(),
			install.health,
		));

		if let Some(subject) = install.tether {
			let mut user = TetherIntelligenceUser::new(self.tether.objective(subject))
				.with_added_radius(self.tether_added_radius);
			user.enabled = true;
			install_tether(commands, entity, user);
		}

		if install.armed {
			if let Some(combat) = self.combat {
				let mut firearm = FirearmIntelligence::new();
				firearm.settings = combat.firearm;
				let mut firearm_movement = FirearmMovementIntelligence::new();
				firearm_movement.settings = combat.movement;
				commands.entity(entity).insert((
					firearm,
					firearm_movement,
					CombatTargeting::default(),
					FirearmTargeting::default(),
					install.engagement.unwrap_or_else(FirearmEngagement::weapons_free),
				));
			}
		}

		if let Some(evade) = self.evade {
			commands.entity(entity).insert((
				EvasionIntelligenceUser::new(EvasionSettings {
					flee_distance: evade.flee_distance,
					memory_secs: 4.0,
				}),
				FleeingUser::new(FleeingSettings { radius: evade.flee_radius }),
			));
			if evade.hide {
				commands.entity(entity).insert(HidingUser::new(HidingSettings::default()));
			}
		}
	}
}
