//! Near fireball: overlapping mesh lobes plus optional flame cards.

use bevy::prelude::*;
use bevy_hanabi::prelude::{
	AccelModifier, Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier,
	EffectAsset, ExprWriter, FlipbookModifier, ImageSampleMapping, LinearDragModifier, OrientMode,
	OrientModifier, ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier,
	ShapeDimension, SimulationSpace, SpawnerSettings,
};

use crate::assets::FlipbookAsset;
use crate::composition::{
	EffectDefinition, EffectLayer, LobeKind, LobeSpec, MeshPart, ParticlePart, FIREBALL,
};
use crate::flipbook::update_sprite_index;
use crate::particles::{add_instance_properties, init_radial_velocity, update_scaled_size3};

pub const FIREBALL_COUNT: f32 = 6.0;
pub const FIREBALL_CAPACITY: u32 = 16;
pub const FIREBALL_LIFE_MIN: f32 = 0.35;
pub const FIREBALL_LIFE_MAX: f32 = 0.55;

pub fn compile_mesh(mesh: Handle<Mesh>) -> MeshPart {
	MeshPart::duration_from_lobes(
		FIREBALL,
		mesh,
		LobeKind::Fire,
		vec![
			lobe(Vec3::new(0.00, 0.05, 0.00), 0.58, 0.88, 0.08, 0.55),
			lobe(Vec3::new(0.16, 0.10, 0.05), 0.40, 0.72, 0.12, 0.50),
			lobe(Vec3::new(-0.14, 0.08, 0.10), 0.38, 0.74, 0.10, 0.52),
			lobe(Vec3::new(0.04, 0.14, -0.16), 0.36, 0.70, 0.14, 0.48),
			lobe(Vec3::new(-0.08, -0.02, -0.12), 0.34, 0.62, 0.06, 0.46),
			lobe(Vec3::new(0.10, -0.04, 0.14), 0.32, 0.64, 0.07, 0.47),
		],
	)
}

fn lobe(offset: Vec3, scale: f32, expand: f32, rise: f32, duration: f32) -> LobeSpec {
	LobeSpec { offset, scale: Vec3::splat(scale), expand, rise, duration }
}

pub fn compile_wisps(effects: &mut Assets<EffectAsset>, fire: &FlipbookAsset) -> ParticlePart {
	let writer = ExprWriter::new();
	let props = add_instance_properties(&writer);
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(Vec3::ZERO).expr(),
		radius: (writer.lit(0.08) * props.scale.clone()).expr(),
		dimension: ShapeDimension::Volume,
	};
	let init_vel = init_radial_velocity(&writer, &props.scale, 1.2, 2.4);
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(FIREBALL_LIFE_MIN).uniform(writer.lit(FIREBALL_LIFE_MAX)).expr(),
	);
	let init_sprite = SetAttributeModifier::new(Attribute::SPRITE_INDEX, writer.lit(0i32).expr());
	let init_color = SetAttributeModifier::new(Attribute::HDR_COLOR, props.tint.expr());
	let update_drag = LinearDragModifier::new(writer.lit(1.4).expr());
	let update_accel =
		AccelModifier::new((writer.lit(Vec3::new(0.0, 0.6, 0.0)) * props.scale.clone()).expr());
	let update_size =
		update_scaled_size3(&writer, &props.scale, Vec3::splat(0.32), Vec3::splat(0.72));
	let update_sprite = update_sprite_index(&writer, fire);
	let texture_slot = writer.lit(0u32).expr();

	let mut color = bevy_hanabi::Gradient::new();
	color.add_key(0.0, Vec4::new(1.05, 0.68, 0.24, 0.95));
	color.add_key(0.4, Vec4::new(0.92, 0.30, 0.06, 0.72));
	color.add_key(1.0, Vec4::new(0.16, 0.04, 0.01, 0.0));

	let mut module = writer.finish();
	module.add_texture_slot("fire");

	let effect = effects.add(
		EffectAsset::new(FIREBALL_CAPACITY, SpawnerSettings::once(FIREBALL_COUNT.into()), module)
			.with_name("vfx-fireball-wisps")
			.with_simulation_space(SimulationSpace::Global)
			.with_alpha_mode(bevy_hanabi::AlphaMode::Blend)
			.init(init_pos)
			.init(init_vel)
			.init(init_age)
			.init(init_lifetime)
			.init(init_sprite)
			.init(init_color)
			.update(update_drag)
			.update(update_accel)
			.update(update_size)
			.update(update_sprite)
			.render(ParticleTextureModifier {
				texture_slot,
				sample_mapping: ImageSampleMapping::Modulate,
			})
			.render(FlipbookModifier { sprite_grid_size: fire.grid })
			.render(ColorOverLifetimeModifier {
				gradient: color,
				blend: ColorBlendMode::Modulate,
				mask: ColorBlendMask::RGBA,
			})
			.render(OrientModifier::new(OrientMode::FaceCameraPosition)),
	);

	ParticlePart {
		name: format!("{FIREBALL}-wisps"),
		effect,
		images: vec![fire.image.clone()],
		count: FIREBALL_COUNT,
		capacity: FIREBALL_CAPACITY,
		max_lifetime: FIREBALL_LIFE_MAX,
	}
}

pub fn definition(mesh: MeshPart, wisps: ParticlePart) -> EffectDefinition {
	EffectDefinition::new(FIREBALL, [EffectLayer::mesh(mesh), EffectLayer::particle(wisps)])
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::assets::fire_flipbook;
	use crate::composition::EffectPart;

	#[test]
	fn fireball_owns_mesh_core_and_wisps() {
		let mut images = Assets::<Image>::default();
		let mut effects = Assets::<EffectAsset>::default();
		let wisps = compile_wisps(&mut effects, &fire_flipbook(&mut images));
		let def = definition(compile_mesh(Handle::default()), wisps);
		assert_eq!(def.layers.len(), 2);
		assert!(matches!(def.layers[0].part, EffectPart::Mesh(_)));
		assert_eq!(
			effects
				.get(match &def.layers[1].part {
					EffectPart::Particle(part) => &part.effect,
					_ => panic!("expected wisps"),
				})
				.unwrap()
				.simulation_space,
			SimulationSpace::Global
		);
	}
}
