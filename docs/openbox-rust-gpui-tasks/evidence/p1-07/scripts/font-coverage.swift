import Foundation
import CoreGraphics
import CoreText
let dir="src/openbox/assets/fonts"
for file in try FileManager.default.contentsOfDirectory(atPath:dir).sorted() where file.hasSuffix("woff2") || file.hasSuffix("ttf") {
 let data=try Data(contentsOf:URL(fileURLWithPath:dir+"/"+file))
 guard let cg=CGFont(CGDataProvider(data:data as CFData)!) else {print("\(file) CGFont_UNSUPPORTED");continue}
 let font=CTFontCreateWithGraphicsFont(cg,14,nil,nil)
 let chars=Array("m面板设置语言".utf16);var glyphs=[CGGlyph](repeating:0,count:chars.count)
 CTFontGetGlyphsForCharacters(font,chars,&glyphs,chars.count)
 print("\(file) postscript=\(CTFontCopyPostScriptName(font)) glyphs=\(glyphs)")
}
