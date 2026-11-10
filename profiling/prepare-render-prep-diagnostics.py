#!/usr/bin/env python3
"""Prepare an explicit, checksum-pinned diagnostic Bevy source override.

This only prepares source/configuration. It never builds, changes Cargo's registry,
resolves dependencies, or overwrites an existing output directory.
"""

import argparse
import hashlib
import json
import pathlib
import shutil
import subprocess
import tarfile

VERSION = "0.20.1"
CHECKSUM = "0c74591faf8ff1b84beb59e93f05b233a03c62d1cbc446874bd2231b72dde184"
PREFIX = "bevy_render-" + VERSION


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def extract_archive(archive, output):
    """Reject links, traversal, nonstandard members and unexpectedly large input."""
    with tarfile.open(archive, "r:gz") as source:
        members = source.getmembers()
        if sum(member.size for member in members) > 32 * 1024 * 1024:
            raise ValueError("Bevy archive exceeds the 32 MiB source limit")
        for member in members:
            path = pathlib.PurePosixPath(member.name)
            if (path.is_absolute() or ".." in path.parts or not path.parts
                    or path.parts[0] != PREFIX or not (member.isfile() or member.isdir())):
                raise ValueError("Unsafe archive member: " + member.name)
        for member in members:
            destination = output.joinpath(*pathlib.PurePosixPath(member.name).parts)
            if member.isdir():
                destination.mkdir(parents=True, exist_ok=True)
            else:
                destination.parent.mkdir(parents=True, exist_ok=True)
                with source.extractfile(member) as contents, destination.open("xb") as target:
                    shutil.copyfileobj(contents, target)


