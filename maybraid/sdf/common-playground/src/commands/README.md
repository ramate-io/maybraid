# In-game command organization (`sdf-common-playground`)

## Goals

- **Clap at the leaves**: `Parser` / `Subcommand` / `Args` types stay the source of truth for `/` text mode.
- **Hierarchical `.react(commands)`**: Each command level spawns itself, then spawns children, so deeper types are plain `Component`s for Bevy queries.
- **Plugins mirror the CLI tree**: A command’s **`plugin.rs`** only composes **child plugins** and registers **systems for that same level** (e.g. announcer). Leaf commands own **`their_dir/plugin.rs`** and optional **`their_dir/plugin/react_*.rs`**.
- **No `mod.rs`**: Use `feature.rs` + `feature/` for children (Rust 2018 path clarity).
- **Split types by leaf**: e.g. [`render/tapered_cylinder.rs`](render/tapered_cylinder.rs), [`render/noisy_cylinder.rs`](render/noisy_cylinder.rs) with `TaperedCylinderHelper` / `NoisyCylinderHelper`.

## React flow

1. **Input** calls [`PlaygroundCommand::parse_line`](../commands.rs) then [`PlaygroundCommand::react`](../commands.rs).
2. **Root** spawns a [`PlaygroundCommand`](../commands.rs) entity and delegates:
   - `Render` → [`Render::react`](render.rs) spawns [`Render`](render.rs) then the appropriate helper.
   - `Settings` → [`Settings::react`](settings.rs) spawns [`Settings`](settings.rs) then leaf components ([`SettingsCheckerSize`](settings/react_checker_size.rs), [`SettingsSeed`](settings/react_seed.rs)).
3. **Systems** (registered by plugins, ordered after [`capture_command_line_input`](../input.rs)):
   - **Render**: [`TaperedCylinderRenderPlugin`](render/tapered_cylinder/plugin.rs), [`NoisyCylinderRenderPlugin`](render/noisy_cylinder/plugin.rs), then [`despawn_render_command_announcer`](render/plugin/announcer.rs) from [`RenderCommandsPlugin`](render/plugin.rs).
   - **Settings**: checker → seed → announcer via [`SettingsCommandsPlugin`](settings/plugin.rs).
   - [`react_playground_command_root`](root.rs): `help` + despawn root [`PlaygroundCommand`](../commands.rs).

## Module layout

```text
commands.rs
commands/
  README.md
  plugin.rs
  root.rs
  render.rs
  render/
    plugin.rs              # RenderCommandsPlugin: child plugins + announcer
    plugin/
      announcer.rs
    tapered_cylinder.rs
    tapered_cylinder/
      plugin.rs            # TaperedCylinderRenderPlugin
      plugin/
        react_tapered_cylinder.rs
    noisy_cylinder.rs
    noisy_cylinder/
      plugin.rs
      plugin/
        react_noisy_cylinder.rs
  settings.rs
  settings/
    plugin.rs
    react_*.rs
```
