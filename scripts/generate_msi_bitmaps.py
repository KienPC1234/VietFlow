#!/usr/bin/env python3
"""
Generate standard WiX MSI installer bitmaps:
1. wix_dialog.bmp (493x312) - Welcome & Exit dialog background
2. wix_banner.bmp (493x58)  - Top strip for intermediate dialogs
"""

import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
ICO_PATH = ASSETS / "VietFlow.ico"
FONT_DIR = Path("C:/Windows/Fonts")

FONT_SEGOE_BOLD = str(FONT_DIR / "segoeuib.ttf")
FONT_SEGOE_REG = str(FONT_DIR / "segoeui.ttf")
FONT_SEGOE_SEMIBOLD = str(FONT_DIR / "seguisb.ttf")


def hex_to_rgb(hex_code: str):
    h = hex_code.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def interpolate_color(c1, c2, factor):
    return tuple(int(a + (b - a) * factor) for a, b in zip(c1, c2))


def draw_vertical_gradient(draw, x0, y0, x1, y1, top_hex, bottom_hex):
    c_top = hex_to_rgb(top_hex)
    c_bot = hex_to_rgb(bottom_hex)
    height = max(1, y1 - y0)
    for y in range(y0, y1):
        factor = (y - y0) / height
        col = interpolate_color(c_top, c_bot, factor)
        draw.line([(x0, y), (x1, y)], fill=col)


def generate_dialog_bmp(out_path: Path):
    width, height = 493, 312
    sidebar_w = 164

    # Create image in RGBA for layered drawing
    img = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    draw = ImageDraw.Draw(img)

    # 1. Left sidebar gradient (#0e1e38 -> #1e3a6f -> #2552a8)
    c1 = hex_to_rgb("#0c192c")
    c2 = hex_to_rgb("#183b70")
    c3 = hex_to_rgb("#2354a8")
    for y in range(height):
        if y < height * 0.55:
            factor = y / (height * 0.55)
            col = interpolate_color(c1, c2, factor)
        else:
            factor = (y - height * 0.55) / (height * 0.45)
            col = interpolate_color(c2, c3, factor)
        draw.line([(0, y), (sidebar_w - 1, y)], fill=col + (255,))

    # 2. Subtle radial glow behind the logo
    glow_center_x, glow_center_y = 82, 108
    glow_radius = 65
    glow_layer = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow_layer)
    for r in range(glow_radius, 0, -2):
        alpha = int(45 * (1.0 - (r / glow_radius) ** 1.5))
        glow_col = (70, 140, 245, alpha)
        glow_draw.ellipse(
            [
                (glow_center_x - r, glow_center_y - r),
                (glow_center_x + r, glow_center_y + r),
            ],
            fill=glow_col,
        )
    img = Image.alpha_composite(img, glow_layer)
    draw = ImageDraw.Draw(img)

    # 3. Paste VietFlow logo (80x80) with soft drop shadow
    raw_ico = Image.open(ICO_PATH).convert("RGBA")
    logo_size = 78
    logo = raw_ico.resize((logo_size, logo_size), Image.Resampling.LANCZOS)

    # Shadow
    shadow_layer = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    shadow_x = 82 - logo_size // 2
    shadow_y = 68
    # Alpha mask of logo for shadow
    logo_alpha = logo.split()[3]
    shadow_mask = Image.new("RGBA", (logo_size, logo_size), (0, 0, 0, 120))
    shadow_mask.putalpha(logo_alpha)
    shadow_layer.paste(shadow_mask, (shadow_x, shadow_y + 4))
    shadow_layer = shadow_layer.filter(ImageFilter.GaussianBlur(4))
    img = Image.alpha_composite(img, shadow_layer)
    draw = ImageDraw.Draw(img)

    # Paste crisp logo
    img.paste(logo, (shadow_x, shadow_y), logo)

    # 4. Typography
    try:
        font_brand = ImageFont.truetype(FONT_SEGOE_BOLD, 22)
        font_sub = ImageFont.truetype(FONT_SEGOE_REG, 11)
        font_footer = ImageFont.truetype(FONT_SEGOE_REG, 9)
    except Exception:
        font_brand = ImageFont.load_default()
        font_sub = font_brand
        font_footer = font_brand

    # "VietFlow" brand text
    brand_text = "VietFlow"
    bbox = draw.textbbox((0, 0), brand_text, font=font_brand)
    tw = bbox[2] - bbox[0]
    draw.text((82 - tw // 2, 160), brand_text, fill=(255, 255, 255, 255), font=font_brand)

    # Tagline
    line1 = "Modern Vietnamese"
    line2 = "Input Method"
    bbox1 = draw.textbbox((0, 0), line1, font=font_sub)
    bbox2 = draw.textbbox((0, 0), line2, font=font_sub)
    draw.text((82 - (bbox1[2] - bbox1[0]) // 2, 192), line1, fill=(180, 210, 255, 240), font=font_sub)
    draw.text((82 - (bbox2[2] - bbox2[0]) // 2, 208), line2, fill=(180, 210, 255, 240), font=font_sub)

    # Footer
    footer_text = "VietFlow Team"
    fbbox = draw.textbbox((0, 0), footer_text, font=font_footer)
    draw.text((82 - (fbbox[2] - fbbox[0]) // 2, 288), footer_text, fill=(130, 165, 215, 200), font=font_footer)

    # 5. Vertical divider between sidebar and content pane
    draw.line([(sidebar_w, 0), (sidebar_w, height)], fill=(205, 215, 225, 255), width=1)

    # Save 24-bit RGB BMP (standard required by Windows Installer)
    final_rgb = img.convert("RGB")
    final_rgb.save(out_path, format="BMP")
    print(f"Generated Dialog BMP: {out_path} ({final_rgb.size})")


def generate_banner_bmp(out_path: Path):
    width, height = 493, 58

    img = Image.new("RGBA", (width, height), (255, 255, 255, 255))
    draw = ImageDraw.Draw(img)

    # Bottom border line
    draw.line([(0, height - 1), (width, height - 1)], fill=(215, 222, 230, 255), width=1)

    # Right side VietFlow icon (44x44)
    raw_ico = Image.open(ICO_PATH).convert("RGBA")
    logo_size = 44
    logo = raw_ico.resize((logo_size, logo_size), Image.Resampling.LANCZOS)

    icon_x = width - logo_size - 12
    icon_y = (height - 1 - logo_size) // 2

    # Subtle drop shadow
    shadow_layer = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    logo_alpha = logo.split()[3]
    shadow_mask = Image.new("RGBA", (logo_size, logo_size), (0, 0, 0, 60))
    shadow_mask.putalpha(logo_alpha)
    shadow_layer.paste(shadow_mask, (icon_x, icon_y + 2))
    shadow_layer = shadow_layer.filter(ImageFilter.GaussianBlur(2))
    img = Image.alpha_composite(img, shadow_layer)

    img.paste(logo, (icon_x, icon_y), logo)

    final_rgb = img.convert("RGB")
    final_rgb.save(out_path, format="BMP")
    print(f"Generated Banner BMP: {out_path} ({final_rgb.size})")


def main():
    ASSETS.mkdir(parents=True, exist_ok=True)
    dialog_bmp = ASSETS / "wix_dialog.bmp"
    banner_bmp = ASSETS / "wix_banner.bmp"

    generate_dialog_bmp(dialog_bmp)
    generate_banner_bmp(banner_bmp)


if __name__ == "__main__":
    main()

