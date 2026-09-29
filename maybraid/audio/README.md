# Maybraid audio

Oddio `SpatialScene` plus CPAL output. Firearms and other gameplay crates play
through [`Audio`](src/lib.rs); they do not open the device.

```text
Bevy ECS
   │
Audio / SpatialOneShot / SpatialEmitter
   │
assets · mixer · spatial
   │
oddio graph → CPAL
```

- **Assets:** [`AudioClip`](src/asset.rs) decodes mono WAV on load. Stereo spatial
  files error instead of averaging (Logic phase/ensemble would cancel).
- **Mixer:** [`AudioBus`](src/mixer.rs) (`Master`, `PlayerWeapon`, `Weapons`,
  `Impacts`, `Ambience`, `Ui`) plus short ducks. A player-weapon one-shot ducks
  ambience (−4 dB / 120 ms) and world weapons (−2 dB / 80 ms).
- **Spatial:** [`SpatialOneShot`](src/spatial.rs) is world-fixed.
  [`SpatialEmitter`](src/spatial.rs) follows an entity with
  [`AudioVelocity`](src/spatial.rs) from simulation, not inferred Δtransform.
- **Output:** CPAL prefers exact F32 stereo, then I16 stereo. Stream errors are
  stored on [`Audio`](src/backend.rs); dropping the resource shuts the thread down.
