# One-cluster persistent vegetation fixture

This experiment starts from current main independently of the generalized [packing prototype](https://github.com/ramate-io/maybraid/pull/1579). No speedup has been established.

Build the game with `--features persistent-vegetation-fixture`. With no selector, the feature leaves generation unchanged and installs no custom rendering plugins. Set both launches to the same explicit selector:

```shell
MAYBRAID_VEGETATION_INSTANCE_FIXTURE_TILE=0,0
MAYBRAID_VEGETATION_INSTANCE_MODE=ordinary # or persistent
```

The selector is a finite, exactly representable, 100 m aligned world XZ tile minimum. It replaces precisely that tile's UpperCanopy recipe with the existing uniform Orchard recipe on the existing real ground. Authored planting density, grove noise, placement/material sampling, and the production 32-variant cap remain the same in both modes. Other tile/layer identities retain their ordinary recipes. The selector is seeded into the HCSG session before generation; renderer mode does not select or grow trees.

Both modes pin only the selected host to High LOD. HCSG still owns loading and removal of that host. The construction split happens in `ChicoGroveHost::scene_chunks_with_level`: ordinary builds its existing scenes, while persistent constructs only the selected High batch list. It never builds an ordinary visual hierarchy to migrate later, and it never creates inactive Medium/Low/UltraLow render proxies.

Both presenters use the same semantic source constructor, including `FlattenedComponentsOnly<PlacedVegetation<Arc<StorybookTree>>>`, `VegetationInstance` and the normal High stick-capsule producer. Persistent draws use original supported GLB kits, exact RGBA material keys, the existing leaf/stick shading, and an instance payload uploaded once per newly loaded batch. Aggregate bounds enclose every transformed kit bound plus canopy wind displacement; `NoAutoAabb` prevents native mesh bounds from replacing that union. Draw lookup uses the phase item's canonical `MainEntity` even when its render entity is a placeholder.

The fixture supports one active layer-0 scene camera, forward depth/normal prepasses, at most one relevant shadow directional, and canopy proxy/off shadows. Disjoint preview lights do not invalidate it. Missing camera during world loading stays pending. Deferred cameras, a main camera exposing the proxy layer, relevant point/spot shadows, full canopy shadows, empty source content, and unsupported/failed assets fail the explicit benchmark and issue one error exit in clean and diagnostic builds. The harness must reject an absent fixture and time out a fixture that never completes loading; no ordinary fallback substitutes for a failed persistent run.

`FixtureStatus::Ready` means CPU construction is complete. It does not claim material/GPU readiness or actual coverage. A valid comparison separately requires exactly one nonempty High fixture, identical source digest/counts/bounds and semantic/collider counts, ordinary visual coverage or positive issued custom draws as appropriate, stable upload totals after warmup, screenshots, and GPU/pass evidence. The digest contains deterministic kit/type/RGBA and transform data; it excludes entities and asset IDs.

Build with the separate `persistent-vegetation-diagnostics` game feature for the bounded census subscriber. Counter storage and draw/upload atomics are absent from clean fixture builds. The fixture's strong kit/material caches are cleared after its last root disappears. Timing uses the clean feature with fullscreen 1920×1080, native render scale and `MAYBRAID_VSYNC=off`; the route and static window are shared between modes.
