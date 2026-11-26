//! Inspect the most recent preview, retaining its final state after cleanup.
use bevy::prelude::*;
use maybraid_vfx::membership::VfxMemberOf;
use maybraid_vfx::spawn::{VfxLayerLife, VfxPendingLayers};
use maybraid_vfx::{
	EffectLayer, EffectPart, PackagedFlipbook, VfxInstance, VfxLayerInfo, VfxLoadFailure,
};

#[derive(Clone, Debug)]
pub(crate) struct LayerRow {
	pub label: String,
	pub path: Option<String>,
	pub runtime_index: Option<usize>,
	pub status: String,
}

impl LayerRow {
	pub fn new(layer: &EffectLayer, runtime_index: Option<usize>) -> Self {
		Self {
			label: layer.label().to_owned(),
			path: match &layer.part {
				EffectPart::Flipbook(part) => Some(part.flipbook.path.clone()),
				_ => None,
			},
			runtime_index,
			status: if runtime_index.is_some() { "pending" } else { "muted" }.into(),
		}
	}
}

#[derive(Resource, Default)]
pub(crate) struct PreviewInspector {
	pub root: Option<Entity>,
	pub layers: Vec<LayerRow>,
	pub failure: Option<String>,
}

impl PreviewInspector {
	pub fn summary(&self) -> String {
		if let Some(failure) = &self.failure {
			return failure.clone();
		}
		self.layers
			.iter()
			.enumerate()
			.map(|(index, row)| format!("{index}: {} — {}", row.label, row.status))
			.collect::<Vec<_>>()
			.join("\n")
	}
}

pub(crate) fn inspect_preview(
	mut inspector: ResMut<PreviewInspector>,
	mut failures: MessageReader<VfxLoadFailure>,
	instances: Query<(&VfxInstance, &VfxPendingLayers)>,
	hosts: Query<(Entity, &VfxMemberOf, &VfxLayerInfo)>,
	nodes: Query<(Option<&VfxLayerLife>, Option<&Children>)>,
	server: Res<AssetServer>,
	assets: Res<Assets<PackagedFlipbook>>,
	mut looping: ResMut<crate::commands::LoopEnabled>,
) {
	for failure in failures.read() {
		if inspector.root == Some(failure.root) {
			inspector.failure = Some(failure.to_string());
			// Keep the actionable error on screen instead of retrying a broken asset forever.
			looping.0 = false;
		}
	}
	if inspector.failure.is_some() {
		return;
	}
	let Some(root) = inspector.root else {
		return;
	};
	let instance = instances.get(root).ok();
	for row in &mut inspector.layers {
		let Some(index) = row.runtime_index else {
			continue;
		};
		let Some((instance, pending)) = instance else {
			row.status = "finished".into();
			continue;
		};
		if let Some((_, layer)) = pending.layers.iter().find(|(i, _)| *i == index) {
			row.status = format!("pending (starts at {:.2}s)", layer.delay);
			continue;
		}
		let Some((host, _, _)) = hosts
			.iter()
			.find(|(_, member, info)| member.root() == root && info.index == index)
		else {
			row.status = "finished".into();
			continue;
		};
		let mut todo = vec![host];
		let mut waiting = !instance.armed;
		let mut age: f32 = 0.;
		let mut duration: f32 = 0.;
		while let Some(entity) = todo.pop() {
			if let Ok((life, children)) = nodes.get(entity) {
				if let Some(life) = life {
					waiting |= life.waiting_for_emitter;
					age = age.max(life.age);
					duration = duration.max(life.duration);
				}
				if let Some(children) = children {
					todo.extend(children.iter());
				}
			}
		}
		row.status = if waiting {
			"loading".into()
		} else if age + 1e-3 >= duration {
			"finished".into()
		} else {
			format!("playing {age:.3}/{duration:.3}s")
		};
		if !waiting {
			if let Some(asset) = row
				.path
				.as_ref()
				.and_then(|path| server.get_handle::<PackagedFlipbook>(path.clone()))
				.and_then(|handle| assets.get(&handle))
			{
				let clock = asset.manifest.clock_at(age);
				row.status.push_str(&format!(
					" · frame {:.0}→{:.0} ({:.2})",
					clock.frame_0, clock.frame_1, clock.fract
				));
			}
		}
	}
}
