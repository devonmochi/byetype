#!/usr/bin/env python3
"""生成 byetype 演示 GIF：光标就位，按 F4 录音，气泡依次走完转写与优化，文字落到光标处。

按 2 倍分辨率渲染，README 里用 width 属性按 1 倍尺寸显示，在 Retina 屏上不糊。

用法：
    python3 generate_demo_gif.py [输出路径]

默认输出到 docs/images/demo.gif。
"""

import glob
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = os.path.dirname(os.path.abspath(__file__))
DEFAULT_OUT = os.path.join(ROOT, "docs", "images", "demo.gif")

SCALE = 2
S = SCALE

# ---------- 画布与配色（气泡配色取自 app 的 bubble.html） ----------
W, H = 444 * S, 150 * S
BG = (22, 22, 23)
TEXT_COLOR = (208, 209, 208)
CARET_COLOR = (205, 205, 205)
CURSOR_COLOR = (246, 238, 248)

C_RECORD = [(255, 71, 87), (255, 107, 129), (255, 71, 87), (196, 69, 105)]
C_THINK = [(95, 39, 205), (108, 99, 255), (162, 155, 254), (95, 39, 205)]
C_OPT = [(10, 189, 227), (72, 219, 251), (116, 185, 255), (10, 189, 227)]
C_DONE = [(46, 204, 113), (85, 239, 196), (0, 184, 148), (46, 204, 113)]

# ---------- 版面（逻辑坐标，绘制时统一乘 S） ----------
TEXT = "欢迎使用byetype"
TEXT_X, TEXT_BASELINE, TEXT_PX = 88, 54, 16
IBEAM_CX, IBEAM_TOP, IBEAM_BOTTOM = 91.5, 44.0, 61.9
CARET_X, CARET_TOP, CARET_BOTTOM = 88.0, 35.0, 53.9
DOT_D = 13
BUBBLE_CX, BUBBLE_CY = 171, 80

ROUND_W, ROUND_H = 34, 34
PILL_W, PILL_H = 104, 30
LABEL = "Thinking..."
GLOW_PAD = 26
SHADOW_DY = 4


def find_font():
    """中文用 PingFang SC，取不到时退回 Hiragino Sans GB。"""
    for path in glob.glob(
        "/System/Library/AssetsV2/com_apple_MobileAsset_Font*/**/PingFang.ttc", recursive=True
    ):
        return path, 3  # index 3 = PingFang SC Regular
    return "/System/Library/Fonts/Hiragino Sans GB.ttc", 0


FONT_FILE, FONT_SC_REGULAR = find_font()
LABEL_FONT_FILE = "/System/Library/Fonts/Supplemental/Arial Bold.ttf"

# ---------- 时间轴（毫秒） ----------
# TEMPO 是整体节奏系数：1.0 保持原始节奏，调小整体变快。只改这一个数就能调速度。
TEMPO = 0.3

FRAME_MS = 30
IDLE_MS = int(520 * TEMPO)
POP_MS = int(600 * TEMPO)
RECORD_MS = int(5200 * TEMPO)
MORPH_MS = int(320 * TEMPO)
THINK_MS = int(2600 * TEMPO)
OPT_MS = int(2600 * TEMPO)
CHECK_HOLD_MS = int(1200 * TEMPO)
FADE_MS = int(300 * TEMPO)
HOLD_MS = int(2600 * TEMPO)
WAVE_MS = int(4000 * TEMPO)
CARET_PERIOD_MS = max(520, int(1100 * TEMPO))  # 光标闪烁不跟着压太快

T_POP = IDLE_MS
T_REC_END = T_POP + POP_MS + RECORD_MS
T_MORPH_END = T_REC_END + MORPH_MS
T_THINK_END = T_MORPH_END + THINK_MS
T_OPT_END = T_THINK_END + OPT_MS
T_FADE = T_OPT_END + CHECK_HOLD_MS
TOTAL_MS = T_FADE + HOLD_MS


def cubic_bezier(x1, y1, x2, y2):
    def ease(t):
        t = min(max(t, 0.0), 1.0)
        u = t
        for _ in range(16):
            bx = 3 * (1 - u) ** 2 * u * x1 + 3 * (1 - u) * u * u * x2 + u ** 3
            dx = 3 * (1 - u) ** 2 * x1 + 6 * (1 - u) * u * (x2 - x1) + 3 * u * u * (1 - x2)
            if abs(dx) < 1e-6:
                break
            u = min(max(u - (bx - t) / dx, 0.0), 1.0)
        return 3 * (1 - u) ** 2 * u * y1 + 3 * (1 - u) * u * u * y2 + u ** 3

    return ease


EASE_BOUNCE = cubic_bezier(0.34, 1.56, 0.64, 1)
EASE_SIZE = cubic_bezier(0.4, 0, 0.2, 1)
EASE_WAVE = cubic_bezier(0.25, 0.1, 0.25, 1)


