# Mesh Reference

Mesh loading via references to avoid loading the same mesh multiple times.

## Usage

```rust
use mesh_ref::{MeshRef, MeshRefHandles, MeshRefPlugin, MirrorAxis};

app.add_plugins(MeshRefPlugin);

// Author a scene with a shared GLB root (`MeshRefRoot` → `WorldAssetRoot` when ready;
// AssetServer + MeshRefHandles keep one strong handle per MeshRef).
let scene = MeshRef::glb("urban/floors/foo.glb").scene();

// Axis-mirrored rebuild (distinct cache key; positive Transform scale at the caller):
let mirrored = MeshRef::glb("urban/roofs/unit_right_triangle.glb")
    .mirrored(MirrorAxis::X)
    .scene();

// Or fetch the handle explicitly (e.g. preload / spawn outside BSN):
fn preload(mut handles: ResMut<MeshRefHandles>, assets: Res<AssetServer>) {
    let _ = handles.handle(&MeshRef::glb("urban/floors/foo.glb"), &assets);
}
```

[`MeshRef::glb`] accepts a path relative to the Bevy asset root. If no `#SceneN`
label is present, scene `0` is used. Optional [`MirrorAxis`] rebuilds positions,
normals, and tangents with reversed winding so single-sided materials stay correct
without negative Transform scale.
