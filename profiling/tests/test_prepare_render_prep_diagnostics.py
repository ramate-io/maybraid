"""Safety checks for the diagnostic-only published-source preparation helper."""

import importlib.util
import io
import pathlib
import tarfile
import tempfile
import unittest

HELPER = pathlib.Path(__file__).resolve().parents[1] / "prepare-render-prep-diagnostics.py"
SPEC = importlib.util.spec_from_file_location("prepare_render_prep", HELPER)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ArchiveSafety(unittest.TestCase):
    def archive(self, directory, name, kind=tarfile.REGTYPE):
        archive = directory / "source.crate"
        with tarfile.open(archive, "w:gz") as target:
            member = tarfile.TarInfo(name)
            member.type = kind
            if kind == tarfile.REGTYPE:
                member.size = 6
                target.addfile(member, io.BytesIO(b"source"))
            else:
                member.linkname = "/tmp/outside"
                target.addfile(member)
        return archive

    def test_valid_source_extracts_only_inside_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = pathlib.Path(temporary)
            archive = self.archive(directory, MODULE.PREFIX + "/src/lib.rs")
            output = directory / "output"
            output.mkdir()
            MODULE.extract_archive(archive, output)
            self.assertEqual((output / MODULE.PREFIX / "src/lib.rs").read_bytes(), b"source")

    def test_traversal_absolute_paths_and_links_are_rejected_before_writes(self):
        cases = [
            (MODULE.PREFIX + "/../../outside", tarfile.REGTYPE),
            ("/absolute/outside", tarfile.REGTYPE),
            ("other-crate/src/lib.rs", tarfile.REGTYPE),
            (MODULE.PREFIX + "/src/link", tarfile.SYMTYPE),
            (MODULE.PREFIX + "/src/link", tarfile.LNKTYPE),
        ]
        for name, kind in cases:
            with self.subTest(name=name, kind=kind), tempfile.TemporaryDirectory() as temporary:
                directory = pathlib.Path(temporary)
                archive = self.archive(directory, name, kind)
                output = directory / "output"
                output.mkdir()
                with self.assertRaises(ValueError):
                    MODULE.extract_archive(archive, output)
                self.assertEqual(list(output.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
