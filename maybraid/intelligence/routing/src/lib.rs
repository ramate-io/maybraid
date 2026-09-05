//! Hierarchical long-range routing. Bands are per-user policy, not a crate ladder.
//!
//! A [`RoutingIntelligenceUser`] commits coarse-to-fine corridors. Callers
//! either hand the current fine hop to [`movement_intelligence::MovementObjective::Reach`]
//! ([`write_route_objectives`]) or consume [`RoutingIntelligenceUser::current_hop`]
//! themselves (journeying mob hosts). This crate does not write
//! [`player::MoveWish`].

mod avian;
mod band;
mod plan;
mod plugin;
mod probe;
mod user;

pub use avian::AvianRouteProbe;
pub use band::{RoutingBand, RoutingSettings};
pub use plan::{plan_route, FailedEdge, LayerPlan, RoutePlan};
pub use plugin::{plan_routes, write_route_objectives, RoutingPlugin, RoutingSystems};
pub use probe::RouteProbe;
pub use user::RoutingIntelligenceUser;
