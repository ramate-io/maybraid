# Combined shadow receiving experiment

Production changes are the seven additions in the terrain and stick shaders.
`combined-receivers.patch` is a test-only overlay, not a shipping material API.
Apply it to a separate worktree of this PR with `git apply`; do not apply it to
an active build. It adds a fixed-layout uniform switch and reuses the explicit
read-only camera/spherical-proxy admission observer. The switch is default off:

- A: `MAYBRAID_EXPERIMENT_SHADOW_RECEIVERS=0`
- B: `MAYBRAID_EXPERIMENT_SHADOW_RECEIVERS=1`

Both arms use existing spherical vegetation proxies ON. Real visual trunks and
foliage retain `NotShadowCaster`. No proxy-OFF or shadow-quality matrix is needed.
The process-start switch controls terrain and stick receiving together. Require
both startup receiver lines to match the selected arm.

Build one immutable native binary with `debug-commands,bevy/trace_tracy`, using
the locked published Bevy graph and the repository's native/Nix setup. Do not
infer shader compilation from Rust compilation: native image acceptance must
exercise both materials. The explicit camera read request with diagnostics on
and `MAYBRAID_SHADOW_ADMISSION=1` reports actual nearest sphere/cascade membership;
its scan and screenshots are observer work outside timing windows. The same
request associates actual `building-walk-collider` children with their building
hosts, then reads realized mesh Aabbs and transforms their eight corners to
world bounds. Geometry ownership follows at most 32 ancestors. Hosts are ranked
by the nearest observed world-AABB surface, reporting the nearest three hosts
and up to 48 mesh records each. It reports actual bounds, mesh load/visibility,
caster/receiver/layer/cascade fields, omitted records, missing/invalid bounds and
ancestry limits. Host/collider and mesh transform origins are metadata only:
Richmond can encode rotation compensation there with world-authored geometry,
so origins are not physical distances or spawn targets. Current Aabbs remain
conservative component observations, not exact surfaces or GPU completion.
Absent custom-material/GPU information stays explicit. This is an
observation, not a synthetic building or a GPU completion stamp.

Each arm uses the same ordinary supported HP100 player route and two fixed
camera stops: close building wall/roof with sunlit ground, then vegetation with
visible ground and bark. Record actual camera, light, cascades, caster layers,
loaded geometry and visibility counts. For buildings distinguish actual near
geometry from deliberately noncasting far massing, and establish whether the
visible paving uses terrain or urban material. Do not assume the screenshot
identifies the missing caster or receiver. No synthetic casters or pose/HP fixes.

Accept localized cast/received shadows, preserved opt-outs, no new obvious acne
or peter-panning, and a short ordinary motion check for cascade/LOD flicker.
Match actual scene/light settings, fullscreen 1920×1080 and VSync OFF/Immediate.
Report clean CPU app-entry/main/render p50/p95/p99, tails/hitches and observer
exclusions. Keep process footprint/RSS samples separate and identify sampled
peaks. Workload-count tolerances are screens, not proof of equivalent GPU draws.

Use separate short Metal captures for actual GPU cost. First prove an 8–10s
recording can finalize within a finite bound; preserve failed attempts. CPU
RenderApp time and approximately 60 FPS do not establish GPU headroom. Compare
the same admitted two arms; do not broaden into more quality modes.

The earlier terrain-only `af86a12d` results do not validate the combined stick
patch or near-building casting. Combined compilation, fixtures, native image
acceptance and CPU/GPU overhead remain pending. Raw images, traces, metadata and
machine-specific build receipts are deliberately excluded from this checkpoint.
