//! **Stalk anchor rings** and projection policy for **Sope's Banyan** ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! # Intent
//!
//! Anchoring follows [§3.1.3 Ball-stick anchors](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/03-ball-stick-anchors/README.md): positions, initial rays, bias directions, and local scale for each canopy chain, usually emitted from the **stalk radial centroid** so limbs read as emerging from trunk mass.
//!
//! Compared to [Honu Banyan](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md), Sope's places rings **much lower**: radial work begins around **40%** of total height \(z_{\min} \approx 0.40 H\), extending to ~**90%** with **5–7** rings at spacing ~**0.08 H**, **6–8** anchors per ring. **Projection length** uses a **vase-like widening**: compact near the bottom of the anchor band, then **`mix` toward longer projections** with normalized height \(u\) (RFC uses `sqrt(u)` between min/max lengths ~**0.25 H** and **0.70 H**).
//!
//! This file will host the **Sope-specific anchor rule** once the shared anchor API exists (compose height bands, ring counts, radial count, direction rules per RFC §3.1.3). Recipe parameters should remain **clap-parseable under feature flags** for playground experiments.
