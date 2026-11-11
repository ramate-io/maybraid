//! Opt-in gameplay evidence. These values carry observations, not AI decisions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDiagnosticActor {
	/// Opaque decimal entity bits, encoded as text to preserve all 64 bits.
	pub entity: String,
	pub player: bool,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub position: Option<[f32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPlayerVitals {
	pub entity: String,
	pub health: Option<f32>,
	pub max_health: Option<f32>,
	pub velocity: Option<[f32; 3]>,
	/// None for the fallback controller whose ground marker is private.
	pub grounded: Option<bool>,
	pub pending_support: bool,
	pub awaiting_surface: bool,
	pub gravity_scale: Option<f32>,
	pub sleeping: bool,
}

/// Explicit one-shot selection. Entity bits stay textual in JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "selection", rename_all = "snake_case")]
pub enum AgentNpcSelector {
	Nearest { radius: f32 },
	Entity { entity: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcGroundHit {
	pub entity: String,
	pub distance: f32,
	pub normal: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcGroundHits {
	pub count: usize,
	pub nearest: Option<AgentNpcGroundHit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcSupport {
	pub failure: Option<String>,
	pub held: [f32; 3],
	pub search_xz: [f32; 2],
	pub search_surface_y: Option<[f32; 2]>,
	pub exhausted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcSpotContact {
	pub subject: String,
	pub success_age: f32,
	pub attempt_age: f32,
	pub next_respot_in: f32,
	pub failures: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcTrajectory {
	pub target: String,
	pub clear: bool,
	pub checked_age: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcCombat {
	pub enabled: Option<bool>,
	pub engaged: Option<String>,
	pub engaged_contact_age: Option<f32>,
	pub player_contact_age: Option<f32>,
	/// At most the engaged target and selected player, deduplicated.
	pub spotting: Vec<AgentNpcSpotContact>,
	/// Cached observations only; no new visibility checks. At most 18 samples.
	pub trajectories: Vec<AgentNpcTrajectory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcWeapon {
	pub held: String,
	pub trigger: Option<bool>,
	pub launch_ready: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentNpcSnapshot {
	pub actor: AgentDiagnosticActor,
	pub velocity: Option<[f32; 3]>,
	pub gravity_scale: Option<f32>,
	/// None when this actor has no shared character motor.
	pub grounded: Option<bool>,
	pub shape_hits: Option<AgentNpcGroundHits>,
	/// Cached plane, with no assertion that it was refreshed this frame.
	pub cached_ground_normal: Option<[f32; 3]>,
	pub pending_support: Option<AgentNpcSupport>,
	pub combat: Option<AgentNpcCombat>,
	pub weapon: Option<AgentNpcWeapon>,
}

/// Actual camera state from an explicit request, never a player-position proxy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCameraSnapshot {
	pub entity: String,
	pub local_translation: [f32; 3],
	pub local_rotation: [f32; 4],
	pub global_translation: [f32; 3],
	pub global_rotation: [f32; 4],
	pub controller_yaw: f32,
	pub controller_pitch: f32,
	pub projection: AgentCameraProjection,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentCameraProjection {
	Perspective { fov: f32, near: f32, far: f32, aspect_ratio: f32 },
	Orthographic { near: f32, far: f32, scale: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticCommand {
	Enabled(bool),
	Health(f32),
	View { yaw: f32, pitch: f32 },
	Camera { pose: Option<[f32; 5]> },
	Npcs { radius: f32 },
	Npc { entity: u64 },
}

impl DiagnosticCommand {
	/// Console-only agent utilities; never registered in the gameplay console.
	pub fn parse(line: &str) -> Result<Self, String> {
		let words: Vec<_> = line.split_whitespace().collect();
		match words.as_slice() {
			["diagnostics", "on"] => Ok(Self::Enabled(true)),
			["diagnostics", "off"] => Ok(Self::Enabled(false)),
			["diagnostics", "health", amount] => {
				let amount = amount.parse::<f32>().map_err(|_| "health must be a number")?;
				if !amount.is_finite() || amount <= 0.0 {
					return Err("health must be finite and greater than zero".into());
				}
				Ok(Self::Health(amount))
			}
			["diagnostics", "view", yaw, pitch] => {
				let yaw = yaw.parse::<f32>().map_err(|_| "yaw must be a number")?;
				let pitch = pitch.parse::<f32>().map_err(|_| "pitch must be a number")?;
				if !yaw.is_finite() || !pitch.is_finite() {
					return Err("yaw and pitch must be finite radians".into());
				}
				Ok(Self::View { yaw, pitch })
			}
			["diagnostics", "camera"] => Ok(Self::Camera { pose: None }),
            ["diagnostics", "camera", x, y, z, yaw, pitch] => {
                let mut pose = [0.0; 5];
                for (output, text) in pose.iter_mut().zip([x,y,z,yaw,pitch]) {
                    *output = text.parse::<f32>().map_err(|_| "camera pose must contain finite numbers")?;
                    if !output.is_finite() { return Err("camera pose must contain finite numbers".into()); }
                }
                Ok(Self::Camera { pose: Some(pose) })
            }
			["diagnostics", "npcs", radius] => {
				let radius = radius.parse::<f32>().map_err(|_| "radius must be a number")?;
				if !radius.is_finite() || radius <= 0.0 || radius > 100.0 {
					return Err("radius must be finite and in (0, 100] metres".into());
				}
				Ok(Self::Npcs { radius })
			}
			["diagnostics", "npc", entity] => {
				if !entity.bytes().all(|byte| byte.is_ascii_digit()) {
					return Err("entity must be opaque decimal entity bits".into());
				}
				let entity = entity.parse::<u64>().map_err(|_| "invalid decimal entity bits")?;
				Ok(Self::Npc { entity })
			}
			_ => Err("expected diagnostics on | off | health AMOUNT | view YAW PITCH | camera [X Y Z YAW PITCH] | npcs RADIUS | npc ENTITY".into()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fixture_health_rejects_non_finite_zero_negative_and_trailing_arguments() {
		for line in [
			"diagnostics health NaN",
			"diagnostics health inf",
			"diagnostics health -inf",
			"diagnostics health 0",
			"diagnostics health -10",
			"diagnostics health 1 extra",
		] {
			assert!(DiagnosticCommand::parse(line).is_err(), "accepted {line}");
		}
	}

	#[test]
	fn commands_accept_explicit_observation_switch_and_finite_fixture() -> anyhow::Result<()> {
		assert_eq!(
			DiagnosticCommand::parse("diagnostics on").map_err(anyhow::Error::msg)?,
			DiagnosticCommand::Enabled(true)
		);
		assert_eq!(
			DiagnosticCommand::parse("diagnostics off").map_err(anyhow::Error::msg)?,
			DiagnosticCommand::Enabled(false)
		);
		assert_eq!(
			DiagnosticCommand::parse("diagnostics health 1000000").map_err(anyhow::Error::msg)?,
			DiagnosticCommand::Health(1_000_000.0)
		);
		Ok(())
	}

	#[test]
	fn view_requires_exactly_two_finite_angles() -> anyhow::Result<()> {
		for line in [
			"diagnostics view NaN 0",
			"diagnostics view 0 inf",
			"diagnostics view -inf 0",
			"diagnostics view 1",
			"diagnostics view 1 2 extra",
		] {
			assert!(DiagnosticCommand::parse(line).is_err(), "accepted {line}");
		}
		assert_eq!(
			DiagnosticCommand::parse("diagnostics view 7 -2").map_err(anyhow::Error::msg)?,
			DiagnosticCommand::View { yaw: 7.0, pitch: -2.0 },
		);
		Ok(())
	}
}

#[cfg(test)]
mod npc_tests {
	use super::*;

	#[test]
	fn npc_commands_require_bounded_radius_or_decimal_id() -> anyhow::Result<()> {
		for line in [
			"diagnostics npcs NaN",
			"diagnostics npcs inf",
			"diagnostics npcs 0",
			"diagnostics npcs -1",
			"diagnostics npcs 101",
			"diagnostics npcs 30 extra",
			"diagnostics npc -1",
			"diagnostics npc +1",
			"diagnostics npc 1.0",
			"diagnostics npc 18446744073709551616",
			"diagnostics npc 2 extra",
		] {
			assert!(DiagnosticCommand::parse(line).is_err(), "accepted {line}");
		}
		assert_eq!(
			DiagnosticCommand::parse("diagnostics npcs 100").map_err(anyhow::Error::msg)?,
			DiagnosticCommand::Npcs { radius: 100.0 }
		);
		assert_eq!(
			DiagnosticCommand::parse("diagnostics npc 18446744073709551615")
				.map_err(anyhow::Error::msg)?,
			DiagnosticCommand::Npc { entity: u64::MAX }
		);
		Ok(())
	}

	#[test]
	fn snapshot_roundtrips_null_missing_state_and_preserves_entity_bits() -> anyhow::Result<()> {
		use crate::{AgentEvent, AgentEventBody};
		let actor = AgentNpcSnapshot {
			actor: AgentDiagnosticActor {
				entity: u64::MAX.to_string(),
				player: false,
				name: None,
				position: None,
			},
			velocity: None,
			gravity_scale: None,
			grounded: None,
			shape_hits: None,
			cached_ground_normal: None,
			pending_support: None,
			combat: None,
			weapon: None,
		};
		let event = AgentEvent {
			t: 1.0,
			frame: 2,
			body: AgentEventBody::NpcSnapshot {
				request_id: 3,
				selector: AgentNpcSelector::Entity { entity: u64::MAX.to_string() },
				player: None,
				actors: vec![actor],
				error: None,
			},
		};
		let line = event.to_line().map_err(anyhow::Error::msg)?;
		let json: serde_json::Value = serde_json::from_str(&line)?;
		assert!(json["actors"][0]["grounded"].is_null());
		assert!(json["actors"][0]["pending_support"].is_null());
		assert!(json["actors"][0]["combat"].is_null());
		assert!(json["actors"][0]["weapon"].is_null());
		assert_eq!(json["actors"][0]["actor"]["entity"], u64::MAX.to_string());
		assert_eq!(AgentEvent::parse_line(&line).map_err(anyhow::Error::msg)?, event);
		assert_eq!(event.tracy_message(), None);
		Ok(())
	}
}

#[cfg(test)]
mod camera_tests {
	use crate::diagnostics::DiagnosticCommand;
	#[test]
	fn camera_pose_parser_requires_exact_finite_arguments() -> Result<(), String> {
		assert_eq!(
			DiagnosticCommand::parse("diagnostics camera")?,
			DiagnosticCommand::Camera { pose: None }
		);
		assert_eq!(
			DiagnosticCommand::parse("diagnostics camera 1 -2 3 4 -5")?,
			DiagnosticCommand::Camera { pose: Some([1., -2., 3., 4., -5.]) }
		);
		for line in [
			"diagnostics camera 1 2 3 4",
			"diagnostics camera 1 2 3 4 5 6",
			"diagnostics camera NaN 0 0 0 0",
			"diagnostics camera 0 inf 0 0 0",
			"diagnostics camera 0 0 0 0 -inf",
			"diagnostics camera 0 0 0 bad 0",
		] {
			assert!(DiagnosticCommand::parse(line).is_err(), "accepted {line}");
		}
		Ok(())
	}
}
