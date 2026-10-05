# 字体探针与明确差异（未宣称等价）

2026-10-04，本机 CoreGraphics/CoreText 原始内存加载探针，结果见 `font-probe.txt`。

- 本地 90 个 MiSans WOFF2 分片均能由 CGFont 解码，所以“macOS 不支持 WOFF2”不是成立的理由。
- 分片共用 PostScript 名 `MiSansVF`。只有 `.95` 分片包含 `m`，而 `.95` 不包含本次探针的“面板设置语言”。中文分散在其余分片。
- 当前 GPUI macOS 文本系统 `gpui-pre-macos 0.3.7/src/text_system.rs`，`load_family` 明确跳过没有 `m` 字形的字体，并按 PostScript 名去重。直接把 React 的所有分片传给 `add_fonts` 不能复制 CSS `unicode-range` 的字体拼接。
- NotoColorEmoji flagsonly TTF 经同一 CGFont(dataProvider) 路径返回 unsupported。
- 因此当前保持 `.SystemUIFont`，中文使用系统 fallback。正文高度仍为 14/20；笔画、字宽、字重与 MiSans 有可见差异。不能写成“完全一致”。
- 字体 name 表仅读到 Xiaomi copyright，未发现仓库内单独 MiSans license 文件或 name13/14 license 字段；本轮没有重新分发或改写字体。没有下载字体，也没有修改系统字体注册。

探针仅进程内读文件、查询字形，不安装字体。若要消除此差异，需要许可明确的完整 MiSans 字体或独立审查 GPUI 字体分片支持；本轮不擅自修改依赖源码。
