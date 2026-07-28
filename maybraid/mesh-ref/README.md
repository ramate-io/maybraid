# Mesh Reference

Mesh loading via references to avoid loading the same mesh multiple times.

## Usage

```rust
use mesh_ref::{MeshRef, MeshRefHandles, MeshRefPlugin};

app.add_plugins(MeshRefPlugin);

// Author a scene with a shared GLB root (BSN resolves via HandleTemplate::Path;
// AssetServer + MeshRefHandles keep one strong handle per MeshRef).
let scene = MeshRef::Glb("urban/floors/foo.glb".into()).scene();

// Or fetch the handle explicitly (e.g. preload / spawn outside BSN):
fn preload(mut handles: ResMut<MeshRefHandles>, assets: Res<AssetServer>) {
    let _ = handles.handle(&MeshRef::glb("urban/floors/foo.glb"), &assets);
}
```

[`MeshRef::Glb`] accepts a path relative to the Bevy asset root. If no `#SceneN`
label is present, scene `0` is used.
