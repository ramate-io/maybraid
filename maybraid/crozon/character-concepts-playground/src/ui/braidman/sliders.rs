use bevy::prelude::*;
use crozon_characters::species::braidman::{sliders::BraidmanSliders, BraidmanConfig};

use crate::ui::{button, text, SLIDER_STEP, TILT_STEP_DEG};

use super::CreatorUiAction;

/// Apply a slider button action to the active Braidman config. Returns true when handled.
pub fn apply_action(sliders: &mut BraidmanSliders, action: CreatorUiAction) -> bool {
	match action {
		CreatorUiAction::ShoulderWidth(delta) => {
			*sliders = sliders.with_shoulder_width(sliders.shoulder_width + delta);
		}
		CreatorUiAction::HipWidth(delta) => {
			*sliders = sliders.with_hip_width(sliders.hip_width + delta);
		}
		CreatorUiAction::ChestThickness(delta) => {
			*sliders = sliders.with_chest_thickness(sliders.chest_thickness + delta);
		}
		CreatorUiAction::HipThickness(delta) => {
			*sliders = sliders.with_hip_thickness(sliders.hip_thickness + delta);
		}
		CreatorUiAction::LegThickness(delta) => {
			*sliders = sliders.with_leg_thickness(sliders.leg_thickness + delta);
		}
		CreatorUiAction::ButtocksThickness(delta) => {
			*sliders = sliders.with_buttocks_thickness(sliders.buttocks_thickness + delta);
		}
		CreatorUiAction::WaistThickness(delta) => {
			*sliders = sliders.with_waist_thickness(sliders.waist_thickness + delta);
		}
		CreatorUiAction::LowerTrunkThickness(delta) => {
			*sliders = sliders.with_lower_trunk_thickness(sliders.lower_trunk_thickness + delta);
		}
		CreatorUiAction::ArmLength(delta) => {
			*sliders = sliders.with_arm_length(sliders.arm_length + delta);
		}
		CreatorUiAction::ArmThickness(delta) => {
			*sliders = sliders.with_arm_thickness(sliders.arm_thickness + delta);
		}
		CreatorUiAction::LegLength(delta) => {
			*sliders = sliders.with_leg_length(sliders.leg_length + delta);
		}
		CreatorUiAction::EyeWidth(delta) => {
			*sliders = sliders.with_eye_width(sliders.eye_width + delta);
		}
		CreatorUiAction::EyeHeight(delta) => {
			*sliders = sliders.with_eye_height(sliders.eye_height + delta);
		}
		CreatorUiAction::EyeTilt(delta) => {
			*sliders = sliders.with_eye_tilt(sliders.eye_tilt + delta);
		}
		CreatorUiAction::NoseWidth(delta) => {
			*sliders = sliders.with_nose_width(sliders.nose_width + delta);
		}
		CreatorUiAction::NoseHeight(delta) => {
			*sliders = sliders.with_nose_height(sliders.nose_height + delta);
		}
		CreatorUiAction::MouthWidth(delta) => {
			*sliders = sliders.with_mouth_width(sliders.mouth_width + delta);
		}
		CreatorUiAction::MouthHeight(delta) => {
			*sliders = sliders.with_mouth_height(sliders.mouth_height + delta);
		}
		CreatorUiAction::EarWidth(delta) => {
			*sliders = sliders.with_ear_width(sliders.ear_width + delta);
		}
		CreatorUiAction::EarHeight(delta) => {
			*sliders = sliders.with_ear_height(sliders.ear_height + delta);
		}
		_ => return false,
	}
	true
}

pub fn spawn_body(parent: &mut ChildSpawnerCommands, braidman: &BraidmanConfig) {
	let sliders = &braidman.sliders;
	row(parent, "Shoulder Width", sliders.shoulder_width, CreatorUiAction::ShoulderWidth);
	row(parent, "Hip Width", sliders.hip_width, CreatorUiAction::HipWidth);
	row(parent, "Chest Thickness", sliders.chest_thickness, CreatorUiAction::ChestThickness);
	row(parent, "Hip Thickness", sliders.hip_thickness, CreatorUiAction::HipThickness);
	row(parent, "Leg Thickness", sliders.leg_thickness, CreatorUiAction::LegThickness);
	row(
		parent,
		"Buttocks Thickness",
		sliders.buttocks_thickness,
		CreatorUiAction::ButtocksThickness,
	);
	row(parent, "Waist Thickness", sliders.waist_thickness, CreatorUiAction::WaistThickness);
	row(
		parent,
		"Lower Trunk Thickness",
		sliders.lower_trunk_thickness,
		CreatorUiAction::LowerTrunkThickness,
	);
	row(parent, "Arm Length", sliders.arm_length, CreatorUiAction::ArmLength);
	row(parent, "Arm Thickness", sliders.arm_thickness, CreatorUiAction::ArmThickness);
	row(parent, "Leg Length", sliders.leg_length, CreatorUiAction::LegLength);
}

pub fn spawn_eyes(parent: &mut ChildSpawnerCommands, braidman: &BraidmanConfig) {
	let sliders = &braidman.sliders;
	row(parent, "Eye Width", sliders.eye_width, CreatorUiAction::EyeWidth);
	row(parent, "Eye Height", sliders.eye_height, CreatorUiAction::EyeHeight);
	tilt_row(parent, "Eye Tilt", sliders.eye_tilt, CreatorUiAction::EyeTilt);
}

pub fn spawn_nose(parent: &mut ChildSpawnerCommands, braidman: &BraidmanConfig) {
	let sliders = &braidman.sliders;
	row(parent, "Nose Width", sliders.nose_width, CreatorUiAction::NoseWidth);
	row(parent, "Nose Height", sliders.nose_height, CreatorUiAction::NoseHeight);
}

pub fn spawn_mouth(parent: &mut ChildSpawnerCommands, braidman: &BraidmanConfig) {
	let sliders = &braidman.sliders;
	row(parent, "Mouth Width", sliders.mouth_width, CreatorUiAction::MouthWidth);
	row(parent, "Mouth Height", sliders.mouth_height, CreatorUiAction::MouthHeight);
}

pub fn spawn_ears(parent: &mut ChildSpawnerCommands, braidman: &BraidmanConfig) {
	let sliders = &braidman.sliders;
	row(parent, "Ear Width", sliders.ear_width, CreatorUiAction::EarWidth);
	row(parent, "Ear Height", sliders.ear_height, CreatorUiAction::EarHeight);
}

fn row(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value: f32,
	action: fn(f32) -> CreatorUiAction,
) {
	parent.spawn((crate::ui::row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		button(row, "-", action(-SLIDER_STEP), false);
		text(row, &format!("{value:.2}"), 11.0, Color::srgb(0.85, 0.95, 1.0));
		button(row, "+", action(SLIDER_STEP), false);
	});
}

fn tilt_row(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value_deg: f32,
	action: fn(f32) -> CreatorUiAction,
) {
	parent.spawn((crate::ui::row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		button(row, "-", action(-TILT_STEP_DEG), false);
		text(row, &format!("{value_deg:.1}°"), 11.0, Color::srgb(0.85, 0.95, 1.0));
		button(row, "+", action(TILT_STEP_DEG), false);
	});
}