def wrapper_test_harness(output, source, core, lock):
    """Compile the actual patched module against real pinned public Bevy APIs."""
    harness = output / "wrapper-tests"
    harness.mkdir()
    (harness / "src").mkdir()
    (harness / "Cargo.toml").write_text(
        '[package]\nname = "native-binned-wrapper-tests"\nversion = "0.0.0"\nedition = "2024"\n'
        '[features]\ndefault = []\nrender-prep-diagnostics = ["maybraid-render-prep-diagnostics/diagnostics"]\n'
        '[dependencies]\nbevy = { version = "=0.20.1", features = ["trace_tracy"] }\n'
        'bevy_log = "=0.20.1"\nbytemuck = "1"\nwgpu = { version = "=30.0.1", default-features = false }\n'
        'maybraid-render-prep-diagnostics = { path = ' + json.dumps(str(core))
        + ', features = ["native-dirty-uploads"] }\n'
        '[profile.release.build-override]\nopt-level = 3\nstrip = false\n')
    module = source / "src/render_phase/binned_input.rs"
    (harness / "src/lib.rs").write_text(
        'pub mod render_resource { pub use bevy::render::render_resource::*; }\n'
        'pub mod renderer { pub use bevy::render::renderer::*; }\n'
        'pub mod render_phase { #[path = ' + json.dumps(str(module))
        + '] pub mod binned_input; }\n')
    shutil.copyfile(lock, harness / "Cargo.lock")
    return harness / "Cargo.toml"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=pathlib.Path,
                        help="New absolute directory outside this checkout")
    parser.add_argument("--archive", type=pathlib.Path,
                        help="Published bevy_render-0.20.1.crate; defaults to Cargo cache")
    parser.add_argument("--native-dirty-uploads", action="store_true",
                        help="Enable only the opt-in native input persistence prototype")
    parser.add_argument("--diagnostics", action="store_true",
                        help="Also enable census and independent byte checks for correctness")
    args = parser.parse_args()
    diagnostics = args.diagnostics or not args.native_dirty_uploads
    root = pathlib.Path(__file__).resolve().parent.parent
    output = args.output
    if not output.is_absolute():
        parser.error("--output must be absolute")
    output = output.resolve()
    if output == root or root in output.parents:
        parser.error("--output must be outside this checkout")
    if output.exists():
        parser.error("--output already exists; choose a new directory")
    archive = args.archive
    if archive is None:
        archives = list((pathlib.Path.home() / ".cargo/registry/cache").glob("*/" + PREFIX + ".crate"))
        if len(archives) != 1:
            parser.error("Pass --archive explicitly: expected exactly one cached archive")
        archive = archives[0]
    archive = archive.resolve()
    if digest(archive) != CHECKSUM:
        parser.error("Published archive SHA256 does not match the pinned Cargo.lock checksum")
    lock = root / "Cargo.lock"
    package = next((block for block in lock.read_text().split("[[package]]")
                    if '\nname = "bevy_render"\n' in block), "")
    if ('version = "' + VERSION + '"') not in package or ('checksum = "' + CHECKSUM + '"') not in package:
        parser.error("Start from the normal registry Cargo.lock pinned to Bevy 0.20.1")
    patch = root / "profiling/patches/bevy-render-0.20.1-render-prep.patch"
    core = root / "maybraid/render/prep-diagnostics"
    # Creation must fail if another process raced us. An existing directory is
    # never reused, recursively removed, or overwritten, even after a failure.
    output.mkdir(parents=True, exist_ok=False)
    extract_archive(archive, output)
    source = output / PREFIX
    subprocess.run(["patch", "-t", "-N", "-p1", "-i", str(patch)], cwd=source, check=True)
    native_patch = root / "profiling/patches/bevy-render-0.20.1-native-dirty-uploads.patch"
    if args.native_dirty_uploads:
        subprocess.run(["patch", "-t", "-N", "-p1", "-i", str(native_patch)], cwd=source, check=True)
    cargo = source / "Cargo.toml"
    original = cargo.read_text()
    if original.count('trace = ["profiling"]') != 1:
        raise ValueError("Published trace feature did not match the pinned source")
    enabled = (["native-binned-dirty-uploads"] if args.native_dirty_uploads else [])
    if diagnostics:
        enabled.append("render-prep-diagnostics")
    trace_features = ["profiling"] + enabled
    cargo.write_text(original.replace('trace = ["profiling"]',
        'trace = ' + json.dumps(trace_features) + '\n'
        'render-prep-diagnostics = ["dep:maybraid-render-prep-diagnostics", "maybraid-render-prep-diagnostics/diagnostics"]\n'
        'native-binned-dirty-uploads = ["dep:maybraid-render-prep-diagnostics", "maybraid-render-prep-diagnostics/native-dirty-uploads"]', 1)
        + '\n[dependencies.maybraid-render-prep-diagnostics]\n'
        + 'path = ' + json.dumps(str(core)) + '\noptional = true\ndefault-features = false\n')
    config = output / "render-prep-cargo.toml"
    config.write_text('[patch.crates-io]\nbevy_render = { path = ' + json.dumps(str(source)) + ' }\n')
    shutil.copyfile(lock, output / "input-Cargo.lock")
    harness = wrapper_test_harness(output, source, core, lock) if args.native_dirty_uploads else None
    manifest = {
        "schema": 1, "bevy_render_version": VERSION,
        "published_archive": str(archive), "published_archive_sha256": CHECKSUM,
        "source": str(source), "patch": str(patch), "patch_sha256": digest(patch),
        "patched_source_sha256": digest(source / "src/batching/gpu_preprocessing.rs"),
        "native_dirty_uploads": args.native_dirty_uploads, "diagnostics": diagnostics,
        "wrapper_test_manifest": str(harness) if harness else None,
        "native_patch": str(native_patch) if args.native_dirty_uploads else None,
        "native_patch_sha256": digest(native_patch) if args.native_dirty_uploads else None,
        "bevy_trace_features": trace_features,
        "patched_files_sha256": {str(path.relative_to(source)): digest(path) for path in [
            source / "src/batching/gpu_preprocessing.rs", source / "src/render_phase/mod.rs",
        ] + ([source / "src/render_phase/binned_input.rs"] if args.native_dirty_uploads else [])},
        "patched_manifest_sha256": digest(cargo), "config": str(config), "config_sha256": digest(config),
        "core": str(core), "core_files_sha256": {
            str(path.relative_to(core)): digest(path) for path in sorted(core.rglob("*")) if path.is_file()
        },
        "input_lock_sha256": digest(lock),
        "activation": "explicit --config plus bevy/trace_tracy enables only bevy_trace_features recorded here",
        "runtime": "MAYBRAID_NATIVE_BINNED_UPLOADS=force|dirty (default force); diagnostics additionally MAYBRAID_RENDER_PREP_DIAGNOSTICS=1; MAYBRAID_RENDER_PREP_SAMPLE_FRAMES=120",
        "lock_rule": "First patched resolution updates checkout Cargo.lock. Save/hash as resolved-Cargo.lock; subsequent commands use --locked. Restore normal lock before committing.",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(config)
    print(output / "manifest.json")


if __name__ == "__main__":
    main()
