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

    def test_product_identity_is_consistent_across_desktop_helper_and_bundle(self):
        # 保护新数据namespace、固定handoff来源与两个launchd入口一致；纯文本/JSON，不访问用户目录。
        import json
        repo = self.probe["REPO"]
        identifier = "me.disign.veyra"
        old = ("com.lifei6671" + ".veyra", "me.disign.me" + ".veyra")
        self.assertEqual(self.probe["LABEL"], identifier + ".p005")
        self.assertEqual(json.loads((repo / "src-tauri/tauri.conf.json").read_text())["identifier"], identifier)
        expected = {
            "crates/veyra-desktop/src/platform/directories.rs": [
                f'PRODUCT_IDENTIFIER: &str = "{identifier}"',
                f'PREVIEW_NAMESPACE: &str = "{identifier}.gpui-preview"'],
            "crates/veyra-helper/src/production/source.rs": [
                f'Library/Application Support/{identifier}.gpui-preview'],
            "crates/veyra-helper/src/production/install.rs": [
                f'/Library/LaunchDaemons/{identifier}.helper.plist',
                f'LABEL: &str = "system/{identifier}.helper"',
                f'<string>{identifier}.helper</string>'],
            "crates/veyra-helper/src/p0_05/main.rs": [
                f'/Library/LaunchDaemons/{identifier}.p005.plist',
                f'LABEL: &str = "{identifier}.p005"'],
        }
        for filename, values in expected.items():
            content = (repo / filename).read_text()
            for obsolete in old:
                self.assertNotIn(obsolete, content)
            for value in values:
                self.assertIn(value, content)

    def test_kernel_local_filename_contract_preserves_upstream_archive_member(self):
        # 保护安装、原型和探针一致使用真实本地basename，下载成员/上游字节身份不变。
        repo = self.probe["REPO"]
        production = (repo / "crates/veyra-helper/src/production/transport.rs").read_text()
        self.assertIn('/Veyra/Helper/veyra-sing-box"', production)
        installer = (repo / "crates/veyra-helper/src/production/install.rs").read_text()
        self.assertIn('source("veyra-sing-box",', installer)
        self.assertIn('("veyra-sing-box", 0o755)', installer)
        for name in ["main.rs", "probe.rs"]:
            text = (repo / "crates/veyra-helper/src/p0_05" / name).read_text()
            self.assertNotIn('path("sing-box")', text)
            self.assertIn('path("veyra-sing-box")', text)
        probe = (repo / "tools/p0-05-helper-probe.py").read_text()
        self.assertIn('staging / "veyra-sing-box"', probe)
        self.assertIn('sing-box-1.14.0-darwin-arm64/sing-box', probe)
        for filename in ["p0-04-kernel-probe.py", "p0-07-observation-probe.py"]:
            text = (repo / "tools" / filename).read_text()
            self.assertIn("mkdtemp", text)
            self.assertIn("upstream_binary.rename(binary)", text)
            self.assertIn("veyra-sing-box", text)
            compile(text, filename, "exec")  # 只解析，绝不运行联网/内核诊断入口。

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
