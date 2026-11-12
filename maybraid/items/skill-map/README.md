# Skill maps

Skill-map items share one inventory slot and the existing Laufer / Tanz inputs.
Their category chooses either a spatial map or a timed feed. The original
Fireball, Dumbwave, Rockadder, and Cosimo maps keep their spatial behavior.

## Rockadder Feed

`SkillMapSpec::new(SkillMapKind::RockadderFeed, seed)` selects the first feed.
It is included in the starter-map roll and uses Rockadder's mosaic materials
and shockwave effect. Its inventory and thumbnail identity is separate from
the spatial Rockadder map.

Follow the directional prompts toward the hit line. Holding RB starts the
phrase immediately; releasing pauses it, and holding again restarts from the
first prompt. Complete each flick by returning the stick to rest, using the
same input rules as the spatial maps:

- **Laufer:** hold the skill-map modifier (RB / keyboard V) to expand the overlay, then flick either stick.
- **Tanz:** hold the same modifier to expand the overlay, then use the existing Tanz stick gestures. Releasing RB collapses the map back to the corner. Tanz execution itself does not require the hold.

The eight prompts are spaced 0.75 seconds apart, accepting a matching flick
up to 0.22 seconds early or late. Successful prompts trigger Rockadder; missed
prompts pass without stopping the sequence. Holding a direction does not
count as repeated flicks. After the last timing window closes, a 1.5-second
pause leads into the same sequence automatically.

Holding RB expands the map from the bottom-right corner into a full-screen
stained-glass overlay: tile shaders switch to overlay mode (lighter tint,
gentler sway) and a compositor warps the map camera image so the world stays
readable through coloured, refracted glass. The expand edge plays the authored
`change_item` clip on the UI bus; releasing RB is silent. The clock pauses when
skill-map input is inactive or gameplay is suspended.
Changing the equipped map starts a fresh feed on the next RB hold.
Timing and sequence definitions live in `src/feed.rs`; presentation lives in
`src/feed_view.rs`. Rhythm maps draw only the chevrons and the hit line — no
lane slots — at [`TANZ_CALLOUT_ICON_PX`], with the four lanes occupying a
third of the original overlay width.

Run the focused checks with:

```sh
cargo test -p maybraid-skill-map --lib
cargo test -p character-items --lib
```
