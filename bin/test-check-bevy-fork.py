#!/usr/bin/env python3
"""Focused failure checks for mixed Bevy source identities; Python 3.11+."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('check_bevy', Path(__file__).with_name('check-bevy-fork.py'))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


def fixture():
    rev = '1' * 40
    names = ['bevy', 'bevy_render', 'bevy_math', 'bevy_platform', 'bevy_shape']
    pin = dict(git=check.REPOSITORY, rev=rev)
    manifest = dict(workspace=dict(metadata={'bevy-fork': {**pin, 'upstream-rev': check.UPSTREAM}},
                                  dependencies={'bevy': dict(features=['native_binned_dirty_uploads'])}),
                    patch={'crates-io': {name: dict(pin) for name in names}})
    source = f'git+{check.REPOSITORY}?rev={rev}#{rev}'
    packages = [dict(name=name, version=check.VERSION, source=source, id=name+'@fork') for name in names]
    # Independently maintained integration stays registry, but uses the same math ID.
    packages.append(dict(name='bevy_heavy', version='0.6.0', source='registry+https://github.com/rust-lang/crates.io-index', id='heavy@registry'))
    nodes = [dict(id=p['id'], features=[], deps=[]) for p in packages]
    nodes[-1]['deps'] = [dict(name='bevy_math', pkg='bevy_math@fork')]
    metadata = dict(packages=copy.deepcopy(packages), resolve=dict(nodes=nodes))
    return manifest, dict(package=packages), metadata


class ForkGraphTests(unittest.TestCase):
    def test_third_party_can_share_the_exact_fork_math_id(self):
        manifest, lock, metadata = fixture()
        report = check.validate(manifest, lock, metadata)
        self.assertEqual(report['central_crates'], 5)
        self.assertEqual(report['integration_bevy_dependencies']['bevy_heavy'][0]['id'], 'bevy_math@fork')

    def test_mixed_registry_math_is_rejected_even_with_fork_root(self):
        manifest, lock, metadata = fixture()
        duplicate = dict(metadata['packages'][2], id='math@registry', source='registry+https://github.com/rust-lang/crates.io-index')
        metadata['packages'].append(duplicate)
        with self.assertRaisesRegex(ValueError, 'mixes registry/path/other Git source'):
            check.validate(manifest, lock, metadata)

    def test_path_override_cannot_replace_an_exact_revision(self):
        manifest, lock, metadata = fixture()
        manifest['patch']['crates-io']['bevy_render'] = dict(path='/tmp/patched-bevy')
        with self.assertRaisesRegex(ValueError, 'without a path/branch override'):
            check.validate(manifest, lock, metadata)

    def test_no_deps_metadata_is_not_a_resolved_graph_proof(self):
        manifest, lock, metadata = fixture()
        metadata['resolve'] = None
        with self.assertRaisesRegex(ValueError, '--no-deps is insufficient'):
            check.validate(manifest, lock, metadata)


if __name__ == '__main__':
    unittest.main()
