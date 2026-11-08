# Skill maps

Skill-map items share one inventory slot and the existing Laufer / Tanz inputs.
Their category chooses either a spatial map or a timed feed. The original
Fireball, Dumbwave, Rockadder, and Cosimo maps keep their spatial behavior.

## Rockadder Feed

`SkillMapSpec::new(SkillMapKind::RockadderFeed, seed)` selects the first feed.
It is included in the starter-map roll and uses Rockadder's mosaic materials
and shockwave effect. Its inventory and thumbnail identity is separate from
the spatial Rockadder map.

Follow the directional prompts toward the hit line. The first matching flick
starts the feed and counts as the first hit. Complete each flick by returning
the stick to rest, using the same input rules as the spatial maps:

- **Laufer:** hold the skill-map modifier (RB / keyboard V) and flick either stick.
- **Tanz:** use the existing Tanz stick gestures; no extra modifier is required.

The eight prompts are spaced 0.75 seconds apart, accepting a matching flick
up to 0.22 seconds early or late. Successful prompts trigger Rockadder; missed
prompts pass without stopping the sequence. Holding a direction does not
count as repeated flicks. After the last timing window closes, a 1.5-second
pause leads into the same sequence automatically.

The clock pauses when skill-map input is inactive or gameplay is suspended.
Changing the equipped map starts a fresh feed, waiting for its first flick.
Timing and sequence definitions live in `src/feed.rs`; presentation lives in
`src/feed_view.rs`.

Run the focused checks with:

```sh
cargo test -p maybraid-skill-map --lib
cargo test -p character-items --lib
```
