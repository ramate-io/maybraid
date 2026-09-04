# Threat intelligence

Local semantic threat discovery with retained per-recipient knowledge.

- `ThreatSubject` supplies stable identity and salience.
- `Affiliations` records weighted group memberships and directional antagonist
  beliefs; individual entities also have reserved singular groups.
- A Gimme typed index provides bounded local candidate scans.
- `ThreatObservation` is the directed inbox for sessions, received fire,
  sharing, and other non-spatial discovery sources.
- `ThreatKnowledge` retains candidates between scans and reclassifies them as
  affiliation weights decay.
- Threat-owned spotting hints feed candidates to spotting without fabricating
  visual contacts.

Static memberships normally use `AffiliationStrength::permanent`; temporary
suspicion and hostility can use a half-life.
