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

GPUI matches face weights but does not expand a single VF wght axis. Runtime UI therefore uses official static 400/500/600/700 faces from the same 4.009 archive, all unmodified; the VF identity above records the inspected source, not a separately shipped file.

- MiSans-Regular.ttf SHA256: 9c120f0a849bc0aa5048daae2a3c0f6eecd828b5b33fce682a9622833f5feea6
- MiSans-Medium.ttf SHA256: b03e98374e971594b0b7a9706d0704241f76e1b88556cdda79c5039ef8a638d1
- MiSans-Semibold.ttf SHA256: 77c23f31ae124867778344970155a0c8d34a89897dedaab81aeee82ff00a4ce6
- MiSans-Bold.ttf SHA256: d0c1d327952ed935e86fb78a97a6c182b44f2c2b08777326786b1f8b26d1fe1e
