# Training Ground

Selecting Training Ground in the Maybraid executable first asks who plays the
rounds: **Your character** (the active saved character) or **Random trainee**.
A trainee is a fresh player-scale character (Braidman, Lero, Mygr, Tuberwaber,
or Wumbus) with a random starter loadout. It is never saved. Its bag does not
reach the active character's files, and the pause menu's Character editor is
locked while one plays.

Every session starts on a fresh seed, and every respawn in Training is a new
round on the next seed. The round's seed picks:

- the site: a pinned FinePatch of the Durham / Chico / Richmond stack (four
  160 m cells) centered within 48 cells of the origin. The site is rerolled
  up to eight times within the seed when no development kind fits it;
- the single-terrace Richmond development, its kind and layout, and the wall
  finish;
- the Brawler rosters;
- the trainee, when the round plays one.

The development sits inside a flat courtyard that is composed onto the
FinePatch mesh. A level 20 m wall encloses the courtyard. The development is
stamped once all four cells exist. The padded courtyard cells then replace the
raw FinePatch cells beneath them, so the courtyard has one floor.

Brawlers stand in two to four squads, each just outside the building of a
development POI. The player spawns 10 m from the nearest squad, facing it.
Brawlers belong to the FFA group and are hostile to it, so they fight the
player, the other squads, and their own packmates from the first frame. Every
fighter is a player-scale biped (`CharacterSpecies::PLAYER_SCALE_BIPEDS`), so
none are too tiny or too large to read in close combat. World mob streaming and
hopscotch are off while Training is live; turning hopscotch off also removes
any urbanized Discovery terrain and buildings. A Training pose is not written.

A death runs the usual glaze, then the next round loads in behind the loading
screen. The pause menu's **Next round** row flips between your character and a
random trainee for that next round.

Leave restores the playable-world fill, parks the player at the default spawn
so Discovery resumes from its saved trail, and returns home.
