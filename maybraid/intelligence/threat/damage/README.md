# Threat intelligence damage adapter

Maps [`DamageApplied`](../../../damage/src/lib.rs) onto threat classification.

The victim learns a decaying individual antagonism toward the source, then
writes a directed `RECEIVED_DAMAGE` observation. Threat discovery still decides
whether that finding clears the affiliation threshold; this crate does not
fabricate spotting contacts.