def wave_phase(t):
    """对应 CSS 的 wave 4s ease infinite：渐变位置来回移动。"""
    p = (t % WAVE_MS) / WAVE_MS
    if p < 0.5:
        return EASE_WAVE(p / 0.5)
    return 1.0 - EASE_WAVE((p - 0.5) / 0.5)


def gradient_image(colors, bw, bh, phase):
    """线性渐变按 background-size 400% 取样，相位沿水平方向平移。"""
    total = max(bw * 4, 8)
    colors = list(reversed(colors))  # CSS 270deg：首个色停在右端
    stops = np.linspace(0.0, 1.0, len(colors))
    xs = np.linspace(0.0, 1.0, total)
    line = np.zeros((total, 3), dtype=np.float64)
    for ch in range(3):
        line[:, ch] = np.interp(xs, stops, [c[ch] for c in colors])
    offset = int(round(phase * (total - bw)))
    offset = min(max(offset, 0), total - bw)
    tile = np.repeat(line[offset:offset + bw][None, :, :], bh, axis=0)
    return Image.fromarray(np.clip(tile, 0, 255).astype("uint8"))


def rounded_mask(size, radius):
    mask = Image.new("L", size, 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, size[0] - 1, size[1] - 1], radius=radius, fill=255
    )
    return mask


