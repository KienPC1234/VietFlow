// VietFlow Windows - High-DPI Tray Icon Glyph Renderer
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::Once;
use windows_sys::Win32::Graphics::Gdi::{
    AddFontMemResourceEx, CreateBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontW,
    DeleteDC, DeleteObject, GetGlyphOutlineW, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, FF_DONTCARE, FIXED,
    FW_BOLD, GGO_BITMAP, GLYPHMETRICS, MAT2, OUT_TT_PRECIS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CopyIcon, CreateIconFromResourceEx, CreateIconIndirect, GetSystemMetrics, LoadIconW,
    LoadImageW, HICON, ICONINFO, IMAGE_ICON, LR_LOADFROMFILE, SM_CXSMICON, SM_CYSMICON,
};

use crate::config::AppConfig;

// Only the two glyphs used by the tray icon. Keeping the outlines (rather
// than a scaled bitmap) makes the icon sharp at every DPI.
const ICON_FONT: &[u8] = include_bytes!("../../../ui_imgui/assets/JetBrainsMonoTrayVE.ttf");
const ICON_FACE: &str = "JetBrainsMono NF";

static FONT_ONCE: Once = Once::new();
static FONT_RESOURCE: AtomicIsize = AtomicIsize::new(0);

fn ensure_icon_font() -> bool {
    FONT_ONCE.call_once(|| {
        let mut count = 0u32;
        let handle = unsafe {
            AddFontMemResourceEx(
                ICON_FONT.as_ptr() as *const c_void,
                ICON_FONT.len() as u32,
                null_mut(),
                &mut count,
            )
        };
        FONT_RESOURCE.store(handle as isize, Ordering::SeqCst);
    });
    FONT_RESOURCE.load(Ordering::SeqCst) != 0
}

pub fn create_v_icon(config: &AppConfig) -> HICON {
    if !config.custom_icon_v.is_empty() {
        if let Some(icon) = load_icon_from_file(&config.custom_icon_v) {
            return icon;
        }
    }

    if let Some(hicon) = render_glyph_icon(
        'V',
        config.tray_icon_v_fill,
        &config.tray_icon_v_fill_color,
        config.tray_icon_v_border,
        &config.tray_icon_v_border_color,
    ) {
        return hicon;
    }

    // Copy the shared system fallback so TrayIcon may always DestroyIcon.
    unsafe { CopyIcon(LoadIconW(null_mut(), 32512 as *const u16)) }
}

pub fn create_e_icon(config: &AppConfig) -> HICON {
    if !config.custom_icon_e.is_empty() {
        if let Some(icon) = load_icon_from_file(&config.custom_icon_e) {
            return icon;
        }
    }

    if let Some(hicon) = render_glyph_icon(
        'E',
        config.tray_icon_e_fill,
        &config.tray_icon_e_fill_color,
        config.tray_icon_e_border,
        &config.tray_icon_e_border_color,
    ) {
        return hicon;
    }

    // Copy the shared system fallback so TrayIcon may always DestroyIcon.
    unsafe { CopyIcon(LoadIconW(null_mut(), 32512 as *const u16)) }
}

/// Load the main VietFlow application icon (from embedded resource or disk assets).
/// Used in TSF IME mode where the tray icon indicates the app itself rather than V/E state.
pub fn create_app_icon() -> HICON {
    let size_x = unsafe { GetSystemMetrics(SM_CXSMICON) };
    let size_y = unsafe { GetSystemMetrics(SM_CYSMICON) };
    let cx = if size_x > 0 { size_x } else { 16 };
    let cy = if size_y > 0 { size_y } else { 16 };

    // 1. Try loading embedded resource icon (ID 1) from the executable module
    unsafe {
        let hinstance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(null_mut());
        let hicon = LoadImageW(hinstance, 1 as *const u16, IMAGE_ICON, cx, cy, 0);
        if !hicon.is_null() {
            return hicon as HICON;
        }
    }

    // 2. Fall back to loading from disk assets if running in unbundled dev environment
    let candidate_paths = [
        "assets/VietFlow.ico",
        "../../assets/VietFlow.ico",
        "../assets/VietFlow.ico",
    ];
    for p in &candidate_paths {
        if std::path::Path::new(p).is_file() {
            if let Some(icon) = load_icon_from_file(p) {
                return icon;
            }
        }
    }

    // 3. Fall back to system application icon
    unsafe { CopyIcon(LoadIconW(null_mut(), 32512 as *const u16)) }
}

