"""P0-05 固定路径与预检纯回归；不调用编排 main、launchctl、内核或网络。"""
import runpy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


class ProtectedResourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.probe = runpy.run_path(str(Path(__file__).resolve().parents[1] / "p0-05-helper-probe.py"))

    def test_fixed_paths_match_rust_install_contract(self):
        # 保护管理员编排/手动探针/清理使用的固定 helper 与 Rust 安装路径一致。
        root = self.probe["ROOT"]
        helper = self.probe["HELPER"]
        plist = self.probe["PLIST"]
        self.assertEqual(str(root), "/Library/Application Support/VeyraP005")
        self.assertEqual(helper, root / "helper")
        self.assertEqual(str(plist), "/Library/LaunchDaemons/" + self.probe["LABEL"] + ".plist")
        source = (self.probe["REPO"] / "crates/veyra-helper/src/p0_05/main.rs").read_text()
        self.assertIn(f'const HELPER: &str = "{helper}";', source)
        self.assertIn(f'const PLIST: &str = "{plist}";', source)
        self.assertNotIn("PrivilegedHelperTools", str(helper))

    def test_preflight_and_cleanup_detect_dangling_symlinks(self):
        # 保护拒绝覆盖与残留判定；测试资源只在 temporary directory 中。
        resources = self.probe["existing_protected_resources"]
        with tempfile.TemporaryDirectory(prefix="veyra-p005-orchestrator-test-") as temporary:
            base = Path(temporary)
            paths = {"ROOT": base / "root", "HELPER": base / "helper", "PLIST": base / "daemon.plist"}
            for path in paths.values():
                path.symlink_to(base / "missing-target")
            with patch.dict(resources.__globals__, paths):
                self.assertEqual(resources(), [str(p) for p in paths.values()])
                for path in paths.values():
                    path.unlink()
                self.assertEqual(resources(), [])


if __name__ == "__main__":
    unittest.main()
