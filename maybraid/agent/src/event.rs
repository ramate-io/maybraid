//! Line-delimited JSON events the game writes for the agent / Tracy alignment.

use serde::{Deserialize, Serialize};

use crate::command::ScenarioMark;
use crate::diagnostics::{
	AgentCameraSnapshot, AgentDiagnosticActor, AgentNpcSelector, AgentNpcSnapshot,
	AgentPlayerVitals,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
	Acknowledged,
	Started,
	Finished,
	Failed,
}

/// One event. Internally tagged with `"kind"`; `t` and `frame` sit beside it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEvent {
	pub t: f32,
	pub frame: u64,
	#[serde(flatten)]
	pub body: AgentEventBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEventBody {
	Ready {
		detail: String,
	},
	Player {
		x: f32,
		y: f32,
		z: f32,
		yaw: f32,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		pitch: Option<f32>,
		#[serde(rename = "move")]
		r#move: [f32; 2],
		/// Composed Durham height under the capsule, when that cell is published.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		ground: Option<f32>,
		/// Overhead map is visible (player-toggled or locked picker).
		#[serde(default, skip_serializing_if = "std::ops::Not::not")]
		map_open: bool,
		/// Locked first-life / death spawn picker (`map.open && close_locked`).
		#[serde(default, skip_serializing_if = "std::ops::Not::not")]
		picker: bool,
		/// Present only when explicitly enabled with --agent-diagnostics.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		vitals: Option<AgentPlayerVitals>,
	},
	/// Accepted damage, including zero-effect hits and overkill accounting.
	DamageApplied {
		target: AgentDiagnosticActor,
		source: Option<AgentDiagnosticActor>,
		amount: f32,
		applied: f32,
		remaining: f32,
		point: [f32; 3],
	},
	Died {
		actor: AgentDiagnosticActor,
		source: Option<AgentDiagnosticActor>,
		point: [f32; 3],
	},
	/// An actual shot, rather than a trigger request or AI intention.
	WeaponFired {
		shooter: AgentDiagnosticActor,
		recoil: f32,
		age: f32,
	},
	/// Explicit diagnostic fixture mutation. No invulnerability is installed.
	FixtureHealth {
		actor: AgentDiagnosticActor,
		before: [f32; 2],
		after: [f32; 2],
	},
	/// One-shot, explicitly requested camera reset; angles are yaw/pitch radians.
	FixtureView {
		actor: AgentDiagnosticActor,
		camera: String,
		before: [f32; 2],
		after: [f32; 2],
	},
	/// One-shot actual camera evidence. After a set, global pose still precedes
	/// that local write; request a later snapshot after transform propagation.
	CameraSnapshot {
		request_id: u32,
		applied_local: bool,
		global_pose_pre_apply: bool,
		state: AgentCameraSnapshot,
	},

	Diagnostics {
		enabled: bool,
	},
	/// Explicit one-shot NPC evidence, capped at eight actors. Missing state is null.
	NpcSnapshot {
		request_id: u32,
		selector: AgentNpcSelector,
		player: Option<String>,
		actors: Vec<AgentNpcSnapshot>,
		error: Option<String>,
	},
	Command {
		id: u32,
		status: CommandStatus,
		cmd: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		detail: Option<String>,
	},
	Waypoint {
		id: u32,
		xz: [f32; 2],
		reached: bool,
		remaining: f32,
	},
	Scenario {
		name: String,
		mark: ScenarioMark,
	},
	ChunkLoads {
		added: u32,
	},
	/// Live actor census, written when it changes. `bodies` are spawned mob
	/// characters; `brains` run threat management (the NPC mixer).
	Actors {
		hosts: u32,
		bodies: u32,
		brains: u32,
		/// Mob bodies still suspended while waiting for support and clearance.
		#[serde(default)]
		pending_support: u32,
		/// Lowest / highest mob-body Y when `bodies > 0`.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_y: Option<f32>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		max_y: Option<f32>,
		/// Composed height under the lowest body, when that cell is published.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		ground_at_min_y: Option<f32>,
		/// Identity and full pose of the body selected by the minimum-Y census.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_body: Option<AgentDiagnosticActor>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_body_physics_position: Option<[f32; 3]>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_body_velocity: Option<[f32; 3]>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_body_health: Option<f32>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_body_downed: Option<bool>,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		min_body_pending_support: Option<bool>,
	},
	/// Locked first-life / death spawn picker opened or closed.
	Picker {
		open: bool,
	},
	/// Offscreen present image resize or blit/camera recreate.
	Presentation {
		action: String,
		window: [u32; 2],
		target: [u32; 2],
	},
	/// Live menu screen changed (`home`, `gallery`, …, `none`).
	Screen {
		name: String,
	},
	/// A leave prompt, name modal, or catalog overlay opened or closed.
	Modal {
		name: Option<String>,
	},
	/// Focused menu row changed.
	Focus {
		screen: String,
		row: u32,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		label: Option<String>,
	},
	/// `GameFlow` or `WorldPause` transition.
	Flow {
		flow: String,
		pause: String,
	},
	/// A menu or cycling invariant broke. Always a playtest failure.
	Invariant {
		detail: String,
	},
	/// D-pad-selected inventory cycle category changed.
	Cycle {
		category: String,
	},
	/// Equipped / active item in a cycle category changed.
	Equipped {
		category: String,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		item: Option<String>,
	},
	/// Tanzer mode and/or the Tanz that just became active.
	Tanz {
		mode: String,
		#[serde(default, skip_serializing_if = "Option::is_none")]
		id: Option<String>,
	},
	Quit,
}

