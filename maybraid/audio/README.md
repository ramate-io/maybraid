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
  `Impacts`, `Ambience`, `Ui`, `Voices`) plus short ducks. A player-weapon
  one-shot ducks ambience (−4 dB / 120 ms) and world weapons (−2 dB / 80 ms).
- **Flinch:** [`FlinchProfile`](src/flinch.rs) on a character names a
  [`GruntStyle`](src/flinch.rs). Damage picks a numbered variant noisily and
  plays a world-fixed `Voices` one-shot.
- **Movement:** [`MovementSounds`](src/movement.rs) plays footstep and
  change-item one-shots on `Voices`. Gait cadence follows the walk/run clip
  rates (two plants per cycle). Jump and leap plant once on takeoff and once
  on land, with inhale / exhale on those edges. A grounded sprint cycles the
  same breaths at the authored clip lengths.
- **Ambient:** [`AmbientSounds`](src/ambient.rs) catalogs birdsong (Ambience)
  plus herd grunt / wail (Voices). Weather places birdsong like wind; mobs play
  grunts near a herd and a wail when a member starts fleeing.
- **Weather:** [`maybraid-weather`](../weather/) places quiet, wide-radius
  breeze loops and gust one-shots on the `Ambience` bus.
- **Spatial:** [`SpatialOneShot`](src/spatial.rs) is world-fixed.
  [`SpatialEmitter`](src/spatial.rs) follows an entity with
  [`AudioVelocity`](src/spatial.rs) from simulation, not inferred Δtransform.
- **Output:** CPAL prefers exact F32 stereo, then I16 stereo. Stream errors are
  stored on [`Audio`](src/backend.rs); dropping the resource shuts the thread down.
