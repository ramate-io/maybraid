#!/usr/bin/env python3
"""Verify the normal Cargo manifest/lock and optional resolved Bevy package IDs.

Requires Python 3.11+ (included in nix develop). Never runs Cargo or fetches code.
"""
import argparse
import json
from pathlib import Path
import re
import sys
try:
    import tomllib
except ImportError:
    raise SystemExit('Use Python 3.11+ from nix develop; tomllib is required.')

# Independently maintained integrations, not crates from the Bevy monorepo.
EXTERNAL = {'bevy_hanabi', 'bevy_heavy', 'bevy_mikktspace', 'bevy_transform_interpolation'}
REPOSITORY = 'https://github.com/ramate-io/bevy'
UPSTREAM = 'a0577b5c6a7b208719dd761c5c6a7a539e472a97'
VERSION = '0.20.1'


def central(name):
    return (name == 'bevy' or name.startswith('bevy_')) and name not in EXTERNAL


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validate(manifest, lock, metadata=None):
    pin = manifest['workspace']['metadata']['bevy-fork']
    require(pin['git'] == REPOSITORY, 'unexpected Bevy fork repository')
    revision = pin['rev']
    require(re.fullmatch('[0-9a-f]{40}', revision) is not None, 'fork rev must be a full commit SHA')
    require(pin['upstream-rev'] == UPSTREAM, 'published upstream base changed; repeat the archive audit')
    expected_source = f'git+{REPOSITORY}?rev={revision}#{revision}'
    patches = {name: value for name, value in manifest['patch']['crates-io'].items() if central(name)}
    require('bevy' in patches and 'bevy_render' in patches, 'missing root/renderer fork patches')
    for name, patch in patches.items():
        require(patch == {'git': REPOSITORY, 'rev': revision}, f'{name}: patch must use the same exact Git revision without a path/branch override')
    required = {'bevy_math', 'bevy_platform', 'bevy_shape'}
    require(required <= patches.keys(), 'missing direct Bevy math/platform/shape patches')
    require('native_binned_dirty_uploads' in manifest['workspace']['dependencies']['bevy'].get('features', []), 'Maybraid must explicitly enable the primary native upload feature')

    def check_packages(packages, label):
        grouped = {}
        for package in packages:
            name = package['name']
            if not central(name):
                continue
            require(name in patches, f'{label}: {name} has no root patch')
            require(package['version'] == VERSION, f'{label}: unexpected {name} version {package["version"]}')
            require(package.get('source') == expected_source, f'{label}: {name} mixes registry/path/other Git source: {package.get("source")}')
            require(name not in grouped, f'{label}: duplicate package identity for {name}')
            grouped[name] = package
        require(grouped.keys() == patches.keys(), f'{label}: patch/package mismatch: missing={sorted(patches.keys()-grouped.keys())}, extra={sorted(grouped.keys()-patches.keys())}')
        return grouped

    locked = check_packages(lock['package'], 'Cargo.lock')
    report = dict(repository=REPOSITORY, revision=revision, upstream_revision=UPSTREAM,
                  central_crates=len(locked), source=expected_source,
                  independently_maintained_integrations=sorted(EXTERNAL),
                  proof='manifest and lock only; resolved package IDs not supplied')
    if metadata is not None:
        packages = check_packages(metadata['packages'], 'cargo metadata')
        require(metadata.get('resolve') is not None, 'full cargo metadata required; --no-deps is insufficient')
        by_id = {package['id']: package for package in metadata['packages']}
        nodes = {node['id']: node for node in metadata['resolve']['nodes']}
        require(all(package['id'] in nodes for package in packages.values()), 'resolved central package node missing')
        integrations = {}
        for package in metadata['packages']:
            if package['name'] not in EXTERNAL | {'avian3d'}:
                continue
            targets = []
            for dep in nodes.get(package['id'], {}).get('deps', []):
                target = by_id[dep['pkg']]
                if central(target['name']):
                    require(target['id'] == packages[target['name']]['id'], 'integration points at a second Bevy package ID')
                    targets.append(dict(name=target['name'], id=target['id']))
            integrations[package['name']] = targets
        report.update(proof='manifest, lock, resolved package IDs and integration edges',
                      integration_bevy_dependencies=integrations,
                      render_features=nodes[packages['bevy_render']['id']]['features'])
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--metadata', type=Path, help='Full cargo metadata --locked --format-version 1 JSON; never --no-deps')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    try:
        manifest = tomllib.loads((root / 'Cargo.toml').read_text())
        lock = tomllib.loads((root / 'Cargo.lock').read_text())
        metadata = json.loads(args.metadata.read_text()) if args.metadata else None
        print(json.dumps(validate(manifest, lock, metadata), indent=2))
    except (KeyError, ValueError, OSError) as error:
        print(f'Bevy fork source check failed: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
