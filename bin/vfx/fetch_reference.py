#!/usr/bin/env python3
"""Explicit, checksum-verified retrieval of pinned dust source groups.

This script is never called by a build. The default operation lists groups;
downloads require both --group and --destination outside the runtime assets.
"""

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_LOCK = ROOT / "maybraid/assets/vfx/dust/reference.lock.json"


def checked_path(root, relative):
    """Reject traversal, absolute paths, backslashes, and escaping symlinks."""
    path = PurePosixPath(relative)
    if not relative or path.is_absolute() or ".." in path.parts or "\\" in relative:
        raise ValueError(f"unsafe source path: {relative}")
    root = Path(root).resolve()
    result = (root / path).resolve()
    if not result.is_relative_to(root) or result == root:
        raise ValueError(f"source path escapes destination: {relative}")
    return result


def verify(data, entry):
    if len(data) != entry["bytes"]:
        raise ValueError(f"wrong byte length: {entry['path']}")
    if hashlib.sha256(data).hexdigest() != entry["sha256"]:
        raise ValueError(f"checksum mismatch: {entry['path']}")
    return data


def source_url(lock, path):
    # Validate paths even though the lock and fetched inventory are hash pinned.
    checked_path(Path('/tmp'), path)
    repository = lock["repository"].removeprefix("https://github.com/")
    if repository == lock["repository"] or len(repository.split("/")) != 2:
        raise ValueError("expected a GitHub repository URL")
    return "https://raw.githubusercontent.com/" + "/".join(
        urllib.parse.quote(part, safe="/")
        for part in (repository, lock["commit"], lock["source_root"], path)
    )


def download(lock, entry):
    with urllib.request.urlopen(source_url(lock, entry["path"]), timeout=60) as response:
        data = response.read(entry["bytes"] + 1)
    return verify(data, entry)


def selected_entries(lock, group, inventory=None):
    if group == "reusable":
        return lock["reusable"]
    spec = lock["optional_groups"][group]
    if inventory is None:
        raise ValueError("optional source groups require the verified upstream inventory")
    entries = [entry for entry in inventory["files"]
               if any(entry["path"].startswith(prefix) for prefix in spec["prefixes"])
               and not any(entry["path"].startswith(prefix)
                           for prefix in spec.get("exclude_prefixes", []))]
    if len(entries) != spec["file_count"] or sum(e["bytes"] for e in entries) != spec["bytes"]:
        raise ValueError("optional group differs from the pinned inventory totals")
    return entries


def fetch(lock, group, destination):
    destination = Path(destination).resolve()
    assets = (ROOT / "maybraid/assets").resolve()
    if destination.is_relative_to(assets) or assets.is_relative_to(destination):
        raise ValueError("source downloads must be outside the runtime assets tree")
    inventory_data = download(lock, lock["inventory"])
    inventory = json.loads(inventory_data)
    entries = selected_entries(lock, group, inventory)
    # Validate the entire selection before creating any files.
    outputs = [(entry, checked_path(destination, entry["path"])) for entry in entries]
    for entry, output in outputs:
        if output.exists():
            verify(output.read_bytes(), entry)
            continue
        data = download(lock, entry)
        output.parent.mkdir(parents=True, exist_ok=True)
        temporary = output.with_name(output.name + ".partial")
        temporary.write_bytes(data)
        temporary.replace(output)
    inventory_output = checked_path(destination, lock["inventory"]["path"])
    inventory_output.parent.mkdir(parents=True, exist_ok=True)
    inventory_output.write_bytes(inventory_data)
    return len(entries), sum(entry["bytes"] for entry in entries)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lock", type=Path, default=DEFAULT_LOCK)
    parser.add_argument("--group", choices=("reusable", "recordings", "authoring"))
    parser.add_argument("--destination", type=Path)
    args = parser.parse_args()
    lock = json.loads(args.lock.read_text())
    if args.group is None:
        if args.destination:
            parser.error("--destination requires --group")
        print(json.dumps({"reusable": {"file_count": len(lock["reusable"]),
                                      "bytes": sum(e["bytes"] for e in lock["reusable"])},
                          **lock["optional_groups"]}, indent=2))
        return
    if args.destination is None:
        parser.error("--group requires an explicit --destination outside runtime assets")
    count, size = fetch(lock, args.group, args.destination)
    print(f"Verified {count} files ({size} bytes) in {args.destination}")


if __name__ == "__main__":
    main()
