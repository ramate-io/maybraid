# In-game command organization (`sdf-common-playground`)

## Goals

- **Clap at the leaves**: `Parser` / `Subcommand` / `Args` types stay the source of truth for `/` text mode.
- **Hierarchical `.react(commands)`**: The top-level command keeps parsing generic through `game-commands`, then bespoke reaction code spawns the child components that Bevy systems consume.
- **Plugins mirror the CLI tree**: A command’s **`plugin.rs`** only composes **child plugins** and registers **systems for that same level** (e.g. announcer). Leaf commands own **`their_dir/plugin.rs`** and optional **`their_dir/plugin/react_*.rs`**.
- **No `mod.rs`**: Use `feature.rs` + `feature/` for children (Rust 2018 path clarity).
- **Split types by leaf**: e.g. [`render/tapered_cylinder.rs`](render/tapered_cylinder.rs), [`render/noisy_cylinder.rs`](render/noisy_cylinder.rs), [`render/crook_cylinder.rs`](render/crook_cylinder.rs) with `*Helper` types.

## React flow

1. **Input** comes from `game_commands::command::GameCommandPlugin`: it calls [`PlaygroundCommand::parse_line`](../commands.rs) then [`PlaygroundCommand::react`](../commands.rs) with the HUD console string.
2. **Argv startup** uses [`PlaygroundCommand::parse_startup_command`](../commands.rs), stores the result in `game_commands::command::PendingStartupCommand`, and runs it on the first frame through the same `react` path.
3. **`script --path FILE`** uses `game_commands::command::CommandScript<PlaygroundCommand>`: read lines → `parse_line` → `react` per line.
4. **Root reaction** handles top-level variants:
   - `Help` writes long clap help to the HUD console.
   - `Render` → [`Render::react`](render.rs) spawns [`Render`](render.rs) then the appropriate helper.
   - `Settings` → [`Settings::react`](settings.rs) spawns [`Settings`](settings.rs) then leaf components ([`SettingsCheckerSize`](settings/react_checker_size.rs), [`SettingsSeed`](settings/react_seed.rs)).
5. **Systems** are registered by command plugins:
   - **Render**: [`TaperedCylinderRenderPlugin`](render/tapered_cylinder/plugin.rs), [`NoisyCylinderRenderPlugin`](render/noisy_cylinder/plugin.rs), [`CrookCylinderRenderPlugin`](render/crook_cylinder/plugin.rs), then [`despawn_render_command_announcer`](render/plugin/announcer.rs) from [`RenderCommandsPlugin`](render/plugin.rs).
   - **Settings**: checker → seed → announcer via [`SettingsCommandsPlugin`](settings/plugin.rs).

## Module layout

```text
commands.rs
commands/
  README.md
  plugin.rs
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
    crook_cylinder.rs
    crook_cylinder/
      plugin.rs
      plugin/
        react_crook_cylinder.rs
  settings.rs
  settings/
    plugin.rs
    react_*.rs
```