fn parse_hex_argb(hex: &str, default_argb: u32) -> u32 {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        if let Ok(rgb) = u32::from_str_radix(clean, 16) {
            return 0xFF000000 | rgb;
        }
    } else if clean.len() == 8 {
        if let Ok(argb) = u32::from_str_radix(clean, 16) {
            return argb;
        }
    }
    default_argb
}

/// Renders a large, crisp, perfectly centered vector letter glyph directly at
/// the exact system tray icon dimensions (SM_CXSMICON x SM_CYSMICON) with zero
/// scaling blur and transparent background.
fn color_components(argb: u32) -> (u8, u8, u8, u8) {
    (
        ((argb >> 16) & 0xFF) as u8,
        ((argb >> 8) & 0xFF) as u8,
        (argb & 0xFF) as u8,
        ((argb >> 24) & 0xFF) as u8,
    )
}

fn downsample(mask: &[u8], render_size: usize, size: usize, scale: usize) -> Vec<u8> {
    let mut result = vec![0u8; size * size];
    let samples = (scale * scale) as u32;
    for y in 0..size {
        for x in 0..size {
            let mut sum = 0u32;
            for sy in 0..scale {
                let row = (y * scale + sy) * render_size;
                for sx in 0..scale {
                    sum += mask[row + x * scale + sx] as u32;
                }
            }
            result[y * size + x] = (sum * 255 / samples) as u8;
        }
    }
    result
}

