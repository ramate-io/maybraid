use std::marker::PhantomData;

/// Blend two animations by weight: 0 = `from`, 1 = `to`.
#[derive(Debug, Clone)]
pub struct Mix<A, B, Rig> {
	pub from: A,
	pub to: B,
	pub weight: f32,
	_rig: PhantomData<Rig>,
}

impl<A, B, Rig> Mix<A, B, Rig> {
	pub fn new(from: A, to: B, weight: f32) -> Self {
		Self { from, to, weight: weight.clamp(0.0, 1.0), _rig: PhantomData }
	}
}

/// Like [`Mix`], but eases the blend weight with smoothstep.
#[derive(Debug, Clone)]
pub struct Smooth<A, B, Rig> {
	pub from: A,
	pub to: B,
	pub weight: f32,
	_rig: PhantomData<Rig>,
}

impl<A, B, Rig> Smooth<A, B, Rig> {
	pub fn new(from: A, to: B, weight: f32) -> Self {
		Self { from, to, weight: weight.clamp(0.0, 1.0), _rig: PhantomData }
	}

	pub fn into_mix(self) -> Mix<A, B, Rig> {
		let t = smoothstep(self.weight);
		Mix::new(self.from, self.to, t)
	}
}

pub fn smoothstep(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn smoothstep_endpoints() -> anyhow::Result<()> {
		assert!(smoothstep(0.0).abs() < 1e-5);
		assert!((smoothstep(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn mix_clamps_weight() {
		let mix = Mix::<(), (), ()>::new((), (), 1.5);
		assert_eq!(mix.weight, 1.0);
	}
}
