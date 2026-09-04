# Threat intelligence

Local semantic threat discovery with retained per-recipient knowledge.

- `ThreatSubject` supplies stable identity and salience.
- `Affiliations` records weighted group memberships plus directional antagonist
  and ally beliefs; individual entities also have reserved singular groups.
  Net threat is max aggravation minus max mitigation.
- A Gimme typed index provides bounded local candidate scans.
- `ThreatObservation` is the directed inbox for sessions, received fire,
  sharing, and other non-spatial discovery sources.
- `ThreatKnowledge` retains candidates between scans and reclassifies them as
  affiliation weights decay.
- Threat-owned spotting hints feed candidates to spotting without fabricating
  visual contacts.
- Acting on that set is [`threat-management-intelligence`](../threat-management):
  exclusive Ignore | Evade | Combat over retained knowledge.

Static memberships normally use `AffiliationStrength::permanent`; temporary
suspicion and hostility can use a half-life.
