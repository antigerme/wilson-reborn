// SPDX-License-Identifier: GPL-3.0-or-later
// The macOS screensaver view for Wilson Reborn. It drives the Rust engine through the
// `wilson-saver` C FFI and blits each 640x480 RGBA frame into the ScreenSaverView,
// letterboxed to 4:3. Build it into a `.saver` bundle with macos/build-saver.sh.

#import <ScreenSaver/ScreenSaver.h>
#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>

// --- wilson-saver C FFI (see crates/wilson-saver/src/lib.rs) ---
extern void *wilson_saver_new(void);
extern uint32_t wilson_saver_next_frame(void *ctx, uint8_t *out, size_t out_len);
extern void wilson_saver_free(void *ctx);

static const size_t kW = 640;
static const size_t kH = 480;

@interface WilsonRebornView : ScreenSaverView {
    void *_ctx;             // Rust runtime handle (NULL if data missing)
    uint8_t *_rgba;         // kW*kH*4 frame buffer the FFI fills
    CFTimeInterval _nextAt; // when to pull the next engine frame
}
@end

@implementation WilsonRebornView

- (instancetype)initWithFrame:(NSRect)frame isPreview:(BOOL)isPreview {
    self = [super initWithFrame:frame isPreview:isPreview];
    if (self) {
        // Poll often; the engine's per-frame delay decides when we actually advance.
        [self setAnimationTimeInterval:1.0 / 60.0];
        _ctx = wilson_saver_new();
        _rgba = (uint8_t *)calloc(kW * kH * 4, 1);
        _nextAt = 0.0;
    }
    return self;
}

- (void)dealloc {
    if (_ctx) {
        wilson_saver_free(_ctx);
        _ctx = NULL;
    }
    free(_rgba);
    _rgba = NULL;
}

- (void)animateOneFrame {
    if (!_ctx || !_rgba) {
        return;
    }
    CFTimeInterval now = CACurrentMediaTime();
    if (now >= _nextAt) {
        uint32_t delayMs = wilson_saver_next_frame(_ctx, _rgba, kW * kH * 4);
        if (delayMs == 0) {
            delayMs = 100; // defensive: never busy-spin
        }
        _nextAt = now + (double)delayMs / 1000.0;
        [self setNeedsDisplay:YES];
    }
}

- (void)drawRect:(NSRect)rect {
    [[NSColor blackColor] setFill];
    NSRectFill(rect);

    NSRect b = [self bounds];

    if (!_ctx || !_rgba) {
        // No data: tell the user where to put it instead of a black screen.
        NSDictionary *attrs = @{
            NSForegroundColorAttributeName : [NSColor whiteColor],
            NSFontAttributeName : [NSFont systemFontOfSize:(self.isPreview ? 9.0 : 18.0)]
        };
        NSString *msg = @"Wilson Reborn — place the original RESOURCE.MAP + RESOURCE.001 in\n"
                        @"~/Library/Application Support/WilsonReborn/";
        [msg drawInRect:NSInsetRect(b, 16, 16) withAttributes:attrs];
        return;
    }

    // Wrap the RGBA buffer in a bitmap rep (top-left origin) — it handles orientation.
    unsigned char *planes[1] = {_rgba};
    NSBitmapImageRep *rep =
        [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:planes
                                                pixelsWide:(NSInteger)kW
                                                pixelsHigh:(NSInteger)kH
                                             bitsPerSample:8
                                           samplesPerPixel:4
                                                  hasAlpha:YES
                                                  isPlanar:NO
                                            colorSpaceName:NSDeviceRGBColorSpace
                                               bytesPerRow:(NSInteger)(kW * 4)
                                              bitsPerPixel:32];

    // Letterbox the 4:3 frame into the view, with crisp (nearest) pixels.
    CGFloat scale = fmin(NSWidth(b) / (CGFloat)kW, NSHeight(b) / (CGFloat)kH);
    CGFloat dw = (CGFloat)kW * scale;
    CGFloat dh = (CGFloat)kH * scale;
    NSRect dst = NSMakeRect((NSWidth(b) - dw) / 2.0, (NSHeight(b) - dh) / 2.0, dw, dh);

    [rep drawInRect:dst
              fromRect:NSZeroRect
             operation:NSCompositingOperationCopy
              fraction:1.0
        respectFlipped:YES
                 hints:@{NSImageHintInterpolation : @(NSImageInterpolationNone)}];
}

- (BOOL)hasConfigureSheet {
    return NO;
}

- (NSWindow *)configureSheet {
    return nil;
}

@end
