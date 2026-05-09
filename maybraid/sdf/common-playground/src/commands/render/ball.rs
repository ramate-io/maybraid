//! Ball render subcommand (`render ball …`).

pub mod plugin;

use super::RenderHelper;
use sdf_common::Ball;

pub type BallHelper = RenderHelper<Ball>;
