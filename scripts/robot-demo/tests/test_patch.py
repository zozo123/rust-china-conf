import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("check_patch", ROOT / "scripts/robot-demo/check-patch.py")
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)


class PatchTests(unittest.TestCase):
    def validate(self, patch):
        with tempfile.NamedTemporaryFile(suffix=".patch") as stream:
            stream.write(patch.encode())
            stream.flush()
            return checker.validate_patch(Path(stream.name), ROOT)

    def test_allows_reviewed_fallback(self):
        checker.validate_patch(ROOT / "demo/fallback-patch.diff", ROOT)

    def test_rejects_protected_tests(self):
        patch = (ROOT / "demo/fallback-patch.diff").read_text().replace(checker.ALLOWED_PATH, "rust/crates/robot-safety-gate/tests/contract.rs")
        with self.assertRaises(ValueError):
            self.validate(patch)

    def test_rejects_additional_file(self):
        patch = (ROOT / "demo/fallback-patch.diff").read_text()
        patch += "\ndiff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-old\n+new\n"
        with self.assertRaises(ValueError):
            self.validate(patch)

    def test_rejects_mode_change(self):
        with self.assertRaises(ValueError):
            self.validate(f"diff --git a/{checker.ALLOWED_PATH} b/{checker.ALLOWED_PATH}\nold mode 100644\nnew mode 100755\n")

    def test_rejects_rename(self):
        with self.assertRaises(ValueError):
            self.validate("diff --git a/old b/new\nsimilarity index 100%\nrename from old\nrename to new\n")

    def test_rejects_empty_patch(self):
        with self.assertRaises((ValueError, subprocess.CalledProcessError)):
            self.validate("")


if __name__ == "__main__":
    unittest.main()
