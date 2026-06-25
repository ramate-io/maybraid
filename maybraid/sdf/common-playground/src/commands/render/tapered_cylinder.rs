//! Tapered-cylinder render subcommand types (`render tapered-cylinder …`).

pub mod plugin;

use super::RenderHelper;
use sdf_common::TaperedCylinder;

pub type TaperedCylinderHelper = RenderHelper<TaperedCylinder>;
