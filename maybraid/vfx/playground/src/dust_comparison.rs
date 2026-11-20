//! Matched sequential scenes: never share an interaction owner across two comparisons.
pub(crate) mod profiling;
use crate::{
	commands::{LoopEnabled, PendingVfxShows, VfxShowRequest},
	show::LastVfxShow,
};
use bevy::prelude::*;
#[derive(Resource)]
pub(crate) struct Comparison {
	pub original: VfxShowRequest,
	pub second: &'static str,
	pub phase: u8,
	wait: f32,
}
pub(crate) fn pair(name: &str) -> Option<(&'static str, &'static str)> {
	Some(match name {
		"landing-compare" => ("cartoon-landing", "combined-landing"),
		"wave-compare" => ("cartoon-wave", "combined-wave"),
		"landing-contact-compare" => ("cartoon-landing-contact", "combined-landing-contact"),
		"wave-contact-compare" => ("cartoon-wave-contact", "combined-wave-contact"),
		_ => return None,
	})
}
pub(crate) fn selection(
	commands: &mut Commands,
	request: &mut VfxShowRequest,
	current: Option<&Comparison>,
) -> VfxShowRequest {
	let original = request.clone();
	if let Some((first, second)) = pair(&request.effect) {
		request.effect = first.into();
		commands.insert_resource(Comparison {
			original: original.clone(),
			second,
			phase: 0,
			wait: 0.,
		});
		return original;
	}
	if let Some(c) = current {
		if request.effect == c.second && c.phase == 1 {
			return c.original.clone();
		}
	}
	commands.remove_resource::<Comparison>();
	original
}
pub(crate) fn tick(
	mut commands: Commands,
	time: Res<Time<Virtual>>,
	comparison: Option<ResMut<Comparison>>,
	roots: Query<&maybraid_vfx::VfxInstance>,
	mut pending: ResMut<PendingVfxShows>,
	mut looping: ResMut<LoopEnabled>,
	last: Res<LastVfxShow>,
) {
	let Some(mut c) = comparison else {
		return;
	};
	if last.request.as_ref().is_none_or(|r| r.effect != c.original.effect) {
		return;
	}
	looping.0 = false;
	if time.is_paused() || c.phase != 0 || !roots.is_empty() {
		return;
	}
	c.wait += time.delta_secs();
	if c.wait < 0.45 {
		return;
	}
	c.phase = 1;
	let mut next = c.original.clone();
	next.effect = c.second.into();
	pending.0.push(next);
	// New event starts from time zero with the same scene input, seed and camera.
	commands.queue(|w: &mut World| {
		w.resource_mut::<maybraid_vfx::parts::combined::interaction::InteractionScene>()
			.rebase_manual();
	});
}
#[cfg(test)]
mod tests {
	#[test]
	fn pairs_are_behavior_specific() {
		for n in ["landing", "wave"] {
			for contact in ["", "-contact"] {
				let key = format!("{n}{contact}-compare");
				let (a, b) = super::pair(&key).unwrap_or(("missing", "missing"));
				assert_eq!(a, format!("cartoon-{n}{contact}"));
				assert_eq!(b, format!("combined-{n}{contact}"));
			}
		}
	}
}

#[derive(Clone, clap::Args)]
pub struct Appearance {
	#[arg(long)]
	pub body_concentration: Option<f32>,
	#[arg(long)]
	pub fine_concentration: Option<f32>,
	#[arg(long)]
	pub cloud_rate: Option<f64>,
	#[arg(long)]
	pub grain_rate: Option<f64>,
}
impl Appearance {
	pub(crate) fn apply(self, w: &mut World) {
		fn update(a: &Appearance, o: &mut maybraid_vfx::ChoreographyOptions) {
			if let Some(v) = a.body_concentration.filter(|v| v.is_finite()) {
				o.body_concentration = v.clamp(0., 3.);
			}
			if let Some(v) = a.fine_concentration.filter(|v| v.is_finite()) {
				o.fine_concentration = v.clamp(0., 3.);
			}
			if let Some(v) = a.cloud_rate.filter(|v| v.is_finite()) {
				o.cloud_rate = v.clamp(0., 30.);
			}
			if let Some(v) = a.grain_rate.filter(|v| v.is_finite()) {
				o.grain_rate = v.clamp(0., 15.);
			}
		}
		if let Some(mut l) = w.get_resource_mut::<maybraid_vfx::VfxLibrary>() {
			update(&self, &mut l.combined_landing.layer.part.options);
			update(&self, &mut l.combined_landing_contact.layer.part.options);
			update(&self, &mut l.combined_wave_contact.layer.part.options);
		}
		for mut o in w
			.query::<&mut maybraid_vfx::parts::combined_choreography::OpticalChoreography>()
			.iter_mut(w)
		{
			update(&self, &mut o.part.options);
		}
	}
}
