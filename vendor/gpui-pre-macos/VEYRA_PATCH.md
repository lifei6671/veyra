# Veyra 原始字体子集补丁

基线：crates.io `gpui-pre-macos 0.3.7`，crate checksum `5a43af845b260b09393e923c4f1e1a67a10fcfc847b4815192bbfd02ed9fe725`，上游 Zed revision `1a28cff4b409169bac058bca40dfbfeb7621d19b`，Apache-2.0。`LICENSE-APACHE` 原件保留，`UPSTREAM.json` 保留所有复制文件的原 SHA256。Cargo 仅通过根 `[patch.crates-io]` 绑定，没有升级依赖。

当前补丁仅修改 `src/text_system.rs` 与 `src/open_type.rs`：

- `add_fonts` 从原字节创建 CoreText descriptor，保留内存资源 face；不注册、不安装、不改字体。
- 缺 `m` 的 face 仍不能成为 primary，维持 editor 的 `m` 度量契约；与 primary 同 family 的这类 face 作为直接 descriptor cascade。
- 显式 `FontFallbacks` 中含缺 `m` 内存子集的 family，按调用者顺序加入直接 descriptor；其中完整 face 按原 matching 选择一个，缺 `m` 子集全部作为 fallback。普通完整字体 family 继续原名称/trait路径。
- CoreText run FontId 按统一字号后的实际 `CTFont` 相等/哈希缓存，不能以 PostScript 名合并不同原子集。FontKit 度量与 raster 直接保留 shape 的 CTFont。
- 新字体加入后清空 family/selection 缓存，已有 FontId 继续有效；后续重新选择可取得新 cascade。

公共 API 不变：桌面 `add_fonts` 加入原字节，主字体选择原件实际 family `MiSans VF`；Noto 等跨 family fallback 必须使用字体的真实 family，而不是 CSS 别名。此补丁不模拟网页 unicode-range；调用方必须核对原 CSS 每个子集和真实 coverage，并保留所有原件及许可。

新增三项本地字体契约测试保护原94/95的真实 glyph/advance/bounds/raster、同 PS 身份、重复不同字号缓存、缺 `m` 不成为 primary 以及新增子集后 cascade 更新，以及名称/direct descriptor混合列表的前后顺序。原94 bitmap对照使用另一独立renderer直接注入raw原件，不经过native FontId/cache；仍共享底层CPU raster算法，不证明Chrome raster等价。测试不启动 App、Runtime 或网络。原上游 BOM/ZWNJ 测试保留。


## 原 Noto Flags 最小内部接线（Host明确批准）

在已有原子集补丁上，仅 macOS text_system.rs与私有noto_flags.rs新增原Noto Flags资源分支；复用锁定rustybuzz0.20.1/其ttf-parser0.25.1及已有image/CoreGraphics，不改字体bytes/系统安装/框架版本。既有add_fonts+实际family fallback接FontId、原GSUB/UTF8 cluster、原CBLC/CBDT strike advance/bearing和emoji BGRA atlas；缺m原Noto不能成为primary。没有新的公共API或通用bitmap字体框架。

10项实际字体合同和当前source/test SHA见任务证据resumed-native/FONT-NOTO-OWNER-RESULT.md与font-owner-noto-final-patch-identity.json。单RI与ccmp=0保持原grapheme cluster；未命中Noto显式fallback的native路径保持。React/Native像素和baseline尚待独立同态验收；mixed bidi未验，不声称全部Unicode等价。中间FAIL/原CoreText加载FAIL均保留。


### Noto 小尺寸原位图插值

真实窗口GPU readback逐像素核实原bitmap→atlas管线后，同态sRGB原React对照确认默认CG縮小插值仍有细边缘残差。Noto私有CGContext仅设置 `CGInterpolationQualityHigh`，不改原件/metrics/advance/bounds/位置或其他native字体路径。六glyph完整支持轮廓bbox未扩大，原十项字体合同仍10 PASS；真实新的Native候选与正式页面视觉仍待独立验收。证据见 resumed-native/FONT-NOTO14-GPU-INVESTIGATION.md / font-owner-noto15-final-identity.json，保留全部旧FAIL与author/layout/ICC/JPEG条件错配记录。
