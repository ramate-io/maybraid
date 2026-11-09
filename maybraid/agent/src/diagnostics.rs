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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiagnosticCommand {
	Enabled(bool),
	Health(f32),
	View { yaw: f32, pitch: f32 },
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
			_ => Err("expected diagnostics on | off | health AMOUNT | view YAW PITCH".into()),
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
