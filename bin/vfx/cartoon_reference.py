"""Shared offline reference validation; never edits Git state or source assets."""
from pathlib import Path
import os
import subprocess

EXPECTED_COMMIT = "070e97eed4d94c8c06377c2286ce89895d38d696"


def reference_checkout(value):
    # Keep the caller's absolute spelling, including symlink aliases/spaces. This
    # also exercises those paths in Cargo, rustc and Bevy, not just argparse.
    path = Path(os.path.abspath(Path(value).expanduser()))
    if not path.is_dir():
        raise ValueError(f"reference checkout does not exist: {path}")
    result = subprocess.run(["git", "rev-parse", "HEAD"], cwd=path,
                            capture_output=True, text=True)
    if result.returncode:
        raise ValueError(f"not a Git reference checkout: {path}")
    actual = result.stdout.strip()
    if actual != EXPECTED_COMMIT:
        raise ValueError(f"reference commit mismatch: expected {EXPECTED_COMMIT}, found {actual} at {path}")
    if not (path / "maybraid/assets").is_dir():
        raise ValueError(f"reference asset root is missing: {path / 'maybraid/assets'}")
    return path