fn render_glyph_icon(
    letter: char,
    fill_enabled: bool,
    fill_hex: &str,
    border_enabled: bool,
    border_hex: &str,
) -> Option<HICON> {
    if !ensure_icon_font() {
        return None;
    }

    unsafe {
        let size_x = GetSystemMetrics(SM_CXSMICON);
        let size_y = GetSystemMetrics(SM_CYSMICON);
        let size = size_x.max(size_y).max(16) as usize;
        let scale = 8usize;
        let render_size = size * scale;

        let face: Vec<u16> = ICON_FACE.encode_utf16().chain(std::iter::once(0)).collect();
        let font = CreateFontW(
            -(render_size as i32),
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_TT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            windows_sys::Win32::Graphics::Gdi::ANTIALIASED_QUALITY as u32,
            (DEFAULT_PITCH | FF_DONTCARE) as u32,
            face.as_ptr(),
        );
        if font.is_null() {
            return None;
        }

        let dc = CreateCompatibleDC(null_mut());
        if dc.is_null() {
            DeleteObject(font);
            return None;
        }
        let old_font = SelectObject(dc, font);

        let identity = FIXED { fract: 0, value: 1 };
        let matrix = MAT2 {
            eM11: identity,
            eM12: FIXED { fract: 0, value: 0 },
            eM21: FIXED { fract: 0, value: 0 },
            eM22: identity,
        };

        let mut metrics: GLYPHMETRICS = std::mem::zeroed();
        let needed = GetGlyphOutlineW(
            dc,
            letter as u32,
            GGO_BITMAP,
            &mut metrics,
            0,
            null_mut(),
            &matrix,
        );
        if needed == u32::MAX || needed == 0 || metrics.gmBlackBoxX == 0 || metrics.gmBlackBoxY == 0
        {
            if !old_font.is_null() {
                SelectObject(dc, old_font);
            }
            DeleteObject(font);
            DeleteDC(dc);
            return None;
        }

        let stride = (metrics.gmBlackBoxX.div_ceil(32) * 4) as usize;
        let mut glyph = vec![0u8; needed as usize];
        let got = GetGlyphOutlineW(
            dc,
            letter as u32,
            GGO_BITMAP,
            &mut metrics,
            needed,
            glyph.as_mut_ptr() as *mut c_void,
            &matrix,
        );

        if !old_font.is_null() {
            SelectObject(dc, old_font);
        }
        DeleteObject(font);
        DeleteDC(dc);

        if got == u32::MAX {
            return None;
        }

        let glyph_w = metrics.gmBlackBoxX as usize;
        let glyph_h = metrics.gmBlackBoxY as usize;
        let origin_x = (render_size - glyph_w) / 2;
        let origin_y = (render_size - glyph_h) / 2;
        let mut fill = vec![0u8; render_size * render_size];
        for y in 0..glyph_h {
            for x in 0..glyph_w {
                let bit = glyph[y * stride + x / 8] & (0x80 >> (x % 8)) != 0;
                if bit {
                    let canvas_x = origin_x + x;
                    let canvas_y = origin_y + y;
                    if canvas_x < render_size && canvas_y < render_size {
                        fill[canvas_y * render_size + canvas_x] = 1;
                    }
                }
            }
        }

        let mut border = vec![0u8; render_size * render_size];
        if border_enabled {
            let pen_width = if size >= 24 { 2.4f32 } else { 1.8f32 };
            let radius = (pen_width * scale as f32).round().max(1.0) as i32;
            let mut offsets = Vec::new();
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy <= radius * radius {
                        offsets.push((dx, dy));
                    }
                }
            }
            for y in 0..render_size {
                for x in 0..render_size {
                    if fill[y * render_size + x] == 0 {
                        continue;
                    }
                    for (dx, dy) in &offsets {
                        let bx = x as i32 + dx;
                        let by = y as i32 + dy;
                        if bx >= 0
                            && by >= 0
                            && (bx as usize) < render_size
                            && (by as usize) < render_size
                        {
                            border[by as usize * render_size + bx as usize] = 1;
                        }
                    }
                }
            }
        }

        let fill_alpha = downsample(&fill, render_size, size, scale);
        let border_alpha = downsample(&border, render_size, size, scale);
        let do_fill = fill_enabled || (!fill_enabled && !border_enabled);
        let (fr, fg, fb, fa) = color_components(parse_hex_argb(fill_hex, 0xFFFFFFFF));
        let default_border = if letter == 'V' {
            0xFFFF007F
        } else {
            0xFF0078D7
        };
        let (br, bg, bb, ba) = color_components(parse_hex_argb(border_hex, default_border));

        let mut pixels = vec![0u8; size * size * 4];
        for i in 0..size * size {
            let mut fill_a = if do_fill { fill_alpha[i] as u32 } else { 0 };
            fill_a = fill_a * fa as u32 / 255;
            let mut border_a = if border_enabled {
                border_alpha[i] as u32
            } else {
                0
            };
            border_a = border_a * ba as u32 / 255;

            let out_a = fill_a + border_a * (255 - fill_a) / 255;
            if out_a == 0 {
                continue;
            }
            let out_r = (fr as u32 * fill_a + br as u32 * border_a * (255 - fill_a) / 255) / out_a;
            let out_g = (fg as u32 * fill_a + bg as u32 * border_a * (255 - fill_a) / 255) / out_a;
            let out_b = (fb as u32 * fill_a + bb as u32 * border_a * (255 - fill_a) / 255) / out_a;
            pixels[i * 4] = out_b as u8;
            pixels[i * 4 + 1] = out_g as u8;
            pixels[i * 4 + 2] = out_r as u8;
            pixels[i * 4 + 3] = out_a as u8;
        }

        let mut bitmap_info: BITMAPINFO = std::mem::zeroed();
        bitmap_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bitmap_info.bmiHeader.biWidth = size as i32;
        bitmap_info.bmiHeader.biHeight = -(size as i32);
        bitmap_info.bmiHeader.biPlanes = 1;
        bitmap_info.bmiHeader.biBitCount = 32;
        bitmap_info.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut c_void = null_mut();
        let color = CreateDIBSection(
            null_mut(),
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bits,
            null_mut(),
            0,
        );
        if color.is_null() || bits.is_null() {
            if !color.is_null() {
                DeleteObject(color);
            }
            return None;
        }
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());

        let mask_stride = (size + 15) / 16 * 2;
        let mut mask = vec![0u8; mask_stride * size];
        for y in 0..size {
            for x in 0..size {
                if pixels[(y * size + x) * 4 + 3] == 0 {
                    mask[y * mask_stride + x / 8] |= 0x80 >> (x % 8);
                }
            }
        }
        let mask_bitmap = CreateBitmap(
            size as i32,
            size as i32,
            1,
            1,
            mask.as_ptr() as *const c_void,
        );
        if mask_bitmap.is_null() {
            DeleteObject(color);
            return None;
        }

        let icon_info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bitmap,
            hbmColor: color,
        };
        let hicon = CreateIconIndirect(&icon_info);
        DeleteObject(mask_bitmap);
        DeleteObject(color);

        if hicon.is_null() {
            None
        } else {
            Some(hicon)
        }
    }
}

