# Combined receiver fix: long-walk validation

This supplements [the scene validation](VALIDATION.md) for [#1632](https://github.com/ramate-io/maybraid/pull/1632). One ordered OFF→ON pair completed approximately 2.4 km of supported cumulative XZ travel per arm. Terrain shadow receiving was visually confirmed at three route checkpoints. This pair shows no obvious broad p95 regression, but unequal route timing and content prevent a precise causal performance claim. These are observations, not statistical bounds.

Both arms used the same immutable diagnostic executable (`1047b8389cdc65ec0762025f28b322d4fc5dc66de4d4a5541d6add8941b4cf87`), source, input route and tooling, fullscreen 1920×1080 Metal with VSync OFF, noon lighting and High 2048×4 cascades. The only configured rendering setting varied was the combined terrain/stick receiver switch; realized control and scene histories differed. Spherical vegetation proxies remained ON in both; real vegetation visual geometry retained its caster exclusions in the tested/default Proxies mode. Optional Full mode is unchanged. The shared diagnostic uniform layout means this measures flag activation within that build, rather than an exact shipping-link comparison. Comparative GPU timing remains unavailable.

## Whole-walk CPU pacing

Application entry gaps, milliseconds; counts are absolute over the unequal traversal durations. Inclusive rows retain observer periods. Excluded rows use the aligned union of marked screenshot/detour and combat masks across corresponding route chapters, preserving boundary-crossing gaps unless they overlap a mask. This does not make workloads equal.

| Window | Arm | Seconds | Gaps | p50 | p95 | p99 | Maximum | >33.33 ms | >50 ms | >100 ms |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Inclusive walk | OFF | 518.707 | 27,316 | 17.570 | 33.208 | 36.013 | 159.139 | 1,300 | 24 | 1 |
| Inclusive walk | ON | 501.210 | 26,111 | 17.967 | 33.087 | 35.270 | 145.905 | 1,164 | 21 | 3 |
| Observer-excluded walk | OFF | — | 27,032 | 17.578 | 33.228 | 36.013 | 159.139 | 1,297 | 23 | 1 |
| Observer-excluded walk | ON | — | 25,884 | 17.988 | 33.105 | 35.268 | 145.905 | 1,161 | 20 | 3 |

Masks cover 5.067 s OFF and 4.018 s ON, retaining 98.96%/99.13% of gaps. All four >100 ms gaps occurred near checkpoints 45–47 and overlapped none of the marked observers or combat. Main-app wrapper overlap was about 97–98% of each; render overlap was under 9 ms and measured pipeline/LOD zones were small. The shared hitch region was approximately (2064.6, −6.1, 2026.3); these poses come from 0.5-second telemetry, not exact in-frame observations. Cross-arm positions at checkpoint 46 were about 0.387 m apart. No typed chunk additions occurred within ±2 seconds of any >100 ms gap, which does not rule out other work. These overlaps do not identify the blocking work. The rare-hitch difference remains unresolved and must not be dismissed as observer overhead.

## Route and workload qualification

| Metric | OFF | ON |
|---|---:|---:|
| Supported cumulative XZ travel | 2,413.00 m | 2,394.83 m |
| Maximum XZ excursion | 927.52 m | 930.73 m |
| Final checkpoint | 118 | 118 |
| Enabled nonplayer body count at checkpoint 65 | 48 | 70 |
| Enabled nonplayer body count after end standing | 92 | 62 |

Both preserved the same player within each run, HP 100, original gravity and supported finish; no player shots, applied damage or walking combat events were observed. ON had three transient unsupported readbacks and six excluded movement edges. Full-display/PID foreground checks passed in all three phases. The interrupted first OFF attempt is excluded.

Across 119 common checkpoints, median cross-arm XZ separation was 1.142 m and maximum was 14.983 m at checkpoint 47. The checkpoint 43–52 chapter took 46.7→30.1 s; OFF also used a marked detour at checkpoint 65. Change-driven body counts differ materially and are not all physics entities or GPU draw counts. Terrain chunk additions were 153/151, not resident LOD counts. Resident mesh/asset and full LOD workload counts were not captured. Consequently neither raw hitch counts nor fractional masks establish a matched causal effect.

## Sampled gameplay memory

Five-second process samples were bound to exact PID/start identity. Units below are MiB; maxima are sampled lower bounds, not instantaneous peaks.

| Metric | OFF | ON |
|---|---:|---:|
| Initial standing footprint median | 6,383.58 | 6,314.33 |
| End standing footprint median | 10,703.64 | 10,654.91 |
| Footprint median growth | +4,320.06 | +4,340.58 |
| Sampled gameplay footprint maximum | 10,704.51 | 10,666.91 |
| Initial standing RSS median | 5,466.52 | 5,422.53 |
| End standing RSS median | 8,211.38 | 8,197.39 |
| Sampled gameplay RSS maximum | 8,213.83 | 8,200.47 |

Both grew about 4.2 GiB in footprint, reaching sampled maxima about 10.45/10.42 GiB. This does not attribute growth to shadows, establish a leak or reclamation effect, identify owners, or measure GPU residency. Median sample cadence was 5.007 s, maximum gap 5.013/5.010 s.

Requested 40-second standing windows stayed supported but drifted 2.995/4.389 m OFF and 2.995/4.275 m ON. They were nonstationary. Final positions remained about 691 m from the initial position, so this was not a controlled origin return. Moving revisits 52→104 were 3.339/4.708 m apart; equal camera, content and quiescence were not established. Peak-gameplay ownership attribution remains a separate follow-up.
