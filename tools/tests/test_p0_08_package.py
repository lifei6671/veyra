"""P0-08 最终真实包定向校验：保护用户拿到一致、可检测破损的完整组件组合。

输入必须是本轮真实构建的 .app，不能用空fixture/mock通过签名验证。
只复制到自有临时目录修改；从不打开GUI或运行helper服务。
"""
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest

TOOLS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("p008", TOOLS / "p0-08-package.py")
p008 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p008)


class PackageTests(unittest.TestCase):
    def setUp(self):
        # 未给真实包即失败，不把skip或零测试当成工程PASS。
        self.source = Path(os.environ["VEYRA_P008_APP"]).resolve()
        self.directory = tempfile.TemporaryDirectory(prefix="veyra-p008-package-test-")
        self.app = Path(self.directory.name) / "Candidate.app"
        subprocess.run(["/bin/cp", "-cR", str(self.source), str(self.app)], check=True, timeout=60)
        self.addCleanup(self.directory.cleanup)

    def test_final_signed_bundle_matches_embedded_resources(self):
        """用户交接包含四个最终arm64 ad-hoc文件，完整资源封印有效。"""
        receipt = p008.verify(self.app)
        self.assertEqual(receipt["result"], "PASS_STATIC_ONLY")
        self.assertEqual(len(receipt["final_binaries"]), 4)
        # 固定内核实际minos=26.0，不能把Rust目标15.0当成整体包支持范围。
        self.assertEqual(receipt["manifest"]["minimum_system_version"], "26.0")
        self.assertEqual(receipt["final_binaries"]["Contents/Resources/helper/veyra-sing-box"]["minimum_macos"], [26, 0])
        self.assertEqual(receipt["final_binaries"]["Contents/MacOS/veyra-gpui-prototype"]["minimum_macos"], [15, 0])

    def test_zip_roundtrip_keeps_final_signature_and_resources(self):
        """打包/解压后仍是同一最终可验证包，不仅验证打包前目录。"""
        receipt = json.loads((self.source.parent / "build-receipt.json").read_text())
        package = Path(receipt["package"]["path"])
        self.assertEqual(p008.sha(package), receipt["package"]["sha256"])
        extracted = Path(self.directory.name) / "extracted"
        subprocess.run(["ditto", "-x", "-k", str(package), str(extracted)], check=True, timeout=60)
        rebuilt = p008.verify(extracted / "VeyraPrototype.app")
        # codesign显示含绝对路径，复制/解压后只比较实际字节身份与内嵌绑定。
        self.assertEqual(rebuilt["manifest"], receipt["manifest"])
        for relative, identity in rebuilt["final_binaries"].items():
            self.assertEqual(identity["sha256"], receipt["final_binaries"][relative]["sha256"])

    def test_corrupt_kernel_is_rejected_before_execution(self):
        """包内内核被改动时拒绝，不执行破损文件或把签名当作摘要。"""
        kernel = self.app / "Contents/Resources/helper/veyra-sing-box"
        with kernel.open("r+b") as stream:
            stream.seek(128)
            stream.write(b"broken")
        with self.assertRaisesRegex(AssertionError, "最终嵌入资源不匹配"):
            p008.verify(self.app)

    def test_missing_or_extra_resource_is_rejected(self):
        """用户拿到缺组件或夹入未知资源的包不能被判为完整。"""
        resource = self.app / "Contents/Resources/Veyra-LICENSE"
        old = resource.read_bytes()
        resource.unlink()
        with self.assertRaisesRegex(AssertionError, "资源缺失"):
            p008.verify(self.app)
        resource.write_bytes(old)
        (self.app / "Contents/Resources/unknown").write_text("extra")
        with self.assertRaisesRegex(AssertionError, "未知文件"):
            p008.verify(self.app)

    def test_mixed_application_version_is_rejected(self):
        """Info.plist与内嵌发行版本不同不能被交接。"""
        path = self.app / "Contents/Info.plist"
        info = plistlib.loads(path.read_bytes())
        info["CFBundleShortVersionString"] = "9.9.9"
        path.write_bytes(plistlib.dumps(info))
        with self.assertRaises(AssertionError):
            p008.verify(self.app)

    def test_signed_manifest_tampering_is_rejected(self):
        """资源摘要一致但封印清单被改动时仍由最终codesign拒绝。"""
        path = self.app / "Contents/Resources/package-manifest.json"
        binding = json.loads(path.read_text())
        binding["helper_version"] = "9.9.9"
        path.write_text(json.dumps(binding))
        with self.assertRaisesRegex(RuntimeError, "command failed"):
            p008.verify(self.app)

    def test_wrong_official_archive_is_rejected_without_build_or_output(self):
        """错误内核来源在提取/执行/构建前被拒绝。"""
        archive = Path(self.directory.name) / "wrong.tar.gz"
        archive.write_bytes(b"wrong official kernel")
        output = p008.REPO / "target/p0-08-wrong-archive-must-not-exist"
        self.assertFalse(output.exists())
        with self.assertRaisesRegex(AssertionError, "官方 archive SHA256"):
            p008.build(archive, output)
        self.assertFalse(output.exists())

    def test_optimized_python_still_rejects_wrong_archive_and_corrupt_bundle(self):
        """优化解释器不能跳过内核来源或包摘要检查，交付仍须拒绝。"""
        archive = Path(self.directory.name) / "wrong.tar.gz"
        archive.write_bytes(b"wrong official kernel")
        output = p008.REPO / "target" / Path(self.directory.name).name
        rejected = subprocess.run(
            [sys.executable, "-O", str(TOOLS / "p0-08-package.py"), "build", "--kernel-archive", str(archive), "--output", str(output)],
            capture_output=True, text=True, timeout=30,
        )
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn("官方 archive SHA256", rejected.stderr)
        self.assertFalse(output.exists())
        kernel = self.app / "Contents/Resources/helper/veyra-sing-box"
        with kernel.open("r+b") as stream:
            stream.seek(128)
            stream.write(b"broken")
        rejected = subprocess.run(
            [sys.executable, "-O", str(TOOLS / "p0-08-package.py"), "verify", str(self.app)],
            capture_output=True, text=True, timeout=30,
        )
        self.assertNotEqual(rejected.returncode, 0)
        self.assertIn("最终嵌入资源不匹配", rejected.stderr)


if __name__ == "__main__":
    unittest.main()
