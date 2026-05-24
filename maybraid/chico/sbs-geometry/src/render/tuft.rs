//! Tuft placement helpers for ball-stick graphs.

use crate::{BallStickChain, BallStickNode, Hysteresis};
use bevy::prelude::*;
use render_item::{CascadeChunk, RenderItem};
use std::marker::PhantomData;

/// Stable mixing key from graph index and node position.
pub fn tuft_mix_seed(node_idx: usize, position: Vec3) -> u32 {
	(node_idx as u32)
		.wrapping_mul(0x9E37_79B9)
		.wrapping_add(position.x.to_bits())
		.wrapping_add(position.y.to_bits().rotate_left(3))
		.wrapping_add(position.z.to_bits().rotate_left(7))
}

/// RFC Liam's Conifer: **2..=3** tufts per joint, deterministic from [`tuft_mix_seed`].
pub fn tufts_per_joint(node_idx: usize, position: Vec3) -> usize {
	2 + (tuft_mix_seed(node_idx, position) % 2) as usize
}

/// Dominant branch direction at a joint: average of outgoing child edges, else incoming parent edge.
pub fn joint_branch_axis<H: Hysteresis>(chain: &BallStickChain<H>, node_idx: usize) -> Vec3 {
	let node = &chain.nodes[node_idx];
	let children = &chain.children[node_idx];
	if !children.is_empty() {
		let mut sum = Vec3::ZERO;
		for &child_idx in children {
			let ray = chain.nodes[child_idx].position - node.position;
			if ray.length_squared() > 1e-10 {
				sum += ray.normalize();
			}
		}
		if sum.length_squared() > 1e-10 {
			return sum.normalize();
		}
	}

	for (parent_idx, child_list) in chain.children.iter().enumerate() {
		if child_list.contains(&node_idx) {
			let ray = node.position - chain.nodes[parent_idx].position;
			if ray.length_squared() > 1e-10 {
				return ray.normalize();
			}
		}
	}

	Vec3::Y
}

/// Mild upward spread: blend branch axis toward world up, then fan instances around the axis.
pub fn tuft_transforms_at_joint(
	node: &BallStickNode,
	branch_axis: Vec3,
	world_scale: f32,
	node_idx: usize,
) -> Vec<Transform> {
	let count = tufts_per_joint(node_idx, node.position);
	let seed = tuft_mix_seed(node_idx, node.position);
	let base_dir = (branch_axis + Vec3::Y * 0.12).normalize_or_zero();
	let base_dir = if base_dir.length_squared() > 1e-10 { base_dir } else { Vec3::Y };

	let mut out = Vec::with_capacity(count);
	for i in 0..count {
		let t = if count == 1 {
			0.0
		} else {
			(i as f32 + 0.5) / count as f32 - 0.5
		};
		let twist = (seed.wrapping_add(i as u32) as f32) * 0.31;
		let spread = Vec3::new((twist + t * 0.9).cos() * 0.14, 0.0, (twist + t * 0.9).sin() * 0.14);
		let dir = (base_dir + spread).normalize_or_zero();
		let rotation = Quat::from_rotation_arc(Vec3::Y, dir);
		out.push(Transform {
			translation: node.position,
			rotation,
			scale: Vec3::splat(world_scale),
		});
	}
	out
}

pub trait TuftRenderRule<R: RenderItem, H: Hysteresis>: Clone {
	fn tuft_placements_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		_hysteresis: &H,
		chain: &BallStickChain<H>,
	) -> Vec<(R, Transform)>;
}

#[derive(Clone)]
pub struct TuftRenderHelper<Item: RenderItem, Rule: TuftRenderRule<Item, H>, H: Hysteresis> {
	rule: Rule,
	chain: BallStickChain<H>,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, Rule: TuftRenderRule<Item, H>, H: Hysteresis>
	TuftRenderHelper<Item, Rule, H>
{
	pub fn new(chain: BallStickChain<H>, rule: Rule) -> Self {
		Self { chain, rule, __marker: PhantomData }
	}

	pub fn render_tufts(&self) -> Vec<(Item, Transform)> {
		self.chain
			.nodes_with_hysteresis_enumerated()
			.flat_map(|(node_idx, node, h)| {
				self.rule.tuft_placements_for(node_idx, node, h, &self.chain)
			})
			.collect()
	}
}

impl<Item: RenderItem, Rule: TuftRenderRule<Item, H>, H: Hysteresis> RenderItem
	for TuftRenderHelper<Item, Rule, H>
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.render_tufts()
			.into_iter()
			.flat_map(|(item, inner)| {
				item.spawn_render_items(commands, cascade_chunk, inner.mul_transform(transform))
			})
			.collect()
	}
}
