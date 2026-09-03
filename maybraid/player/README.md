# Player

Capsule body, character visual, and **handoff slots** for camera and pose.

The collider size comes from the character recipe's
[`LocomotionCapsule`](../crozon/characters/src/components.rs), not from this
crate. Spawn uses the humanoid default until a visual is attached; attaching a
recipe reapplies that hull (`Collider`, `ShapeCaster`, component). This crate
does not know about firearms or melee. Item-user crates write the
slots; `player-camera` and overlay systems read them. [`Npc`](src/identity.rs)
reuses the capsule, [`PlayerLook`](src/identity.rs), and locomotion clips
without pad input, `CameraFollow`, or `PlayerVisual` (so first-person face hide
stays on the followed body).

```text
CharacterIntent ─► wish / jump          (this crate)
               ─► look / POV            (player-camera)
               ─► UseItem               (firearm-user, melee-user)

PlayerLook          camera → item users read this, not Camera3d
PlayerCameraAim     item users → camera follow blends toward pose
PlayerYawOwner      Wish (locomotion face) vs Look (first-person cone)
PlayerUse           which driver currently claims extra pose/camera
```

System sets other crates add to:

| Set | When | Writers |
|---|---|---|
| `PlayerSystems::Intent` | Update | move / jump |
| `PlayerSystems::Body` | Update | capsule physics |
| `PlayerPoseSystems::Item` | after body, before locomotion | held-mesh pose |
| `PlayerSystems::Locomotion` | before `CharacterMotionSystems::Anim` | walk/run clips |
| `PlayerPoseSystems::Overlay` | after Anim | IK / melee overlay |
