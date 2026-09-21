# Threat intelligence

Local semantic threat discovery with retained per-recipient knowledge.

- `ThreatSubject` supplies stable identity and salience.
- `Affiliations` records weighted group memberships plus directional antagonist
  and ally beliefs; individual entities also have reserved singular groups.
  Net threat is max aggravation minus max mitigation.
- A Gimme typed index provides bounded local candidate scans.
- `ThreatObservation` is the directed inbox for sessions, received fire,
  sharing, and other non-spatial discovery sources. Pack share still applies
  `observe` with `ThreatSource::SHARED` directly so mates classify the same
  frame; it does not enqueue a second inbox pass.
- `ThreatSource::FIRST_HAND` is the write-up mask. `SHARED` is derived only
  and is never copied onto pack-mates as a first-hand bit.
- `ThreatKnowledge` retains candidates between scans and reclassifies them as
  affiliation weights decay.
- Threat-owned spotting hints feed candidates to spotting without fabricating
  visual contacts. Received damage, received fire, or a pack-shared finding
  stretches discovery and spotting out to the High envelope (200 m) so a
  long-range hit does not leave an alert plant unable to look back. Local
  scan alone keeps the idle horizon.
- Acting on that set is [`threat-management-intelligence`](../threat-management):
  exclusive Ignore | Evade | Combat over retained knowledge.

Static memberships normally use `AffiliationStrength::permanent`; temporary
suspicion and hostility can use a half-life.
