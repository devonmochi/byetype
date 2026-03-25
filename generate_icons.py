#!/usr/bin/env python3
"""Generate ByeType app icons with F3 design (symmetric sound wave bars)."""
import math
import struct
import zlib
from PIL import Image, ImageDraw

ICON_DIR = "src-tauri/icons"

# F3 design parameters (viewBox 0 0 92 94)
# 5 bars: symmetric pattern (short-tall-tallest-tall-short)
BARS = [
    {"x": 1,    "y": 30, "w": 16, "h": 34},
    {"x": 19.5, "y": 12, "w": 16, "h": 70},
    {"x": 38,   "y": 2,  "w": 16, "h": 90},
    {"x": 56.5, "y": 12, "w": 16, "h": 70},
    {"x": 75,   "y": 30, "w": 16, "h": 34},
]
VB_W, VB_H = 92, 94  # viewBox dimensions


def lerp_color(c1, c2, t):
    """Linear interpolation between two RGB colors."""
    return tuple(int(c1[i] + (c2[i] - c1[i]) * t) for i in range(3))


def create_gradient_image(size):
    """Create an image with 135-degree gradient from #FF8C42 to #FF5E1A."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    c1 = (0xFF, 0x8C, 0x42)  # top-left
    c2 = (0xFF, 0x5E, 0x1A)  # bottom-right
    for y in range(size):
        for x in range(size):
            # 135 degree: top-left to bottom-right
            t = ((x + y) / (2 * size - 2)) if size > 1 else 0
            r, g, b = lerp_color(c1, c2, t)
            img.putpixel((x, y), (r, g, b, 255))
    return img


def draw_rounded_rect(draw, bbox, radius, fill):
    """Draw a rounded rectangle."""
    x0, y0, x1, y1 = [int(v) for v in bbox]
    r = int(radius)
    r = min(r, (x1 - x0) // 2, (y1 - y0) // 2)
    if r < 1:
        draw.rectangle(bbox, fill=fill)
        return
    # Use Pillow's built-in rounded_rectangle
    draw.rounded_rectangle([x0, y0, x1, y1], radius=r, fill=fill)


def create_app_icon(size):
    """Create app icon at given size with orange gradient bg and white bars."""
    # Create gradient background
    img = create_gradient_image(size)

    # Apply rounded corner mask
    corner_radius = size * 0.22  # macOS-style rounded corners
    mask = Image.new("L", (size, size), 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.rounded_rectangle([0, 0, size - 1, size - 1], radius=int(corner_radius), fill=255)
    img.putalpha(mask)

    draw = ImageDraw.Draw(img)

    # Calculate padding: bars should occupy ~82% of icon area, centered
    padding = size * 0.09
    available = size - 2 * padding
    scale_x = available / VB_W
    scale_y = available / VB_H
    offset_x = padding + (available - VB_W * scale_x) / 2
    offset_y = padding + (available - VB_H * scale_y) / 2

    # Draw white bars
    for bar in BARS:
        bx = offset_x + bar["x"] * scale_x
        by = offset_y + bar["y"] * scale_y
        bw = bar["w"] * scale_x
        bh = bar["h"] * scale_y
        br = min(bw, bh) / 2  # full round caps
        draw_rounded_rect(draw, [bx, by, bx + bw, by + bh], br, fill=(255, 255, 255, 255))

    return img


def create_tray_icon(size, color=(0, 0, 0, 255)):
    """Create tray icon (bars only, on transparent background)."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Bars fill most of the icon with small padding
    padding = size * 0.08
    available = size - 2 * padding
    scale_x = available / VB_W
    scale_y = available / VB_H
    offset_x = padding + (available - VB_W * scale_x) / 2
    offset_y = padding + (available - VB_H * scale_y) / 2

    for bar in BARS:
        bx = offset_x + bar["x"] * scale_x
        by = offset_y + bar["y"] * scale_y
        bw = bar["w"] * scale_x
        bh = bar["h"] * scale_y
        br = min(bw, bh) / 2
        draw_rounded_rect(draw, [bx, by, bx + bw, by + bh], br, fill=color)

    return img


def create_icns(icon_dir):
    """Create .icns file using iconutil."""
    import subprocess, tempfile, os, shutil

    iconset_dir = tempfile.mkdtemp(suffix=".iconset")

    # Required sizes for iconset
    sizes = [
        (16, "icon_16x16.png"),
        (32, "icon_16x16@2x.png"),
        (32, "icon_32x32.png"),
        (64, "icon_32x32@2x.png"),
        (128, "icon_128x128.png"),
        (256, "icon_128x128@2x.png"),
        (256, "icon_256x256.png"),
        (512, "icon_256x256@2x.png"),
        (512, "icon_512x512.png"),
        (1024, "icon_512x512@2x.png"),
    ]

    for size, name in sizes:
        icon = create_app_icon(size)
        icon.save(os.path.join(iconset_dir, name), "PNG")

    output = os.path.join(icon_dir, "icon.icns")
    subprocess.run(["iconutil", "-c", "icns", iconset_dir, "-o", output], check=True)
    shutil.rmtree(iconset_dir)
    print(f"  Created {output}")


def create_ico(icon_dir):
    """Create .ico file with multiple sizes."""
    import os
    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = []
    for s in sizes:
        images.append(create_app_icon(s))

    output = os.path.join(icon_dir, "icon.ico")
    images[0].save(output, format="ICO", sizes=[(s, s) for s in sizes], append_images=images[1:])
    print(f"  Created {output}")


def main():
    import os
    os.makedirs(ICON_DIR, exist_ok=True)

    print("Generating app icons...")
    # Standard app icons
    for size, name in [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png")]:
        icon = create_app_icon(size)
        path = os.path.join(ICON_DIR, name)
        icon.save(path, "PNG")
        print(f"  Created {path} ({size}x{size})")

    # Large icon.png (512x512)
    icon = create_app_icon(512)
    path = os.path.join(ICON_DIR, "icon.png")
    icon.save(path, "PNG")
    print(f"  Created {path} (512x512)")

    print("\nGenerating tray icons...")
    # Tray default: black bars on transparent (macOS template image)
    tray_color = (0, 0, 0, 255)
    for size, name in [(16, "tray-default.png"), (32, "tray-default@2x.png")]:
        icon = create_tray_icon(size, tray_color)
        path = os.path.join(ICON_DIR, name)
        icon.save(path, "PNG")
        print(f"  Created {path} ({size}x{size})")

    # Tray recording: red bars on transparent
    rec_color = (255, 59, 48, 255)  # system red
    for size, name in [(16, "tray-recording.png"), (32, "tray-recording@2x.png")]:
        icon = create_tray_icon(size, rec_color)
        path = os.path.join(ICON_DIR, name)
        icon.save(path, "PNG")
        print(f"  Created {path} ({size}x{size})")

    print("\nGenerating icon.icns...")
    create_icns(ICON_DIR)

    print("\nGenerating icon.ico...")
    create_ico(ICON_DIR)

    print("\nAll icons generated successfully!")


if __name__ == "__main__":
    main()
