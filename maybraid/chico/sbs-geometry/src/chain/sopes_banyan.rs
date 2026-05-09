//! [`ChainHysteresisRule`](crate::ChainHysteresisRule) (and related hysteresis) for **Sope's Banyan** ball-stick chains.
//!
//! # Intent ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! Sope's Banyan uses **long banyan-like chains** with an **upward torch bias** (closer to [Penmarch Torch §3.1.7.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/04-penmarch-torch/README.md) than [Honu Banyan §3.1.7.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md)): bias strength and effective growth angle should **rise with height** along the chain so the crown reads as a **tall, vase-like** lift rather than Honu's broad horizontal spread.
//!
//! The RFC still calls for **periodic downward descenders** (every third to fourth segment, slightly less frequent than Honu if we want a more vertical read). Implementation should alternate or phase hysteresis so most segments use upward `bias_ray` / canopy bias while selected indices switch to a **strongly downward** descender profile (tighter angle tolerance, different length/radius ranges).
//!
//! Parameters called out in the RFC (segment count, child count `1..=3`, angle tolerance on the order of ~12°, length/radius ranges) should eventually be **wireable from CLI** (feature-gated `clap`) for playground tuning.
//!
//! This module only owns the **chain growth rule**; stalk, anchors, sticks, balls, and jungle growths live in sibling crates/modules.
