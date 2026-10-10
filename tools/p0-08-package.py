#!/usr/bin/env python3
"""P0-08 本地 Apple Silicon 原型包：复用 P0-03 构建路线，不安装/运行 GUI。

只消费现有官方固定 archive，签名从内向外；最后的包摘要写在外部发行清单。
保留官方内核已有的 linker ad-hoc，不能重签后破坏 P0-05 的固定内核摘要。
内嵌 manifest 不计算自身或最终主可执行文件 SHA，避免资源封印的循环依赖；
最终主文件 SHA 在外部构建收据中绑定，codesign --strict 检查完整资源封印。
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tarfile
import tomllib

REPO = Path(__file__).resolve().parents[1]
REPOSITORY = "lifei6671/veyra"
KERNEL_VERSION = "1.14.0"
# 直接消费 P0-04 的已审查身份；不维护另一份内核下载/版本选择框架。
import importlib.util

spec = importlib.util.spec_from_file_location("p004", REPO / "tools/p0-04-kernel-probe.py")
p004 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p004)
spec = importlib.util.spec_from_file_location("p005", REPO / "tools/p0-05-helper-probe.py")
p005 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p005)


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as stream:
        while data := stream.read(1024 * 1024):
            digest.update(data)
    return digest.hexdigest()


def run(command, timeout=600):
    result = subprocess.run(command, cwd=REPO, env={**os.environ, "MACOSX_DEPLOYMENT_TARGET": "15.0"}, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"command failed {command}: {result.returncode}\n{result.stdout}\n{result.stderr}")
    return result.stdout + result.stderr


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n")


def version(crate):
    return tomllib.loads((REPO / f"crates/{crate}/Cargo.toml").read_text())["package"]["version"]


def minimum_macos(path):
    # 以最终Mach-O为准，不以Rust部署变量猜测第三方内核的最低系统要求。
    output = run(["otool", "-l", str(path)], 30)
    match = re.search(r"(?m)^\s+(?:minos|version) (\d+)\.(\d+)", output)
    if not match:
        raise RuntimeError(f"缺少Mach-O最低系统版本: {path}")
    return tuple(map(int, match.groups()))


def verify(app):
    """只读验证最终嵌入资源与签名；不触发 Gatekeeper、helper 或 GUI。"""
    # 使用显式失败，python -O 也必须保留摘要、版本与签名检查。
    app = Path(app)
    manifest_path = app / "Contents/Resources/package-manifest.json"
    manifest = json.loads(manifest_path.read_text())
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    if not (manifest["schema"] == 1 and manifest["repository"] == REPOSITORY):
        raise AssertionError("包版本或结构校验失败")
    if not (manifest["platform"] == "macos-arm64" and manifest["sing_box_version"] == KERNEL_VERSION):
        raise AssertionError("包版本或结构校验失败")
    if not (manifest["resources_version"] == manifest["app_version"] == info["CFBundleShortVersionString"] == info["CFBundleVersion"]):
        raise AssertionError("包版本或结构校验失败")
    if not (info["CFBundleExecutable"] == "veyra-gpui-prototype"):
        raise AssertionError("包版本或结构校验失败")
    if not (info["LSMinimumSystemVersion"] == manifest["minimum_system_version"]):
        raise AssertionError("包版本或结构校验失败")
    if not (manifest["helper_kind"] == "p0-05-prototype-not-production"):
        raise AssertionError("包版本或结构校验失败")
    expected = set(manifest["files"])
    actual = {p.relative_to(app).as_posix() for p in (app / "Contents/Resources").rglob("*") if p.is_file() and p != manifest_path}
    if not (actual == expected):
        raise AssertionError("资源缺失或夹入未知文件")
    if not (not any(p.is_symlink() for p in app.rglob("*"))):
        raise AssertionError("包内不接受符号链接")
    for relative, identity in manifest["files"].items():
        path = app / relative
        if not (path.stat().st_size == identity["size"] and sha(path) == identity["sha256"]):
            raise AssertionError(f"最终嵌入资源不匹配: {relative}")
    signatures = {}
    for relative in ["Contents/MacOS/veyra-gpui-prototype", "Contents/Resources/bin/veyra-update-prototype", "Contents/Resources/helper/veyra-helper-prototype", "Contents/Resources/helper/veyra-sing-box"]:
        path = app / relative
        if not (run(["lipo", "-archs", str(path)], 30).strip() == "arm64"):
            raise AssertionError(relative)
        run(["codesign", "--verify", "--strict", str(path)], 30)
        signature = run(["codesign", "-dv", "--verbose=4", str(path)], 30)
        if not ("Signature=adhoc" in signature):
            raise AssertionError(relative)
        signatures[relative] = {"sha256": sha(path), "signature": signature, "minimum_macos": list(minimum_macos(path))}
    minimum = max(tuple(identity["minimum_macos"]) for identity in signatures.values())
    if not (manifest["minimum_system_version"] == ".".join(map(str, minimum))):
        raise AssertionError("包最低系统版本必须覆盖全部嵌入二进制")
    run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)], 30)
    if not (sha(app / "Contents/Resources/helper/veyra-sing-box") == p005.BINARY_DIGEST):
        raise AssertionError("内核与 P0-05 helper 固定摘要不匹配")
    kernel_output = run([str(app / "Contents/Resources/helper/veyra-sing-box"), "version"], 30)
    if not (f"sing-box version {KERNEL_VERSION}" in kernel_output and "darwin/arm64" in kernel_output):
        raise AssertionError("包版本或结构校验失败")
    return {"result": "PASS_STATIC_ONLY", "bundle": str(app.resolve()), "manifest": manifest, "final_binaries": signatures, "kernel_version_output": kernel_output, "browser_quarantine_first_open": "NOT_RUN"}


def build(archive, output):
    if not (run(["uname", "-m"], 30).strip() == "arm64"):
        raise AssertionError("仅 Apple Silicon 原型")
    archive = Path(archive).resolve()
    if not (sha(archive) == p004.DIGEST):
        raise AssertionError("官方 archive SHA256 不匹配，拒绝执行")
    output = Path(output).resolve()
    # 不覆盖已有候选或用户文件；输出必须在当前独立 worktree 的生成目录。
    if not (output.is_relative_to(REPO / "target")):
        raise AssertionError("候选输出必须在本 worktree/target 下")
    output.mkdir(parents=True, exist_ok=False)
    build_commands = [
        ["cargo", "+1.99.0", "build", "--offline", "--locked", "-p", "veyra-gpui-prototype"],
        # 当前原型bin未消费空lib，Cargo不传播native link；只在此构建命令
        # 补齐现有build.rs的三项链接，不改Helper/IPC owner源码或正式入口。
        ["cargo", "+1.99.0", "rustc", "--offline", "--locked", "-p", "veyra-helper-prototype", "--features", "p0-05-prototype", "--bin", "veyra-helper-prototype", "--", "-l", "static=p005_network", "-l", "framework=Foundation", "-l", "framework=SystemConfiguration"],
        ["cargo", "+1.99.0", "build", "--offline", "--locked", "-p", "veyra-core", "--features", "p0-06-prototype", "--example", "p0_08_update"],
    ]
    for index, command in enumerate(build_commands):
        (output / f"build-{index}.log").write_text(run(command))
    app = output / "VeyraPrototype.app"
    resources = app / "Contents/Resources"
    (app / "Contents/MacOS").mkdir(parents=True)
    (resources / "helper").mkdir(parents=True)
    (resources / "bin").mkdir()
    shutil.copy2(REPO / "target/debug/veyra-gpui-prototype", app / "Contents/MacOS/veyra-gpui-prototype")
    shutil.copy2(REPO / "target/debug/veyra-helper-prototype", resources / "helper/veyra-helper-prototype")
    shutil.copy2(REPO / "target/debug/examples/p0_08_update", resources / "bin/veyra-update-prototype")
    # 只读取两个固定 regular member，不解压 archive 的任意路径。
    with tarfile.open(archive) as bundle:
        for upstream, target in [("sing-box", "helper/veyra-sing-box"), ("LICENSE", "sing-box-LICENSE")]:
            member = bundle.getmember(f"sing-box-{KERNEL_VERSION}-darwin-arm64/{upstream}")
            if not (member.isfile()):
                raise AssertionError("内核 archive 成员必须是普通文件")
            with bundle.extractfile(member) as source, (resources / target).open("xb") as destination:
                shutil.copyfileobj(source, destination)
    (resources / "helper/veyra-sing-box").chmod(0o755)
    shutil.copy2(REPO / "LICENSE", resources / "Veyra-LICENSE")
    shutil.copy2(REPO / "src-tauri/icons/icon.icns", resources / "icon.icns")
    shutil.copy2(REPO / "docs/openbox-rust-gpui-tasks/P0-08-installation.md", resources / "INSTALLATION.md")
    app_version = version("veyra-gpui-prototype")
    helper_version = version("veyra-helper")
    minimum_system = ".".join(map(str, max(minimum_macos(p) for p in [app / "Contents/MacOS/veyra-gpui-prototype", resources / "bin/veyra-update-prototype", resources / "helper/veyra-helper-prototype", resources / "helper/veyra-sing-box"])))
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps({
        "CFBundleExecutable": "veyra-gpui-prototype", "CFBundleIdentifier": "me.veyra.gpui-prototype", "CFBundleName": "VeyraPrototype", "CFBundlePackageType": "APPL", "CFBundleIconFile": "icon.icns", "CFBundleShortVersionString": app_version, "CFBundleVersion": app_version, "LSMinimumSystemVersion": minimum_system, "NSHighResolutionCapable": True,
    }))
    # 显式签名每个内嵌 Mach-O，再生成资源摘要；最后签 .app 外壳。
    # 官方 Go 内核已带有效 linker ad-hoc；保留字节以匹配 P0-05 固定摘要。
    if not (sha(resources / "helper/veyra-sing-box") == p005.BINARY_DIGEST):
        raise AssertionError("包版本或结构校验失败")
    run(["codesign", "--verify", "--strict", str(resources / "helper/veyra-sing-box")], 30)
    for relative in ["bin/veyra-update-prototype", "helper/veyra-helper-prototype"]:
        run(["codesign", "--force", "--sign", "-", str(resources / relative)], 30)
    inputs = {}
    for crate in ["veyra-core", "veyra-gpui-prototype", "veyra-helper"]:
        for path in sorted((REPO / "crates" / crate).rglob("*")):
            if path.is_file() and path.suffix in {".rs", ".toml", ".m"}:
                inputs[path.relative_to(REPO).as_posix()] = sha(path)
    for relative in ["Cargo.lock", "Cargo.toml", "tools/p0-08-package.py", "tools/p0-04-kernel-probe.py", "tools/p0-05-helper-probe.py", "docs/openbox-rust-gpui-tasks/P0-08-installation.md", "src-tauri/icons/icon.icns", "LICENSE"]:
        inputs[relative] = sha(REPO / relative)
    write_json(output / "build-inputs.json", inputs)
    binding = {
        "schema": 1, "repository": REPOSITORY, "platform": "macos-arm64", "app_version": app_version, "helper_version": helper_version, "helper_kind": "p0-05-prototype-not-production", "sing_box_version": KERNEL_VERSION, "resources_version": app_version,
        "source_commit": run(["git", "rev-parse", "HEAD"], 30).strip(), "cargo_lock_sha256": sha(REPO / "Cargo.lock"), "build_inputs_sha256": sha(output / "build-inputs.json"), "rustc": run(["rustc", "+1.99.0", "--version"], 30).strip(), "deployment_target": "15.0", "minimum_system_version": minimum_system, "kernel_source": {"url": p004.URL, "archive_sha256": p004.DIGEST},
        "files": {p.relative_to(app).as_posix(): {"sha256": sha(p), "size": p.stat().st_size} for p in sorted(resources.rglob("*")) if p.is_file()},
    }
    write_json(resources / "package-manifest.json", binding)
    run(["codesign", "--force", "--sign", "-", str(app)], 30)
    receipt = verify(app)
    name = f"VeyraPrototype-{app_version}-macos-arm64.zip"
    package = output / name
    run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(app), str(package)], 120)
    release_manifest = {key: binding[key] for key in ["schema", "repository", "app_version", "helper_version", "sing_box_version", "resources_version", "platform", "minimum_system_version", "source_commit", "cargo_lock_sha256"]}
    release_manifest.update(asset_name=name, size=package.stat().st_size, sha256=sha(package))
    write_json(output / "veyra-release.json", release_manifest)
    # 本地结构 fixture，不创建/发布真实 GitHub Release。
    write_json(output / "github-release-fixture.json", {
        "tag_name": "v" + app_version, "html_url": f"https://github.com/{REPOSITORY}/releases/tag/v{app_version}", "body": "P0-08 工程候选：GPUI 能力原型；helper 只作嵌入版本绑定，不安装/启用。下载校验后手动交接；正式 UI/升级由 P6/P7 承接。此文件为本地 fixture，无实际 Release。", "draft": False, "prerelease": False,
        "assets": [{"name": asset, "size": (output / asset).stat().st_size, "browser_download_url": f"https://github.com/{REPOSITORY}/releases/download/v{app_version}/{asset}"} for asset in [name, "veyra-release.json"]],
    })
    receipt.update(build_commands=build_commands, package={"path": str(package), "size": package.stat().st_size, "sha256": sha(package)}, source_tree="UNCOMMITTED_ENGINEERING_CANDIDATE", install="NOT_RUN", gui="NOT_RUN")
    write_json(output / "build-receipt.json", receipt)
    print(json.dumps(receipt["package"], ensure_ascii=False))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    b = commands.add_parser("build")
    b.add_argument("--kernel-archive", required=True)
    b.add_argument("--output", default=str(REPO / "target/p0-08"))
    v = commands.add_parser("verify")
    v.add_argument("app")
    args = parser.parse_args()
    if args.action == "build":
        build(args.kernel_archive, args.output)
    else:
        print(json.dumps(verify(args.app), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