fn load_icon_from_file(path_str: &str) -> Option<HICON> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return None;
    }
    let p = std::path::Path::new(trimmed);
    if !p.is_file() {
        return None;
    }

    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    if ext.as_deref() == Some("ico") {
        let wide: Vec<u16> = trimmed.encode_utf16().chain(std::iter::once(0)).collect();
        let size_x = unsafe { GetSystemMetrics(SM_CXSMICON) };
        let size_y = unsafe { GetSystemMetrics(SM_CYSMICON) };
        let cx = if size_x > 0 { size_x } else { 16 };
        let cy = if size_y > 0 { size_y } else { 16 };

        let hicon = unsafe {
            LoadImageW(
                null_mut(),
                wide.as_ptr(),
                IMAGE_ICON,
                cx,
                cy,
                LR_LOADFROMFILE,
            )
        };
        if !hicon.is_null() {
            return Some(hicon as HICON);
        }
    } else {
        if let Ok(bytes) = std::fs::read(p) {
            let hicon = create_icon_from_png(&bytes);
            if !hicon.is_null() {
                return Some(hicon);
            }
        }
    }
    None
}

pub fn create_icon_from_png(bytes: &[u8]) -> HICON {
    unsafe {
        let size_x = GetSystemMetrics(SM_CXSMICON);
        let size_y = GetSystemMetrics(SM_CYSMICON);
        let cx = if size_x > 0 { size_x } else { 16 };
        let cy = if size_y > 0 { size_y } else { 16 };

        let hicon =
            CreateIconFromResourceEx(bytes.as_ptr(), bytes.len() as u32, 1, 0x00030000, cx, cy, 0);

        if !hicon.is_null() {
            hicon
        } else {
            CreateIconFromResourceEx(bytes.as_ptr(), bytes.len() as u32, 1, 0x00030000, 0, 0, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_glyph_icons() {
        let config = AppConfig::default();
        let icon_v = create_v_icon(&config);
        assert!(!icon_v.is_null(), "V icon must not be null");

        let icon_e = create_e_icon(&config);
        assert!(!icon_e.is_null(), "E icon must not be null");

        let glyph_v = render_glyph_icon('V', true, "#FFFFFF", true, "#FF007F");
        assert!(glyph_v.is_some(), "Glyph V rendering must succeed");

        let glyph_e = render_glyph_icon('E', true, "#FFFFFF", true, "#0078D7");
        assert!(glyph_e.is_some(), "Glyph E rendering must succeed");
    }
}
