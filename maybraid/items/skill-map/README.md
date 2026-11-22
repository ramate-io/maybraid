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
stained-glass overlay: land/mosaic tiles paint onto the world by mosaic ink
(chroma plus luma occupancy so earthy land still stamps); power tiles,
water (debraid), the cursor, and feed arrows keep full alpha through the
compositor. Spatial maps keep a 5×5 land pocket around spawn so the origin
2×2 is never water. Hitting water fades a black Debraid plate in over the
steer lock instead of holding a red flash. Lowering `style.w` makes land tiles more opaque, not less. Map
Overlay and Map Focus are in-game Settings toggles (on by default). Overlay
off keeps the map in the corner. Focus off skips tunnel vision and HUD hide.
While focus is on and the overlay is up, the rest of the HUD hides and a
vignette-style edge blur (`TUNNEL_VISION_STRENGTH` / `TUNNEL_VISION_RADIUS` /
`TUNNEL_VISION_DARKEN`) fades in on the 3D camera so the centre stays sharp
enough to navigate. The stained-glass field leaves a circular lens in the
centre — sized from the spawn-clearing (no-debraid) pocket, not a magic UV
number — with a 2D braid around the rim (apa-kan tile-rim weave: opposite
sines, integer twists, true over/under stacking). Inside the ring is a clear
window onto the live 3D world (the little centre cursor is replaced); outside
is the skill map as before. Feed arrows and the hit line stay drawn on top of
that window, with a dark outline so they read against the world; the lens
itself is not tinted. Skill activation flashes the braid and sends a
bright sheen sweeping across the lens glass; a water Debraid unravels the
strands from the top of the ring into loose curling ribbons (kept on-screen,
outside the lens — not flattened dashed bands hugging the rim) and then
re-plaits them. Releasing RB restores the HUD and fades the blur out. The expand edge plays
the authored `change_item` clip on the UI bus; releasing RB is silent. The clock pauses when skill-map input is inactive or gameplay
is suspended. Changing the equipped map starts a fresh feed on the next RB
hold.
Timing and sequence definitions live in `src/feed.rs`; presentation lives in
`src/feed_view.rs`. Rhythm maps draw only the chevrons and the hit line — no
lane slots — at [`TANZ_CALLOUT_ICON_PX`], with the four lanes occupying a
third of the original overlay width.

GPU stills of the held overlay (idle, fire flash, mid-unravel, mid re-plait)
for each authored skill map, including the DDR-style Rockadder Feed, live in
[`captures/`](captures/). They are 1920×1080 window screenshots of the real
compositor with a 3D world behind the hole. The lens radius is the spawn-clearing
pocket: about 43.9% of the short overlay axis (~474px at 1080p).

Run the focused checks with:

```sh
cargo test -p maybraid-skill-map --lib --release --locked
cargo test -p character-items --lib --release --locked
```
