# Bevy fork dependency and native input uploads

Maybraid uses an exact revision of [ramate-io/bevy](https://github.com/ramate-io/bevy), based on the published Bevy 0.20.1 source commit `a0577b5c6a7b208719dd761c5c6a7a539e472a97`. The initial fork pin is [`17e8a12c5e0e374535fe2064c4c0d7d26f0a85d1`](https://github.com/ramate-io/bevy/commit/17e8a12c5e0e374535fe2064c4c0d7d26f0a85d1). The authoritative full pin is in `workspace.metadata.bevy-fork` and every central Bevy entry in root `[patch.crates-io]`; `Cargo.lock` records the resolved Git source. Normal builds need no temporary source directory or Cargo command-line override.

All 69 central Bevy crates are patched together, including the directly used `bevy_math`, `bevy_platform` and `bevy_shape`. Patching only `bevy` or `bevy_render` could leave registry types in dependencies such as Avian or Hanabi. The initial audit verified all 69 published archive checksums against the normal lock and found the same upstream VCS commit and matching repository path in every archive.

`bevy_hanabi`, `bevy_heavy`, `bevy_mikktspace` and `bevy_transform_interpolation` are independently maintained integrations, not packages from that Bevy monorepo. They retain their own existing Git/registry sources. Their dependencies on central Bevy packages resolve through the same root patch. Avian stays on its existing published version as well. No Maybraid diagnostics crate is a dependency of the Bevy fork.

## Runtime and verification features

Maybraid explicitly enables Bevy's `native_binned_dirty_uploads` feature. The feature is default-off in Bevy itself and affects only the three retained CPU-authored binned inputs; ordinary `RawBufferVec::write_buffer`, fan scratch and per-frame jobs retain their native behavior. It changes those three public field types only while enabled.

`BEVY_NATIVE_BINNED_UPLOADS=force|dirty` is read once per process. **The enabled feature defaults to dirty when the variable is absent.** Explicit `force` requests every original nonempty native upload for rollback and same-binary comparisons; an unrecognized value also falls back to force. The Bevy feature itself remains default-off upstream; Maybraid explicitly opts in. Do not assume the historical diagnostic CPU reduction establishes GPU speedup, whole-frame pixel parity or universal correctness. This integration stacks the separately reviewed grounding fix, which removes historical-plane velocity projection when no current walkable contact exists. Its exact commit and source hash are part of runtime build provenance; the earlier incomplete normal-component restoration is not accepted as the final fix.

Independent submitted-byte checks are separately compiled using:

```sh
cargo build --locked -p maybraid --features bevy/native_binned_dirty_upload_diagnostics
```

That verification feature is not enabled by ordinary workspace builds. Every would-skip is checked independently of census settings, and mismatches always emit an error. Set `BEVY_NATIVE_BINNED_UPLOAD_DIAGNOSTICS=1` to emit bounded `native_binned_upload_census` frame/phase summaries; `BEVY_NATIVE_BINNED_UPLOAD_SAMPLE_FRAMES=120` selects the sampling interval, with a default cap of 64 samples per phase. `BEVY_NATIVE_BINNED_UPLOAD_MAX_SAMPLES` can raise that per-phase cap up to 1024 (capture preparation uses an explicit cap). Verification is independent of Tracy and should be absent from clean timing comparisons. Feature names forward through the root Bevy crate and `bevy_internal` to the renderer, avoiding an application-only direct dependency on `bevy_render`.

## Check the complete package graph

Use the repository's Nix environment (Python 3.11+, Cargo and the pinned Rust toolchain):

```sh
python3 bin/check-bevy-fork.py
cargo metadata --locked --format-version 1 > /tmp/maybraid-bevy-metadata.json
python3 bin/check-bevy-fork.py --metadata /tmp/maybraid-bevy-metadata.json
python3 bin/test-check-bevy-fork.py
```

The first check validates the checked-in manifest/lock without fetching or building. The full metadata check additionally verifies resolved package IDs and Avian/integration edges. `--no-deps` is insufficient. Registry/path/other-Git central Bevy packages, duplicate central versions, mismatched patch revisions and missing math/platform/shape patches fail the check. Independently maintained integrations are explicitly identified rather than inferred from a `bevy_` prefix.

When upgrading the fork, review the target commit's ancestry/source against the published base, update every fork patch and the metadata revision together, resolve the lock with Cargo, and rerun both checks. Inspect lock changes for unrelated dependency upgrades. Do not use a branch name, the fork's unrelated default branch, a local absolute path or an ambient Cargo override as the final dependency pin.

## Validation before shipping

After source-graph checks, run focused fork upload-state/mutation/diagnostic tests, the Maybraid player/controller suite and representative renderer consumers including Avian/Hanabi. Build the actual application with ordinary features, then separately compile byte verification; inspect compiler feature/source proofs so the normal binary excludes verification. Exercise rendering, shadows/prepass, streaming and movement/fire in a smoke observation; retain actual correctness evidence and acknowledge dynamic actors/workload variation.

Packaging is requested with `ci-action::package` in the integration commit message. Existing package and script-smoke workflows must consume the same checked-in lock/pin on macOS, Windows and Linux. No previous rejected Metal capture or artificial fixture is accepted as evidence for the fork build.
