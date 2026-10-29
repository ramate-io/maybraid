# Movement integration

Branch `integration/animations`, review PR [#1525](https://github.com/ramate-io/maybraid/pull/1525).
Source heads below identify feature-only ports; original PRs remain open until integration lands.
No source branch infrastructure, unrelated assets, or historical controllers are imported wholesale.

Render-driven adaptations ground Bow through the torso instead of moving its feet and lower
Shrug to a shoulder lift with bent forearms. FacePalm reach is checked for arm clearance as
well as endpoint placement. Phase-advanced shift tests exposed an existing Run knee jump at
phase .5 and the cycle seam; Run now interpolates smoothly through the authored neutral,
extended, and contracted landmarks, retaining the shared .60 flex peak and existing cadence.
Quadruped rest poses account for the fixed front/rear bind branches: sitting lowers the
haunches, and lying folds all four legs with a body offset scaled to measured leg lengths.

The original three-clip pilot and its exact sources are documented in
[the pilot notes](previews/three-clip-pilot/README.md).
HandClap now targets lower, in front of the sternum, and retains calibrated visible hand contact.

## Composition and controller policy

`GroundedGait { walk_weight, run_weight }` consolidates the continuous Approach (#1022)
and Gait (#981) ideas: the first weight blends Idle to moving gait, and the second blends
Walk to Run, all at one mailbox phase. Weight changes keep the same clip identity and phase.
Production controller ownership is `player/lib/src/locomotion.rs`; the optional world-player
motor is disabled in the game and retains its existing selector in this batch. Standalone
character previews select clips directly. There is no second animation clock.

Authored WalkStart/WalkStop/RunStart/RunStop and explicit WalkToRun/RunToWalk remain callable
and previewable alternatives. Continuous grounded composition supersedes their competing
biped automatic selectors. This is an intentional source behavior consolidation, not a claim
that all historical selectors run together. Quadruped launch and trot-to-gallop shifts use their
authored phase handoffs; jump and stance changes interrupt grounded selection.

#1084 and #1331 are the same WalkToRun feature; the #1331 phase-advancing version is retained.
#996 and #1284 combine the left/right Fall stagger and slower ramp.
#1173, #1044, and #1333 are distinct takeoff-to-air, air-to-land, and land-to-run windows.
#1362 shared gait helpers and #1377 LayOrientation are related design changes, integrated
only where their parent features need them. Infrastructure-only #960/#962/#963 are excluded.

## Validation contract

Gestures use fixed Idle0 endpoints. HandClap and FacePalm constrain reach using calibrated
`rig.forearm_endpoints`; the other gestures retain authored angles and are checked against
calibrated visible tips where hand placement matters. Geometry checks use full affine FK
under scaled ancestors. +Y × .5 is only the unsupported/no-geometry fallback.
Nested pose blends preserve scratch depth. Manual `AnimProgress` samples the exact coordinate,
including Still; automatic NPC idle keeps its independent phase. Real-character renders are
review artifacts stored outside the repository. The matrix records port status separately
from tests and visual approval; authored source is not automatically approved.

## Exact source matrix

| PR | Source head | Source author | Feature disposition |
| --- | --- | --- | --- |
| [#1332](https://github.com/ramate-io/maybraid/pull/1332) | `e691a57d79dc76be72549f538501db1046b21722` | Cursor Agent | Feature-only port; verification coverage below |
| [#1315](https://github.com/ramate-io/maybraid/pull/1315) | `8e85f00ca002dc3e8cbc38f5db8e5ead64ccb460` | Cursor Agent | Feature-only port; verification coverage below |
| [#1246](https://github.com/ramate-io/maybraid/pull/1246) | `c2b4fdde1cd054b804f8eaff6e7a9203ff77dc20` | Cursor Agent | Feature-only port; verification coverage below |
| [#1244](https://github.com/ramate-io/maybraid/pull/1244) | `b3d741d3d78c213e4528678397fedc5391aa67e7` | Cursor Agent | Feature-only port; verification coverage below |
| [#1206](https://github.com/ramate-io/maybraid/pull/1206) | `32dffc17feb7fb5593fb34eec032b26f71e0770b` | Cursor Agent | Feature-only port; verification coverage below |
| [#1201](https://github.com/ramate-io/maybraid/pull/1201) | `75c2872d94d6ff6ddbbff5a6708ec2a53a819fc5` | Cursor Agent | Feature-only port; verification coverage below |
| [#1175](https://github.com/ramate-io/maybraid/pull/1175) | `d8d2b5671f63181c8aa37d12b21852f429d25453` | Cursor Agent | Feature-only port; verification coverage below |
| [#1082](https://github.com/ramate-io/maybraid/pull/1082) | `1e0e7d2525d0afaf45bb3ad95fb4aecb16f41d5e` | Cursor Agent | Feature-only port; verification coverage below |
| [#1345](https://github.com/ramate-io/maybraid/pull/1345) | `615c2a98160a19cf3813aa93715e7d412a984b4c` | Cursor Agent | Biped Sit/seat and supine/side Lay callable; breathing held poses |
| [#1344](https://github.com/ramate-io/maybraid/pull/1344) | `31ae1e72b399e74ae2351edc284fa56094817a47` | Cursor Agent | Feature-only port; verification coverage below |
| [#1016](https://github.com/ramate-io/maybraid/pull/1016) | `30fe7b4f9be3a7b910701ad70526b347795519f9` | Cursor Agent | Feature-only port; verification coverage below |
| [#1313](https://github.com/ramate-io/maybraid/pull/1313) | `1c36aec418d36f356609c32b95aca0a8c9082dc1` | Liam Monninger | Feature-only port; verification coverage below |
| [#1000](https://github.com/ramate-io/maybraid/pull/1000) | `d4c7284da5bb04bd776fcb114d0993df66089081` | Cursor Agent | Feature-only port; verification coverage below |
| [#1326](https://github.com/ramate-io/maybraid/pull/1326) | `d3abd16f7f3498e16f5b1d362e15db6fc6bef9d4` | Cursor Agent | Signed flight elbow stroke; production-bind movement regression added |
| [#1310](https://github.com/ramate-io/maybraid/pull/1310) | `27d8e6dfdc74e857fb1276d91d2e5f960f5fbf82` | Cursor Agent | Feature-only port; verification coverage below |
| [#1001](https://github.com/ramate-io/maybraid/pull/1001) | `453d284f723e2803d2564ed9358f5f6a9d3ffc4a` | Cursor Agent | Feature-only port; verification coverage below |
| [#1237](https://github.com/ramate-io/maybraid/pull/1237) | `cdaf84e83a3e6f04d69f4b08e8b516bb530dd9a6` | Cursor Agent | Feature-only port; verification coverage below |
| [#1173](https://github.com/ramate-io/maybraid/pull/1173) | `f39d6206d3f1d5c168b96b2a0d2e9ff8af85ba63` | Cursor Agent | Feature-only port; verification coverage below |
| [#1208](https://github.com/ramate-io/maybraid/pull/1208) | `1e6a0716821b1a2cff83118d896cdf8ef8d2f71f` | Cursor Agent | Feature-only port; verification coverage below |
| [#1168](https://github.com/ramate-io/maybraid/pull/1168) | `679e70af84b6903e120fd8ceeee91e777953a272` | Cursor Agent | Feature-only port; verification coverage below |
| [#1022](https://github.com/ramate-io/maybraid/pull/1022) | `5d4d9487039c8287d21852d75b69d8a54122edfd` | Cursor Agent | Continuous low-speed behavior consolidated into GroundedGait; unrelated urban assets excluded |
| [#1343](https://github.com/ramate-io/maybraid/pull/1343) | `e50e43b584803f2840c54c49a1041af1aa0aa142` | Cursor Agent | Quadruped Sit/LieDown/curl callable; quiet held idle defaults, authored cycle builders retained |
| [#1027](https://github.com/ramate-io/maybraid/pull/1027) | `2ac444b418f6913462eef7205d280a91bdb4148e` | Cursor Agent | Feature-only port; verification coverage below |
| [#1333](https://github.com/ramate-io/maybraid/pull/1333) | `60278439b2c62148607170e8ad8c8a5d5786f307` | Cursor Agent | Feature-only port; verification coverage below |
| [#1331](https://github.com/ramate-io/maybraid/pull/1331) | `9ba5d64457d739d76646353439dc5efc51b7aff2` | Cursor Agent | Explicit WalkToRun alternative; phase-advancing version |
| [#1275](https://github.com/ramate-io/maybraid/pull/1275) | `822f51983163c33939e49c4887adc022f5599e26` | Cursor Agent | Feature-only port; verification coverage below |
| [#1242](https://github.com/ramate-io/maybraid/pull/1242) | `a480a1a04a5bbd91526e9d94a31297365d0b9c46` | Cursor Agent | Feature-only port; verification coverage below |
| [#1207](https://github.com/ramate-io/maybraid/pull/1207) | `52684e4b8477f52dcea6afd009c9cc19851ba1ca` | Cursor Agent | Feature-only port; verification coverage below |
| [#1081](https://github.com/ramate-io/maybraid/pull/1081) | `61b07deb14c6785d624b67075f271bcc39f28edb` | Cursor Agent | Feature-only port; verification coverage below |
| [#1171](https://github.com/ramate-io/maybraid/pull/1171) | `5cae3460b38645537dd596d6c17a7c38a42abde6` | Cursor Agent | Feature-only port; verification coverage below |
| [#1044](https://github.com/ramate-io/maybraid/pull/1044) | `b0f6a485305726e3e39cad76e7a699ffbf50ce46` | Cursor Agent | Feature-only port; verification coverage below |
| [#1088](https://github.com/ramate-io/maybraid/pull/1088) | `674dabd0995b22cfabb3f95c23472142e64630a9` | Cursor Agent | Feature-only port; verification coverage below |
| [#1084](https://github.com/ramate-io/maybraid/pull/1084) | `d6811f816aacca59940b1a9094d362ff07a6aa60` | Cursor Agent | Combined with #1331; newer phase handoff retained |
| [#1033](https://github.com/ramate-io/maybraid/pull/1033) | `63ea22ef785dc83967efa446288ede7f7e8324fb` | Cursor Agent | WalkStop callable; automatic biped selector superseded by continuous policy |
| [#1038](https://github.com/ramate-io/maybraid/pull/1038) | `c80796b1a50fa37b98fce2d83cb093170698ef36` | Cursor Agent | Feature-only port; verification coverage below |
| [#1029](https://github.com/ramate-io/maybraid/pull/1029) | `4b2ef7b9b73e5063c58d6654902b6ee13a69a70c` | Cursor Agent | Feature-only port; verification coverage below |
| [#1018](https://github.com/ramate-io/maybraid/pull/1018) | `5ef1b36d29a9c7e39791c71d492aeb006a02f189` | Cursor Agent | Feature-only port; verification coverage below |
| [#1284](https://github.com/ramate-io/maybraid/pull/1284) | `2764265003ab59cf6947335c08f2384bb7d77d47` | Cursor Agent | Feature-only port; verification coverage below |
| [#996](https://github.com/ramate-io/maybraid/pull/996) | `3ea5e64a05e5e22e32c7320ee931c736ce8f22ce` | Cursor Agent | Feature-only port; verification coverage below |
| [#1031](https://github.com/ramate-io/maybraid/pull/1031) | `1bc721f56a405f86e9d9521d31fb327e07d6fa25` | Cursor Agent | RunStop callable; automatic biped selector superseded by continuous policy |
| [#1023](https://github.com/ramate-io/maybraid/pull/1023) | `1c7dd80d0432ab822d9658131ba5f6a5f259e363` | Cursor Agent | WalkStart callable; automatic biped selector superseded by continuous policy |
| [#981](https://github.com/ramate-io/maybraid/pull/981) | `723b6fbf2f7c434f642fd4081111055cda5800d3` | Cursor Agent | Continuous walk/run behavior consolidated into GroundedGait |
| [#1002](https://github.com/ramate-io/maybraid/pull/1002) | `056528ef6ee5558632392698c733765a8bade349` | Cursor Agent | Feature-only port; verification coverage below |

## Current verification checkpoint

829 full-suite library tests pass across character-animations, character-motion, character-rigs,
characters, characters-playground, player, and world-player; 11 existing tests are ignored.
The 34 character integration tests also pass. After the rendered corrections, 47 quadruped
tests and both preview catalog tests pass, including a full Jump pose/offset endpoint check.
Jump previews use the full windup/flight/landing duration for one-shot and looping playback.
The production game and character playground pass `cargo check` with the shared package
feature union. Scoped Rust formatting, controller boundaries, and whitespace checks pass.
This includes the original three-clip regressions. Rendered review caught and corrected
Bow, Shrug, FacePalm, StompBeat and quadruped rest geometry; passing FK tests alone did
not approve those visible poses. Captures use the real Braidman and
Epiphant assemblies from front, side, and three-quarter views. They exercise sampled poses;
interactive world collision, terrain contact, and gameplay feel still require playtesting.
