import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("manifest", ROOT / "scripts/prebuilt_manifest.py")
manifest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(manifest)

BUILD = 'const OCCT_VERSION: &str = "V8_0_0";\nconst BUILD_REVISION: &str = "cadona1";\n'
COMMIT = "a" * 40


class PrebuiltManifestTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        for target in manifest.TARGETS:
            path = self.directory / f"occt-8_0_0_cadona1-{target.replace('-', '_')}.tar.gz"
            path.write_bytes(target.encode())

    def test_every_package_has_the_exact_source_revision_size_and_digest(self):
        result = manifest.describe(self.directory, COMMIT, BUILD)
        self.assertEqual(result["source_commit"], COMMIT)
        self.assertEqual(result["release_tag"], "occt-8_0_0_cadona1")
        self.assertEqual(len(result["artifacts"]), 7)
        for artifact in result["artifacts"]:
            payload = artifact["target"].encode()
            self.assertEqual(artifact["sha256"], hashlib.sha256(payload).hexdigest())
            self.assertEqual(artifact["bytes"], len(payload))

    def test_missing_empty_and_unexpected_packages_fail(self):
        path = next(self.directory.glob("*.tar.gz"))
        original = path.read_bytes()
        for replacement in (None, b""):
            path.unlink()
            if replacement is not None:
                path.write_bytes(replacement)
            with self.assertRaises(ValueError):
                manifest.describe(self.directory, COMMIT, BUILD)
            path.write_bytes(original)
        path.rename(self.directory / "occt-stale.tar.gz")
        with self.assertRaises(ValueError):
            manifest.describe(self.directory, COMMIT, BUILD)

    def test_unknown_source_revision_and_changed_build_constants_fail(self):
        for commit in ("main", "a" * 7, "a" * 39, "g" * 40):
            with self.assertRaises(ValueError):
                manifest.describe(self.directory, commit, BUILD)
        for source in ("", BUILD + BUILD, BUILD.replace('"cadona1"', '"cadona2"')):
            with self.assertRaises(ValueError):
                manifest.describe(self.directory, COMMIT, source)


if __name__ == "__main__":
    unittest.main()
