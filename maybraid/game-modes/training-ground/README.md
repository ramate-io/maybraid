# Training Ground

Selecting Training Ground in the Maybraid executable first asks who plays the
rounds: **Your character** (the active saved character) or **Random trainee**.
A trainee is a fresh player-scale character (Braidman, Lero, Mygr, Tuberwaber,
or Wumbus) with a random starter loadout. It is never saved. Its bag does not
reach the active character's files, and the pause menu's Character editor is
locked while one plays.

Every session starts on a fresh seed. A respawn as your character is a new
round on the next seed, so it lands on a new map. A respawn as a random trainee
keeps the map, with its development, wall, and the Brawlers still standing,
and only rolls the next trainee. The seed picks:

- the site: a pinned FinePatch of the Durham / Chico / Richmond stack (four
  160 m cells) centered within 48 cells of the origin. The site is rerolled
  up to eight times within the seed when no development kind fits it;
- the single-terrace Richmond development, its kind and layout, and the wall
  finish;
- the Brawler rosters;
- the trainee, when the round plays one, together with how many lives have
  been played on the map.

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
screen. A trainee's next life reseats on the arena's player seat. The pause
menu's **Next round** row flips between your character and a random trainee
for that next round.

A score panel in the top-right corner keeps the session's tally across every
round and life. It uses the hit-marker points: 1 for a hit, 2 for a headshot,
and 5 more for a down. It also shows downs, deaths, the current streak (downs
since the last death), and the best streak. Only the player's own hits count;
Brawlers downing each other score nothing. Under the score, a live enemy
count shows how many Brawlers are standing right now. A downed Brawler drops
out of it until its squad respawns it. The panel hides with the rest of the
combat HUD while paused.

A red dot floats over every standing Brawler. As with the mob HUD's pins, a
Brawler off screen pins its dot to the nearest screen edge, ringed in white.
So a squad beyond the wall, or under the floor, still shows where it is. The
dots hide while paused, too.

Leave restores the playable-world fill, parks the player at the default spawn
so Discovery resumes from its saved trail, and returns home. Leaving drops the
score, and the next session starts from zero.
