//! Jersey terrain stamps ([RFC-105 §3.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)).
//!
//! Pure stamp / modulation construction — no LOD `GenerationScheme` wiring.
//! Durham models consume [`JerseyModulation`] and stamp outputs.
//!
//! Hydrology-shaped families emit height ops and semantic tags only; wet
//! surface rendering is deferred (see Marazion / RFC-127).

pub mod config;
pub mod modulation;
pub mod region;
pub mod stamp;
pub mod stamps;

pub use config::{
	DownhillPair, FractalAnchors, HysteresisSpine, JitteredCenter, MidpointGrading,
	SoftmaskAlongSpine,
};
pub use modulation::{JerseyModulation, RegionAffineModulation, RegionGradingModulation};
pub use region::{CircleRegion, RectRegion, Region2D, RegionNoise};
pub use stamp::{StampSemantics, StampSet};
pub use stamps::{
	BasinWater, BasinWaterParams, Canyon, CanyonParams, CanyonVariant, CaveNetwork,
	CaveNetworkParams, CaveSegment, CaveSegmentKind, HydrologyComplex, HydrologyComplexKind,
	HydrologyComplexParams, KarstNavClass, KarstPocket, KarstPocketParams, MassifStyle,
	PlateauCap, PlateauCapParams, PlateauFootprint, PlateauSurfaceClass, PocketTermination,
	PocketWater, PocketWaterParams, RollingGround, RollingGroundParams, RuggedMassif,
	RuggedMassifParams, ValleyBasin, ValleyBasinParams, ValleyCrossSection, ValleyFloorKind,
	ValleyTrain, ValleyTrainParams, ValleyTrainSegment, ValleyTrainSegmentRole,
};