class Renderer:
    def __init__(self):
        self.text_font = ImageFont.truetype(FONT_FILE, TEXT_PX * S, index=FONT_SC_REGULAR)
        self.label_font = ImageFont.truetype(LABEL_FONT_FILE, 13 * S)
        self.text_width = self.text_font.getlength(TEXT) / S

    # ---------- 文字与光标 ----------
    def draw_cursor(self, base):
        """macOS 的 I 形鼠标指针，空心描边。"""
        layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        col = CURSOR_COLOR + (255,)
        cx = IBEAM_CX * S
        top, bottom = IBEAM_TOP * S, IBEAM_BOTTOM * S
        bar_half = 3.4 * S
        d.rectangle([cx - bar_half, top, cx + bar_half, top + 1.2 * S], fill=col)
        d.rectangle([cx - bar_half, bottom - 1.2 * S, cx + bar_half, bottom], fill=col)
        d.rectangle([cx - 1.6 * S, top, cx - 0.6 * S, bottom], fill=col)
        d.rectangle([cx + 0.6 * S, top, cx + 1.6 * S, bottom], fill=col)
        base.paste(layer, (0, 0), layer)

    def draw_caret(self, base, x, top, bottom, alpha=1.0):
        layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        d.rectangle(
            [x * S, top * S, (x + 1.9) * S, bottom * S],
            fill=CARET_COLOR + (int(255 * alpha),),
        )
        base.paste(layer, (0, 0), layer)

    def draw_text(self, base):
        ImageDraw.Draw(base).text(
            (TEXT_X * S, TEXT_BASELINE * S), TEXT, font=self.text_font,
            fill=TEXT_COLOR, anchor="ls",
        )

    def draw_check(self, base, alpha=1.0):
        layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        col = (255, 255, 255, int(255 * alpha))
        cx, cy = BUBBLE_CX * S, BUBBLE_CY * S
        pts = [(cx - 7 * S, cy + 0.5 * S), (cx - 2.2 * S, cy + 5.5 * S), (cx + 7.5 * S, cy - 6 * S)]
        d.line(pts, fill=col, width=3 * S, joint="curve")
        for p in (pts[0], pts[2]):
            d.ellipse([p[0] - 1.5 * S, p[1] - 1.5 * S, p[0] + 1.5 * S, p[1] + 1.5 * S], fill=col)
        base.paste(layer, (0, 0), layer)

    # ---------- 气泡 ----------
    def bubble(self, colors, phase, bw, bh, scale=1.0, alpha=1.0, dot=False):
        """bw/bh 是逻辑尺寸；气泡主体的中心精确落在 (BUBBLE_CX, BUBBLE_CY)。"""
        bw, bh = int(round(bw * S)), int(round(bh * S))
        radius = min(bw, bh) // 2 if bw == bh else min(15 * S, bh // 2)

        grad = gradient_image(colors, bw, bh, phase)
        mask = rounded_mask((bw, bh), radius)
        shape = Image.new("RGBA", (bw, bh), (0, 0, 0, 0))
        shape.paste(grad, (0, 0), mask)
        if dot:
            sd = ImageDraw.Draw(shape)
            r = DOT_D * S / 2
            sd.ellipse([bw / 2 - r, bh / 2 - r, bw / 2 + r, bh / 2 + r], fill=(255, 255, 255, 255))

        # box-shadow: 0 4px 14px rgba(主色, 0.4)
        glow = Image.new("RGBA", (bw, bh), (0, 0, 0, 0))
        glow.paste(Image.new("RGBA", (bw, bh), colors[1] + (255,)), (0, 0),
                   mask.filter(ImageFilter.GaussianBlur(7 * S)))
        glow.putalpha(glow.getchannel("A").point(lambda v: int(v * 0.4)))

        pad = GLOW_PAD * S
        pad_bottom = pad + SHADOW_DY * S
        layer = Image.new("RGBA", (bw + pad * 2, bh + pad + pad_bottom), (0, 0, 0, 0))
        layer.alpha_composite(glow, (pad, pad + SHADOW_DY * S))
        layer.alpha_composite(shape, (pad, pad))

        if scale != 1.0:
            layer = layer.resize(
                (max(1, int(layer.width * scale)), max(1, int(layer.height * scale))),
                Image.LANCZOS,
            )
        if alpha < 1.0:
            layer.putalpha(layer.getchannel("A").point(lambda v: int(v * alpha)))

        # 以气泡主体的中心对齐到落点，而不是以整层（含发光留白）的中心对齐
        pos = (
            int(round(BUBBLE_CX * S - pad - bw / 2)),
            int(round(BUBBLE_CY * S - pad - bh / 2)),
        )
        return layer, pos

    # ---------- 单帧 ----------
    def frame(self, t):
        base = Image.new("RGB", (W, H), BG)
        is_done = t >= T_OPT_END
        blink_on = int(t // (CARET_PERIOD_MS / 2)) % 2 == 0

        if not is_done:
            self.draw_cursor(base)
            self.draw_caret(base, CARET_X, CARET_TOP, CARET_BOTTOM, 1.0 if blink_on else 0.12)
        else:
            self.draw_text(base)
            self.draw_caret(base, TEXT_X + self.text_width + 8, TEXT_BASELINE - 17,
                            TEXT_BASELINE, 1.0 if blink_on else 0.12)

        if t < T_POP:
            return base

        phase = wave_phase(t)
        label = False
        if t < T_POP + POP_MS:
            p = (t - T_POP) / POP_MS
            e = EASE_BOUNCE(p)
            if p < 0.55:
                scale = 0.4 + 0.66 * e
                alpha = min(1.0, p / 0.25)
            else:
                scale = 1.06 - 0.06 * ((p - 0.55) / 0.45)
                alpha = 1.0
            layer, pos = self.bubble(C_RECORD, phase, ROUND_W, ROUND_H, scale, alpha, dot=True)
        elif t < T_REC_END:
            layer, pos = self.bubble(C_RECORD, phase, ROUND_W, ROUND_H, dot=True)
        elif t < T_MORPH_END:
            e = EASE_SIZE((t - T_REC_END) / MORPH_MS)
            bw = ROUND_W + (PILL_W - ROUND_W) * e
            bh = ROUND_H + (PILL_H - ROUND_H) * e
            layer, pos = self.bubble(C_THINK, phase, bw, bh)
            label = e > 0.65
        elif t < T_THINK_END:
            layer, pos = self.bubble(C_THINK, phase, PILL_W, PILL_H)
            label = True
        elif t < T_OPT_END:
            layer, pos = self.bubble(C_OPT, phase, PILL_W, PILL_H)
            label = True
        elif t < T_FADE:
            layer, pos = self.bubble(C_DONE, phase, ROUND_W, ROUND_H)
        else:
            fade = min(1.0, (t - T_FADE) / FADE_MS)
            layer, pos = self.bubble(C_DONE, phase, ROUND_W, ROUND_H, alpha=1.0 - fade)

        base.paste(layer, pos, layer)
        if label:
            ImageDraw.Draw(base).text(
                (BUBBLE_CX * S, BUBBLE_CY * S), LABEL, font=self.label_font,
                fill=(255, 255, 255), anchor="mm",
            )
        if is_done and t < T_FADE + FADE_MS:
            fade = 0.0 if t < T_FADE else min(1.0, (t - T_FADE) / FADE_MS)
            self.draw_check(base, 1.0 - fade)
        return base


def build_frames():
    r = Renderer()
    frames = []
    t = 0
    while t < TOTAL_MS:
        frames.append((r.frame(t), FRAME_MS))
        t += FRAME_MS
    return frames


def save_gif(frames, out_path):
    # 每帧都是整幅画面，disposal=1 让 GIF 只存相邻帧的差异，体积约为 disposal=2 的一半。
    # 用整段动画采样出一张全局调色板，避免逐帧量化引起颜色跳变
    step = max(1, len(frames) // 24)
    sample = frames[::step]
    mosaic = Image.new("RGB", (W, H * len(sample)))
    for i, (img, _) in enumerate(sample):
        mosaic.paste(img, (0, i * H))
    palette = mosaic.quantize(colors=256, method=Image.MEDIANCUT)

    quantized = [img.quantize(palette=palette, dither=Image.NONE) for img, _ in frames]
    quantized[0].save(
        out_path,
        save_all=True,
        append_images=quantized[1:],
        duration=[ms for _, ms in frames],
        loop=0,
        optimize=True,
        disposal=1,
    )


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_OUT
    frames = build_frames()
    save_gif(frames, out_path)
    print(
        f"{out_path}  {W}×{H}（显示 {W // S}×{H // S}）  帧数 {len(frames)}  "
        f"时长 {TOTAL_MS / 1000:.1f}s  体积 {os.path.getsize(out_path) / 1024:.0f}KB"
    )


if __name__ == "__main__":
    main()
