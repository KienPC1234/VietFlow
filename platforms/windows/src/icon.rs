// VietFlow Windows - High-DPI Tray Icon Vector Renderer
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ptr::null_mut;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, GetSystemMetrics, LoadIconW, LoadImageW, HICON, IMAGE_ICON,
    LR_LOADFROMFILE, SM_CXSMICON, SM_CYSMICON,
};

use crate::config::AppConfig;

#[link(name = "gdiplus")]
extern "system" {
    fn GdiplusStartup(
        token: *mut usize,
        input: *const std::ffi::c_void,
        output: *mut std::ffi::c_void,
    ) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipCreateBitmapFromScan0(
        w: i32,
        h: i32,
        stride: i32,
        fmt: i32,
        scan0: *mut u8,
        bitmap: *mut *mut std::ffi::c_void,
    ) -> i32;
    fn GdipGetImageGraphicsContext(
        img: *mut std::ffi::c_void,
        g: *mut *mut std::ffi::c_void,
    ) -> i32;
    fn GdipSetSmoothingMode(g: *mut std::ffi::c_void, mode: i32) -> i32;
    fn GdipSetTextRenderingHint(g: *mut std::ffi::c_void, hint: i32) -> i32;
    fn GdipCreateSolidFill(color: u32, brush: *mut *mut std::ffi::c_void) -> i32;
    fn GdipCreatePen1(color: u32, width: f32, unit: i32, pen: *mut *mut std::ffi::c_void) -> i32;
    fn GdipSetPenLineJoin(pen: *mut std::ffi::c_void, join: i32) -> i32;
    fn GdipCreatePath(fill_mode: i32, path: *mut *mut std::ffi::c_void) -> i32;
    fn GdipAddPathString(
        path: *mut std::ffi::c_void,
        s: *const u16,
        len: i32,
        family: *mut std::ffi::c_void,
        style: i32,
        em: f32,
        rect: *const f32,
        fmt: *mut std::ffi::c_void,
    ) -> i32;
    fn GdipGetPathWorldBounds(
        path: *mut std::ffi::c_void,
        bounds: *mut f32,
        matrix: *mut std::ffi::c_void,
        pen: *mut std::ffi::c_void,
    ) -> i32;
    fn GdipCreateFontFamilyFromName(
        name: *const u16,
        col: *mut std::ffi::c_void,
        fam: *mut *mut std::ffi::c_void,
    ) -> i32;
    fn GdipDeleteFontFamily(fam: *mut std::ffi::c_void) -> i32;
    fn GdipTranslateWorldTransform(
        g: *mut std::ffi::c_void,
        dx: f32,
        dy: f32,
        order: i32,
    ) -> i32;
    fn GdipFillPath(
        g: *mut std::ffi::c_void,
        b: *mut std::ffi::c_void,
        p: *mut std::ffi::c_void,
    ) -> i32;
    fn GdipDrawPath(
        g: *mut std::ffi::c_void,
        pen: *mut std::ffi::c_void,
        p: *mut std::ffi::c_void,
    ) -> i32;
    fn GdipCreateHICONFromBitmap(
        bmp: *mut std::ffi::c_void,
        hicon: *mut *mut std::ffi::c_void,
    ) -> i32;
    fn GdipDeletePath(p: *mut std::ffi::c_void) -> i32;
    fn GdipDeletePen(p: *mut std::ffi::c_void) -> i32;
    fn GdipDeleteBrush(b: *mut std::ffi::c_void) -> i32;
    fn GdipDeleteGraphics(g: *mut std::ffi::c_void) -> i32;
    fn GdipDisposeImage(img: *mut std::ffi::c_void) -> i32;
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

    unsafe { LoadIconW(null_mut(), 32512 as *const u16) }
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

    unsafe { LoadIconW(null_mut(), 32512 as *const u16) }
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
#[repr(C)]
struct GdiplusStartupInput {
    version: u32,
    debug_callback: usize,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

fn render_glyph_icon(
    letter: char,
    fill_enabled: bool,
    fill_hex: &str,
    border_enabled: bool,
    border_hex: &str,
) -> Option<HICON> {
    unsafe {
        let size_x = GetSystemMetrics(SM_CXSMICON);
        let size_y = GetSystemMetrics(SM_CYSMICON);
        let size = size_x.max(size_y).max(16);

        let mut token: usize = 0;
        let startup_input = GdiplusStartupInput {
            version: 1,
            debug_callback: 0,
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        let status = GdiplusStartup(&mut token, &startup_input as *const _ as _, null_mut());
        if status != 0 {
            return None;
        }

        let mut bitmap: *mut std::ffi::c_void = null_mut();
        // PixelFormat32bppARGB = 0x26200A
        let status = GdipCreateBitmapFromScan0(size, size, 0, 0x26200A, null_mut(), &mut bitmap);
        if status != 0 {
            GdiplusShutdown(token);
            return None;
        }

        let mut g: *mut std::ffi::c_void = null_mut();
        GdipGetImageGraphicsContext(bitmap, &mut g);
        GdipSetSmoothingMode(g, 4); // AntiAlias
        GdipSetTextRenderingHint(g, 4);

        let font_name: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
        let mut family: *mut std::ffi::c_void = null_mut();
        let s_fam = GdipCreateFontFamilyFromName(font_name.as_ptr(), null_mut(), &mut family);
        if s_fam != 0 || family.is_null() {
            GdipDisposeImage(bitmap);
            GdiplusShutdown(token);
            return None;
        }

        let mut path: *mut std::ffi::c_void = null_mut();
        GdipCreatePath(0, &mut path);

        let text: Vec<u16> = vec![letter as u16, 0];
        let em_size = size as f32 * 1.30;
        let layout = [0.0f32, 0.0, size as f32 * 1.5, size as f32 * 1.5];
        // Style 1 = Bold
        GdipAddPathString(path, text.as_ptr(), 1, family, 1, em_size, layout.as_ptr(), null_mut());

        let mut bounds = [0.0f32; 4];
        GdipGetPathWorldBounds(path, bounds.as_mut_ptr(), null_mut(), null_mut());

        // Center the glyph precisely inside the icon bounds
        if bounds[2] > 0.0 && bounds[3] > 0.0 {
            let dx = ((size as f32 - bounds[2]) / 2.0) - bounds[0];
            let dy = ((size as f32 - bounds[3]) / 2.0) - bounds[1];
            GdipTranslateWorldTransform(g, dx, dy, 0);
        }

        let do_fill = fill_enabled || (!fill_enabled && !border_enabled);
        if do_fill {
            let fill_argb = parse_hex_argb(fill_hex, 0xFFFFFFFF);
            let mut fill_brush: *mut std::ffi::c_void = null_mut();
            if GdipCreateSolidFill(fill_argb, &mut fill_brush) == 0 {
                GdipFillPath(g, fill_brush, path);
                GdipDeleteBrush(fill_brush);
            }
        }

        if border_enabled {
            let default_border = if letter == 'V' { 0xFFFF007F } else { 0xFF0078D7 };
            let border_argb = parse_hex_argb(border_hex, default_border);
            let pen_width = if size >= 24 { 2.4 } else { 1.8 };
            let mut pen: *mut std::ffi::c_void = null_mut();
            if GdipCreatePen1(border_argb, pen_width, 2, &mut pen) == 0 {
                GdipSetPenLineJoin(pen, 2); // Round join
                GdipDrawPath(g, pen, path);
                GdipDeletePen(pen);
            }
        }

        let mut hicon: *mut std::ffi::c_void = null_mut();
        let ok = GdipCreateHICONFromBitmap(bitmap, &mut hicon);

        GdipDeletePath(path);
        GdipDeleteFontFamily(family);
        GdipDeleteGraphics(g);
        GdipDisposeImage(bitmap);
        GdiplusShutdown(token);

        if ok == 0 && !hicon.is_null() {
            Some(hicon as HICON)
        } else {
            None
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

    let ext = p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase());
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

        let hicon = CreateIconFromResourceEx(
            bytes.as_ptr(),
            bytes.len() as u32,
            1,
            0x00030000,
            cx,
            cy,
            0,
        );

        if !hicon.is_null() {
            hicon
        } else {
            CreateIconFromResourceEx(
                bytes.as_ptr(),
                bytes.len() as u32,
                1,
                0x00030000,
                0,
                0,
                0,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_vector_icons() {
        let config = AppConfig::default();
        let icon_v = create_v_icon(&config);
        assert!(!icon_v.is_null(), "V icon must not be null");

        let icon_e = create_e_icon(&config);
        assert!(!icon_e.is_null(), "E icon must not be null");

        let glyph_v = render_glyph_icon('V', true, "#FFFFFF", true, "#FF007F");
        assert!(glyph_v.is_some(), "Vector glyph V rendering must succeed");

        let glyph_e = render_glyph_icon('E', true, "#FFFFFF", true, "#0078D7");
        assert!(glyph_e.is_some(), "Vector glyph E rendering must succeed");
    }
}
