# Desktop font provenance

Veyra uses MiSans Fonts (MiSans, Version 4.009) by Xiaomi. The unmodified official Regular/Medium/Semibold/Bold faces are embedded only as part of Veyra; it is not offered as a standalone font download.

- Official source: https://hyperos.mi.com/font/en/download/
- Official archive: https://hyperos.mi.com/font-download/MiSans.zip
- License: https://hyperos.mi.com/font-download/MiSans字体知识产权许可协议.pdf
- TTF SHA256: 0ddef90648998900175cfdca9a6f087a2544c182f130b0ad4f7e94a03a115e79
- Family/PostScript: MiSans VF / MiSansVF; version 4.009.

The official agreement permits use in apps and distribution of the resulting work, requires attribution, and prohibits standalone redistribution and adaptation of the font. The font here is unchanged. Preserve this attribution with the application.

OpenBox's repository WOFF2 subsets report MiSans VF 4.003. GPUI's current macOS font loader skips subsets without an `m` glyph and deduplicates identical PostScript names, so registering those web subsets does not reproduce browser unicode-range composition. Host explicitly authorized the official full 4.009 family for this task. The version difference remains visible in the fidelity matrix; it is not called byte-identical to the reference assets.

The existing platform emoji cascade remains enabled (Apple Color Emoji on macOS when NotoEmoji is unavailable). No font binaries are placed in evidence or exported in chat.

GPUI matches face weights but does not expand a single VF wght axis. Runtime UI therefore uses official static Regular/Medium/Semibold/Bold faces (their actual OS/2 weights are 330/380/520/630) from the same 4.009 archive, all unmodified; the VF identity above records the inspected source, not a separately shipped file.

- MiSans-Regular.ttf SHA256: 9c120f0a849bc0aa5048daae2a3c0f6eecd828b5b33fce682a9622833f5feea6
- MiSans-Medium.ttf SHA256: b03e98374e971594b0b7a9706d0704241f76e1b88556cdda79c5039ef8a638d1
- MiSans-Semibold.ttf SHA256: 77c23f31ae124867778344970155a0c8d34a89897dedaab81aeee82ff00a4ce6
- MiSans-Bold.ttf SHA256: d0c1d327952ed935e86fb78a97a6c182b44f2c2b08777326786b1f8b26d1fe1e

## P2-08 Latin 原件加载实验（2026-10-10）

原始 `src/openbox/assets/fonts/MiSans-VF.95-B0-MMqvr.woff2` 可由本机 GPUI/CoreText 原样加载；只有不含 m 的其它子集与相同 PostScript 去重存在障碍，不再笼统称全部 WOFF2 不可用。代理页 Selector/URLTest、协议和数值文本显式使用原始 Latin family；搜索使用同 family 并回退至现有 MiSans。其它页面不更换字体。未生成、重命名、拼接、子集化或改写任何字体文件。

只读独立实验 originalfontprobe3 无 Store/Runtime：registration Ok；Selector12宽75.300 vs React75.3203125；URLTest12宽76.284004 vs React76.3046875，仍有 CoreText/浏览器小数度量差。文件名和 family 中的 VF 不是可变字体证明；本轮原件 95/94 的 WOFF2 目录均无 fvar/avar/gvar，不应称单 VF face 轴等价，中文和中等字重仍使用未修改官方静态家族，混合文本、字重和 fallback 差异须单独审查。实验源码和结果保存在 resumed-native；临时 example 已删除。

## P2-08 实际字体事实修正（2026-10-10）

独立 WOFF2 目录解析确认：原始 95 Latin 与 94 CJK 各含 10 个 SFNT 表，均不含 fvar、avar、gvar、HVAR、MVAR、VVAR、STAT、CFF2。结论仅针对这两个文件，不推断全部 96 子集。浏览器 CSS 的 `font-weight:100 900` 声明不改变文件事实；CoreText 原始 95 的 OS/2 weight 为 400。

