# Scene Reference

Scene loading via references to avoid loading the same scene multiple times.

## Usage

```rust
use scene_ref::{SceneRef, SceneRefHandles, SceneRefPlugin, MirrorAxis};

app.add_plugins(SceneRefPlugin);

// Author a scene with a shared GLB root (`SceneRefRoot` → `WorldAssetRoot` when ready;
// AssetServer + SceneRefHandles keep one strong handle per SceneRef).
let scene = SceneRef::glb("urban/floors/foo.glb").scene();

// Axis-mirrored rebuild (distinct cache key; positive Transform scale at the caller):
let mirrored = SceneRef::glb("urban/roofs/unit_right_triangle.glb")
    .mirrored(MirrorAxis::X)
    .scene();

// Or fetch the handle explicitly (e.g. preload / spawn outside BSN):
fn preload(mut handles: ResMut<SceneRefHandles>, assets: Res<AssetServer>) {
    let _ = handles.handle(&SceneRef::glb("urban/floors/foo.glb"), &assets);
}
```

[`SceneRef::glb`] accepts a path relative to the Bevy asset root. If no `#SceneN`
label is present, scene `0` is used. Optional [`MirrorAxis`] rebuilds positions,
normals, and tangents with reversed winding so single-sided materials stay correct
without negative Transform scale.
