# Maybraid weather

Spawns and despawns weather events near the listener. Wind is the first layer:

| Event | Playback | Life |
|-------|----------|------|
| Breeze | Looping `Ambience` emitter | 10–15 s |
| Gust | World-fixed one-shot | clip length |

Volume stays low; oddio radius is wide so a breeze still reads at a distance.
Player-weapon ducks already pull `Ambience` down. Clips are authored mono WAV
under `sound-effects/environment/wind/`.
