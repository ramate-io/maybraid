# Maybraid weather

Spawns and despawns weather events near the listener. Wind is the first layer:

| Event | Playback | Life |
|-------|----------|------|
| Breeze | Looping `Ambience` emitter | 10–15 s |
| Gust | World-fixed one-shot | clip length |

Volume stays low; oddio radius is wide so a breeze still reads at a distance.
Player-weapon ducks already pull `Ambience` down. Clips are authored mono WAV
under `sound-effects/environment/wind/`.

Birdsong is a second Ambience one-shot layer on the same listener ring. It does
not require a live herd; gaps are 10–22 s.