官方完整 4.009 VF 的 wght 范围为 150–700、默认 330；命名实例 Normal=305、Regular=330、Medium=380、Semibold=520、Bold=630。不能把静态文件名自动映射为 400/500/600/700。Normal 只用于隔离实验，未加入产品。

同文本 14px 浏览器实测搜索占位：原始复合字体 179.3046875，官方 Regular 179.2421875，Medium 179.375，Normal 179.1328125。Selector/URLTest 混合元数据的差异更大，所以不能用搜索宽度接近来证明全页等价或全局替换。当前 GPUI Search 使用原始 Latin family 加 MiSans fallback；同 API cascade 和 glyph metrics 实验指向官方 Medium，私有 CTFont 没有直接读取。这个 fallback 仍是视觉 FAIL，不宣称已等价。

证据：`resumed-native/woff2-directory-probe-repair1-result.json`、`font-dom-measurement.json`、`font-dom-platform-facts.json` 与独立 `VISUAL_REVIEW.md`。所有字体二进制保持原件，不修改、合并、子集化、重命名或安装系统字体。

## P2-08 原 MiSans/Noto 当前接线（2026-10-10）

原 96 个 MiSans 4.003 子集已由 Host 批准的 macOS 字体 owner 接通；代理页、搜索、Sidebar 和共享 Tooltip 使用 `MiSans VF` 原子集，显式 fallback `Noto Color Emoji Flags`，其它内容区保持既有官方静态字重。前文关于原子集/Apple fallback 的失败记录属于较早候选事实，保留追溯，不表示当前消费仍使用替代字体。

Noto 使用未改原件 `src/openbox/assets/fonts/NotoColorEmoji-flagsonly-CWWDk9km.ttf`，SHA256 `bccb829e97ca446771231993cf5bff3449974851201c497be6d6d52e906aae7b`；name table Copyright 2022 Google Inc.、family Noto Color Emoji Flags、PostScript NotoColorEmojiFlags。字体来源为原 OpenBox 资源；[Noto Emoji 上游说明](https://github.com/googlefonts/noto-emoji#license)将字体列为 SIL Open Font License 1.1，工具与多数图片的 Apache 许可不可替代字体许可。保留原件及此来源/许可归属，不重写、重命名、系统安装或独立再分发字体。

原件缺 m，仅作显式 fallback；不让它成为 GPUI primary。仅在该 fallback 的位置优先级允许时，用原 GSUB、原 CBLC/CBDT 和既有 GPUI 彩色 glyph atlas 绘制，复用已锁定依赖。10 项字体合同及实际隔离窗口出字已执行；约 1 物理像素的纵向差、小字号边缘、mixed bidi 与其它未验 Unicode 仍不能称完全一致。详见 resumed-native/FONT-NOTO-OWNER-RESULT.md、noto11-probe-identity.json、noto11-font-contact-sheet.png 和独立 VISUAL_REVIEW.md；这不是整页验收通过。


## 全局同源字体消费（2026-10-11，用户明确要求）

Theme、App Shell、共享按钮/输入/下拉、普通弹窗、设置页、Toast和Tooltip现在共用原96个MiSans4.003未改子集与显式Noto Flags fallback；CSS字重语义集中于ui/tokens.rs（400/500/600/700，Toast650）。先前使用4.009静态face内部330/380/520/630的内容区消费者全部迁移，静态字体资产/注册仅保留历史来源与既有加载，不作为当前UI主字体或等价证明。原版明确SFMono/Menlo的代码、URL/凭据等内容保留等宽例外，详见本轮font-source-audit及React源码锚点。未改字体字节、字体owner、Cargo或系统字体。

实际Glyph run/advance、最终原生截图与CSS computed记录见resumed-native/policy101-*；统一字体族不宣称CoreText与浏览器所有字体像素完全一致，未测的页面/状态仍须标明。
