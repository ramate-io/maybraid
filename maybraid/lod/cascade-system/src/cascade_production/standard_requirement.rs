//! Default [`RequirementBuilder`] (`Visible` / `Remove` trait defaults).

use bevy::prelude::*;

use super::RequirementBuilder;

#[derive(Component, Clone, Copy, Default)]
pub struct StandardRequirement;

impl RequirementBuilder for StandardRequirement {}