impl AgentEvent {
	pub fn to_line(&self) -> Result<String, String> {
		serde_json::to_string(self).map_err(|error| error.to_string())
	}

	pub fn parse_line(line: &str) -> Result<Self, String> {
		serde_json::from_str(line.trim()).map_err(|error| error.to_string())
	}

	pub fn tracy_message(&self) -> Option<String> {
		match &self.body {
			AgentEventBody::Scenario { name, mark } => {
				Some(format!("agent.scenario {name} {mark:?}"))
			}
			AgentEventBody::Command { id, status, cmd, detail } => {
				let extra = detail.as_deref().unwrap_or("");
				Some(format!("agent.cmd #{id} {status:?} {cmd} {extra}"))
			}
			AgentEventBody::Waypoint { id, xz, reached, .. } => {
				let status = if *reached { "reached" } else { "timeout" };
				Some(format!("agent.waypoint #{id} {status} {:.1},{:.1}", xz[0], xz[1]))
			}
			AgentEventBody::Ready { detail } => Some(format!("agent.ready {detail}")),
			AgentEventBody::Picker { open } => Some(format!("agent.picker open={open}")),
			AgentEventBody::Actors {
				hosts,
				bodies,
				brains,
				pending_support,
				min_y,
				max_y,
				ground_at_min_y,
				min_body,
				..
			} => {
				let mut message = format!(
					"agent.actors hosts={hosts} bodies={bodies} brains={brains} pending_support={pending_support}"
				);
				if let Some(min_y) = min_y {
					message.push_str(&format!(" min_y={min_y:.1}"));
				}
				if let Some(max_y) = max_y {
					message.push_str(&format!(" max_y={max_y:.1}"));
				}
				if let Some(body) = min_body {
					message.push_str(&format!(" min_body={}", body.entity));
				}
				if let Some(ground) = ground_at_min_y {
					message.push_str(&format!(" ground={ground:.1}"));
				}
				Some(message)
			}
			AgentEventBody::Presentation { action, window, target } => Some(format!(
				"agent.presentation {action} window {}x{} target {}x{}",
				window[0], window[1], target[0], target[1]
			)),
			AgentEventBody::Screen { name } => Some(format!("agent.screen {name}")),
			AgentEventBody::Modal { name } => {
				Some(format!("agent.modal {}", name.as_deref().unwrap_or("none")))
			}
			AgentEventBody::Focus { screen, row, label } => {
				let extra = label.as_deref().unwrap_or("");
				Some(format!("agent.focus {screen} row={row} {extra}"))
			}
			AgentEventBody::Flow { flow, pause } => {
				Some(format!("agent.flow {flow} pause={pause}"))
			}
			AgentEventBody::Invariant { detail } => Some(format!("agent.invariant {detail}")),
			AgentEventBody::Cycle { category } => Some(format!("agent.cycle {category}")),
			AgentEventBody::Equipped { category, item } => {
				Some(format!("agent.equipped {category} {}", item.as_deref().unwrap_or("none")))
			}
			AgentEventBody::Tanz { mode, id } => Some(match id {
				Some(id) => format!("agent.tanz mode={mode} id={id}"),
				None => format!("agent.tanz mode={mode}"),
			}),
			_ => None,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn player_roundtrip() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 1.25,
			frame: 74,
			body: AgentEventBody::Player {
				x: 3.0,
				y: 12.5,
				z: -1.0,
				yaw: 0.4,
				pitch: Some(-0.12),
				r#move: [0.0, 1.0],
				ground: Some(11.4),
				map_open: false,
				picker: false,
				vitals: None,
			},
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		let parsed = AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(parsed, event);
		assert!(line.contains(r#""kind":"player""#));
		Ok(())
	}

	#[test]
	fn actors_roundtrip_and_tracy_message() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 30.0,
			frame: 1800,
			body: AgentEventBody::Actors {
				hosts: 4,
				bodies: 61,
				brains: 61,
				pending_support: 2,
				min_y: Some(-2.0),
				max_y: Some(14.0),
				ground_at_min_y: Some(0.5),
				min_body: None,
				min_body_physics_position: None,
				min_body_velocity: None,
				min_body_health: None,
				min_body_downed: None,
				min_body_pending_support: None,
			},
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"actors""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		assert_eq!(
			event.tracy_message().as_deref(),
			Some(
				"agent.actors hosts=4 bodies=61 brains=61 pending_support=2 min_y=-2.0 max_y=14.0 ground=0.5"
			)
		);
		Ok(())
	}

	#[test]
	fn minimum_actor_observation_roundtrips() -> anyhow::Result<()> {
		let line = r#"{"t":2.0,"frame":20,"kind":"actors","hosts":1,"bodies":3,"brains":3,"min_body":{"entity":"18446744073709551615","player":false,"position":[10.0,-1999.3,20.0]},"min_body_physics_position":[10.0,-1999.3,20.0],"min_body_velocity":[0.0,0.0,0.0],"min_body_health":25.0,"min_body_downed":false,"min_body_pending_support":false}"#;
		let event = AgentEvent::parse_line(line).map_err(|e| anyhow::anyhow!("{e}"))?;
		let encoded = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(AgentEvent::parse_line(&encoded).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		assert!(encoded.contains("18446744073709551615"));
		assert!(event
			.tracy_message()
			.is_some_and(|message| message.contains("min_body=18446744073709551615")));
		Ok(())
	}

	#[test]
	fn older_player_and_actors_lines_still_parse() -> anyhow::Result<()> {
		let player = AgentEvent::parse_line(
			r#"{"t":1.0,"frame":10,"kind":"player","x":0.0,"y":1.4,"z":0.0,"yaw":0.0,"move":[0,0]}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			player.body,
			AgentEventBody::Player { ground: None, map_open: false, picker: false, .. }
		));
		let actors = AgentEvent::parse_line(
			r#"{"t":2.0,"frame":20,"kind":"actors","hosts":1,"bodies":3,"brains":3}"#,
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(
			actors.body,
			AgentEventBody::Actors {
				pending_support: 0,
				min_y: None,
				max_y: None,
				ground_at_min_y: None,
				min_body: None,
				min_body_downed: None,
				..
			}
		));
		Ok(())
	}

	#[test]
	fn diagnostic_events_roundtrip_without_hot_path_tracy_messages() -> anyhow::Result<()> {
		let actor = AgentDiagnosticActor {
			entity: u64::MAX.to_string(),
			player: true,
			name: Some("player".into()),
			position: Some([976.6, -192.2, 1819.9]),
		};
		for body in [
			AgentEventBody::DamageApplied {
				target: actor.clone(),
				source: None,
				amount: 250.0,
				applied: 100.0,
				remaining: 0.0,
				point: [1.0, 2.0, 3.0],
			},
			AgentEventBody::Died { actor: actor.clone(), source: None, point: [1.0, 2.0, 3.0] },
			AgentEventBody::WeaponFired { shooter: actor.clone(), recoil: 0.0, age: 0.1 },
			AgentEventBody::FixtureHealth {
				actor: actor.clone(),
				before: [100.0, 100.0],
				after: [1_000_000.0; 2],
			},
			AgentEventBody::FixtureView {
				actor,
				camera: "9007199254740993".into(),
				before: [0.1, 0.2],
				after: [0.0, -0.12],
			},
			AgentEventBody::Diagnostics { enabled: false },
		] {
			let event = AgentEvent { t: 4.0, frame: 240, body };
			let line = event.to_line().map_err(anyhow::Error::msg)?;
			assert_eq!(AgentEvent::parse_line(&line).map_err(anyhow::Error::msg)?, event);
			assert_eq!(event.tracy_message(), None);
		}
		Ok(())
	}

	#[test]
	fn picker_roundtrip_and_tracy_message() -> anyhow::Result<()> {
		let event = AgentEvent { t: 8.0, frame: 480, body: AgentEventBody::Picker { open: true } };
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"picker""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		assert_eq!(event.tracy_message().as_deref(), Some("agent.picker open=true"));
		Ok(())
	}

	#[test]
	fn presentation_roundtrip() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 12.0,
			frame: 700,
			body: AgentEventBody::Presentation {
				action: "resize".into(),
				window: [1920, 1052],
				target: [719, 394],
			},
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"presentation""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
		Ok(())
	}

