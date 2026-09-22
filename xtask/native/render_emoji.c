// Rasterizes a single emoji (or ZWJ/skin-tone sequence) to a transparent
// PNG using the system's Apple Color Emoji font, via CoreText's normal
// text-layout path (CTLine) so ligature substitution for known ZWJ
// sequences happens exactly as it would anywhere else on macOS.
//
// See PLAN.md Phase 3 for why this exists (Emoji 18 rendering can't
// depend on iamcal's vendor sprite sheets, which lag a fresh Unicode
// release by months) and the documented limitation of the coverage check
// below (accurate for new single-codepoint emoji; optimistic for a
// brand-new multi-codepoint ZWJ combination whose components are each
// individually already supported).
//
// Pure C against CoreText/CoreGraphics/CoreFoundation/ImageIO — no
// Objective-C runtime needed. Built by xtask/build.rs.

#include <CoreText/CoreText.h>
#include <CoreGraphics/CoreGraphics.h>
#include <CoreFoundation/CoreFoundation.h>
#include <ImageIO/ImageIO.h>
#include <stdlib.h>

// Returns 0 on success (PNG written), 1 if the installed font can't
// render this codepoint sequence (no PNG written — caller should fall
// back to no custom icon), or -1 on an unexpected error.
int render_emoji_png(const char *utf8_text, double point_size, int canvas_px, const char *out_path) {
    CFStringRef text = CFStringCreateWithCString(kCFAllocatorDefault, utf8_text, kCFStringEncodingUTF8);
    if (!text) {
        return -1;
    }

    CTFontRef font = CTFontCreateWithName(CFSTR("Apple Color Emoji"), point_size, NULL);
    if (!font) {
        CFRelease(text);
        return -1;
    }

    CFIndex length = CFStringGetLength(text);
    UniChar *units = malloc(sizeof(UniChar) * (size_t)length);
    CGGlyph *glyphs = malloc(sizeof(CGGlyph) * (size_t)length);
    CFStringGetCharacters(text, CFRangeMake(0, length), units);
    bool mapped = CTFontGetGlyphsForCharacters(font, units, glyphs, length);
    free(units);
    free(glyphs);
    if (!mapped) {
        CFRelease(font);
        CFRelease(text);
        return 1;
    }

    CFMutableAttributedStringRef attr = CFAttributedStringCreateMutable(kCFAllocatorDefault, 0);
    CFAttributedStringReplaceString(attr, CFRangeMake(0, 0), text);
    CFAttributedStringSetAttribute(attr, CFRangeMake(0, CFAttributedStringGetLength(attr)), kCTFontAttributeName, font);
    CTLineRef line = CTLineCreateWithAttributedString(attr);
    CGRect bounds = CTLineGetBoundsWithOptions(line, kCTLineBoundsUseGlyphPathBounds);

    CGColorSpaceRef colorspace = CGColorSpaceCreateDeviceRGB();
    CGContextRef ctx = CGBitmapContextCreate(
        NULL, (size_t)canvas_px, (size_t)canvas_px, 8, (size_t)canvas_px * 4,
        colorspace, kCGImageAlphaPremultipliedLast);
    CGColorSpaceRelease(colorspace);
    if (!ctx) {
        CFRelease(line);
        CFRelease(attr);
        CFRelease(font);
        CFRelease(text);
        return -1;
    }
    CGContextClearRect(ctx, CGRectMake(0, 0, canvas_px, canvas_px));

    double x = ((double)canvas_px - bounds.size.width) / 2.0 - bounds.origin.x;
    double y = ((double)canvas_px - bounds.size.height) / 2.0 - bounds.origin.y;
    CGContextSetTextPosition(ctx, x, y);
    CTLineDraw(line, ctx);

    CGImageRef image = CGBitmapContextCreateImage(ctx);
    int result = -1;
    if (image) {
        CFStringRef path_str = CFStringCreateWithCString(kCFAllocatorDefault, out_path, kCFStringEncodingUTF8);
        CFURLRef url = CFURLCreateWithFileSystemPath(kCFAllocatorDefault, path_str, kCFURLPOSIXPathStyle, false);
        CGImageDestinationRef dest = CGImageDestinationCreateWithURL(url, CFSTR("public.png"), 1, NULL);
        if (dest) {
            CGImageDestinationAddImage(dest, image, NULL);
            result = CGImageDestinationFinalize(dest) ? 0 : -1;
            CFRelease(dest);
        }
        CFRelease(url);
        CFRelease(path_str);
        CGImageRelease(image);
    }

    CGContextRelease(ctx);
    CFRelease(line);
    CFRelease(attr);
    CFRelease(font);
    CFRelease(text);
    return result;
}
