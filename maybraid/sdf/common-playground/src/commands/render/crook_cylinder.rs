//! Crook-cylinder render subcommand types (`render crook-cylinder …`).

pub mod plugin;

use super::RenderHelper;
use sdf_common::CrookCylinder;

pub type CrookCylinderHelper = RenderHelper<CrookCylinder>;