	#[test]
	fn menu_events_roundtrip() -> anyhow::Result<()> {
		for body in [
			AgentEventBody::Screen { name: "in_game".into() },
			AgentEventBody::Modal { name: Some("discard".into()) },
			AgentEventBody::Modal { name: None },
			AgentEventBody::Focus {
				screen: "home".into(),
				row: 2,
				label: Some("Characters".into()),
			},
			AgentEventBody::Flow { flow: "world".into(), pause: "menu".into() },
			AgentEventBody::Invariant { detail: "multiple MenuScreen (2)".into() },
			AgentEventBody::Cycle { category: "skill_maps".into() },
			AgentEventBody::Equipped { category: "weapons".into(), item: Some("reltor".into()) },
		] {
			let event = AgentEvent { t: 1.0, frame: 12, body };
			let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
			assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, event);
			assert!(event.tracy_message().is_some(), "{line}");
		}
		Ok(())
	}

	#[test]
	fn tanz_roundtrip_and_tracy_message() -> anyhow::Result<()> {
		let mode = AgentEvent {
			t: 4.0,
			frame: 240,
			body: AgentEventBody::Tanz { mode: "tanzer".into(), id: None },
		};
		let line = mode.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains(r#""kind":"tanz""#));
		assert_eq!(AgentEvent::parse_line(&line).map_err(|e| anyhow::anyhow!("{e}"))?, mode);
		assert_eq!(mode.tracy_message().as_deref(), Some("agent.tanz mode=tanzer"));
		let active = AgentEvent {
			t: 4.2,
			frame: 252,
			body: AgentEventBody::Tanz { mode: "tanzer".into(), id: Some("left_strafe".into()) },
		};
		assert_eq!(
			active.tracy_message().as_deref(),
			Some("agent.tanz mode=tanzer id=left_strafe")
		);
		Ok(())
	}

	#[test]
	fn command_failed_includes_detail() -> anyhow::Result<()> {
		let event = AgentEvent {
			t: 2.0,
			frame: 10,
			body: AgentEventBody::Command {
				id: 3,
				status: CommandStatus::Failed,
				cmd: "goto 10.0,10.0".into(),
				detail: Some("timeout".into()),
			},
		};
		let line = event.to_line().map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(line.contains("timeout"));
		assert!(event.tracy_message().is_some());
		Ok(())
	}
}

#[cfg(test)]
mod camera_snapshot_tests {
	use crate::{AgentCameraProjection, AgentCameraSnapshot, AgentEvent, AgentEventBody};
	#[test]
	fn actual_camera_snapshot_roundtrip_is_explicit_about_propagation() -> Result<(), String> {
		let event = AgentEvent {
			t: 10.,
			frame: 600,
			body: AgentEventBody::CameraSnapshot {
				request_id: 7,
				applied_local: true,
				global_pose_pre_apply: true,
				state: AgentCameraSnapshot {
					entity: "18446744073709551615".into(),
					local_translation: [1., 2., 3.],
					local_rotation: [0., 0., 0., 1.],
					global_translation: [9., 8., 7.],
					global_rotation: [0., 0., 0., 1.],
					controller_yaw: 0.,
					controller_pitch: 0.,
					projection: AgentCameraProjection::Perspective {
						fov: 1.,
						near: 0.1,
						far: 8000.,
						aspect_ratio: 16. / 9.,
					},
				},
			},
		};
		assert_eq!(AgentEvent::parse_line(&event.to_line()?)?, event);
		assert_eq!(event.tracy_message(), None);
		Ok(())
	}
}
