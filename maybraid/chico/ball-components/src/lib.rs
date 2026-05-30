//! Ball components for Chico vegetation ([RFC-183 §3.1.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/README.md)).
//!
//! # Sope's Banyan ([§3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! Canopy allocation uses **[Noisy Ball](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/02-noisy-ball/README.md)** (`chico_ball`) and **[Plane Splay](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/05-plane-splay/README.md)** (`plane_splay`) broadly through the **rising crown** (height fraction, terminal branches, branch order per RFC pseudocode). **[Jungle growths](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md)** for dense variants are composed separately in `tree-components`.
pub mod chico_ball;
pub mod plane_splay;
pub mod tuft;

pub use tuft::{
	BellyTipProfile, BladeTuft, BladeTuftShape, BladeTuftStd, BuddhaHandCluster,
	BuddhaHandElement, BuddhaHandTuft, BuddhaHandTuftShape, BuddhaHandTuftStd, SpearCluster,
	SpearElement, SpearTuft, SpearTuftShape, SpearTuftStd, SucculentTuft, SucculentTuftShape,
	SucculentTuftStd, WeepingTuft, WeepingTuftShape, WeepingTuftStd,
};
