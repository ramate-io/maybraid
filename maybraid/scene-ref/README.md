# Scene Reference

Scene loading via references to avoid loading the same scene multiple times.

## Usage

```rust
use scene_ref::{
    MirrorAxis, MultiSceneMerge, MultiScenePart, SceneRef, SceneRefHandles, SceneRefPlugin,
};
use bevy::prelude::Transform;

app.add_plugins(SceneRefPlugin);

// Author a scene with a shared GLB root (`SceneRefRoot` → `WorldAssetRoot` when ready;
// AssetServer + SceneRefHandles keep one strong handle per SceneRef).
let scene = SceneRef::glb("urban/floors/foo.glb").scene();

// Axis-mirrored rebuild (vertex/winding only; positive Transform scale at the caller):
let mirrored = SceneRef::glb("urban/panels/unit_right_triangle.glb")
    .mirrored(MirrorAxis::X)
    .scene();

// Instance reflection (verts + conjugated hierarchy; for skinned / nested GLBs):
let reflected = SceneRef::glb("characters/ears/round_scoop_lateral_ear_left.glb")
    .reflected(MirrorAxis::X)
    .scene();

// Merge several scenes into one mesh WorldAsset (per-part transforms baked into verts):
let merged = MultiSceneMerge::new([
    MultiScenePart::identity(SceneRef::glb("foliage/tuft_a.glb")),
    MultiScenePart::new(
        SceneRef::glb("foliage/tuft_b.glb"),
        Transform::from_xyz(1.0, 0.0, 0.0),
    ),
])
.scene();

// Or fetch the handle explicitly (e.g. preload / spawn outside BSN):
fn preload(mut handles: ResMut<SceneRefHandles>, assets: Res<AssetServer>) {
    let _ = handles.handle(&SceneRef::glb("urban/floors/foo.glb"), &assets);
}
```

[`SceneRef::glb`] accepts a path relative to the Bevy asset root. If no `#SceneN`
label is present, scene `0` is used. [`SceneRef::mirrored`] rebuilds positions,
normals, and tangents with reversed winding (caller places the instance).
[`SceneRef::reflected`] does that and conjugates each entity `Transform`, matching
a parent axis-flip without negative Transform scale at the caller.

[`MultiSceneMerge`] resolves each part through [`SceneRefHandles`] (including mirrors),
bakes each mesh entity’s hierarchy transform plus the part transform into vertices,
and concatenates into a single mesh. Materials are not preserved.
