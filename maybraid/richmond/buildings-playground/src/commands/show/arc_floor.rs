//! `/show arc-floor` — one circular storey shell with optional openings.
//!
//! With no `--opening` flags, seeds a few **world AABB** plan openings (not `t=`)
//! so you can inspect how Layer 1/2 map coarse voids onto wall / slab geometry.

use bevy::prelude::*;
use clap::Args;
use richmond_buildings::{ArcFloor as ArcFloorShell, OpeningLabel};

use super::opening::{parse_opening_arg, ArcOpeningContext, OpeningArg, PreviewOpening};
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ArcFloor {
	#[arg(long, default_value_t = 4.0)]
	pub radius: f32,
	#[arg(long, default_value_t = 3.0)]
	pub storey_height: f32,
	/// Emit a solid floor slab (openings may still cut / remove it).
	#[arg(long, default_value_t = true)]
	pub floor: bool,
	/// Emit a solid ceiling slab (openings may still cut / remove it).
	#[arg(long, default_value_t = false)]
	pub ceiling: bool,
	/// Opening plan entries. Repeatable. When omitted, a demo AABB plan is used.
	///
	/// Formats: `id:label:minx,miny,minz:maxx,maxy,maxz` or `id:label:t=0.5`.
	#[arg(long = "opening", value_name = "SPEC", value_parser = parse_opening_arg, action = clap::ArgAction::Append)]
	pub openings: Vec<OpeningArg>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ArcFloor {
	pub fn into_preview(self) -> Result<(PreviewSubject, Transform), String> {
		let ctx = ArcOpeningContext {
			center_xz: Vec3::ZERO,
			radius: self.radius,
			storey_height: self.storey_height,
		};
		let openings = if self.openings.is_empty() {
			default_aabb_openings(ctx)
		} else {
			self.openings
				.into_iter()
				.map(|a| a.resolve_aabb(Some(ctx)))
				.collect::<Result<_, _>>()?
		};
		Ok((
			PreviewSubject::ArcFloor {
				radius: self.radius,
				storey_height: self.storey_height,
				floor: self.floor,
				ceiling: self.ceiling,
				openings,
			},
			self.transform.transform(),
		))
	}
}

/// Demo plan: explicit world AABBs only (no `t=`), including one loose / off-ring void.
fn default_aabb_openings(ctx: ArcOpeningContext) -> Vec<PreviewOpening> {
	// Tight AABB hugging the ring at t=0.5 (+X door).
	let east = preview_from_plan_t("east_door", OpeningLabel::Passage, ctx, 0.5);
	// Another tight AABB at t=0 (−Z / north-ish).
	let north = preview_from_plan_t("north_window", OpeningLabel::Aperture, ctx, 0.0);
	// Loose chunky AABB near t=0.25: oversized and pulled outward so Layer 1
	// still has to recover a ring locus from a coarse void.
	let r = ctx.radius;
	let loose_center = Vec3::new(r * 0.85, ctx.storey_height * 0.35, -r * 0.85);
	let loose = PreviewOpening {
		id: "loose_se".into(),
		label: OpeningLabel::Passage,
		min: loose_center - Vec3::new(1.4, 0.1, 1.4),
		max: loose_center + Vec3::new(1.4, ctx.storey_height * 0.75, 1.4),
	};
	vec![east, north, loose]
}

fn preview_from_plan_t(
	id: &str,
	label: OpeningLabel,
	ctx: ArcOpeningContext,
	t: f32,
) -> PreviewOpening {
	let (_id, opening) = ArcFloorShell::plan_opening_at_t(
		id,
		label.clone(),
		ctx.center_xz,
		ctx.radius,
		ctx.storey_height,
		t,
	);
	PreviewOpening {
		id: id.to_string(),
		label,
		min: Vec3::from(opening.bounds.min),
		max: Vec3::from(opening.bounds.max),
	}
}
