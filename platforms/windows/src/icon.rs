// VietFlow Windows - High-DPI In-Memory Tray Icon Generator
// Licensed under GNU General Public License v3.0 (GPLv3)

use windows_sys::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, HICON, ICONINFO};

const RED: u32 = 0xFFD32F2F;   // Vietnamese active badge
const BLUE: u32 = 0xFF1976D2;  // English active badge
const WHITE: u32 = 0xFFFFFFFF;
const TRANS: u32 = 0x00000000;

pub fn create_v_icon() -> HICON {
    let mut pixels = [TRANS; 16 * 16];

    // 16x16 bitmap (top-down)
    for y in 1..15 {
        for x in 1..15 {
            pixels[y * 16 + x] = RED;
        }
    }

    // Letter 'V' in white
    let v_coords = [
        // Row 3
        (3, 3), (4, 3), (11, 3), (12, 3),
        // Row 4
        (3, 4), (4, 4), (11, 4), (12, 4),
        // Row 5
        (4, 5), (5, 5), (10, 5), (11, 5),
        // Row 6
        (4, 6), (5, 6), (10, 6), (11, 6),
        // Row 7
        (5, 7), (6, 7), (9, 7), (10, 7),
        // Row 8
        (5, 8), (6, 8), (9, 8), (10, 8),
        // Row 9
        (6, 9), (7, 9), (8, 9), (9, 9),
        // Row 10
        (6, 10), (7, 10), (8, 10), (9, 10),
        // Row 11
        (7, 11), (8, 11),
        // Row 12
        (7, 12), (8, 12),
    ];

    for &(x, y) in &v_coords {
        pixels[y * 16 + x] = WHITE;
    }

    unsafe { create_icon_from_pixels(&pixels) }
}

pub fn create_e_icon() -> HICON {
    let mut pixels = [TRANS; 16 * 16];

    for y in 1..15 {
        for x in 1..15 {
            pixels[y * 16 + x] = BLUE;
        }
    }

    // Letter 'E' in white
    // Vertical stem: x in 3..6, y in 3..13
    for y in 3..13 {
        for x in 4..7 {
            pixels[y * 16 + x] = WHITE;
        }
    }
    // Top bar: y in 3..5, x in 4..12
    for y in 3..5 {
        for x in 4..12 {
            pixels[y * 16 + x] = WHITE;
        }
    }
    // Middle bar: y in 7..9, x in 4..10
    for y in 7..9 {
        for x in 4..10 {
            pixels[y * 16 + x] = WHITE;
        }
    }
    // Bottom bar: y in 11..13, x in 4..12
    for y in 11..13 {
        for x in 4..12 {
            pixels[y * 16 + x] = WHITE;
        }
    }

    unsafe { create_icon_from_pixels(&pixels) }
}

unsafe fn create_icon_from_pixels(pixels: &[u32; 16 * 16]) -> HICON {
    let mask = [0u8; 16 * 2]; // 16 rows, 2 bytes each (all zeroes = opaque where color is set)
    let hbm_mask = CreateBitmap(16, 16, 1, 1, mask.as_ptr() as *const _);
    let hbm_color = CreateBitmap(16, 16, 1, 32, pixels.as_ptr() as *const _);

    let mut icon_info = ICONINFO {
        fIcon: 1,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: hbm_mask,
        hbmColor: hbm_color,
    };

    let hicon = CreateIconIndirect(&mut icon_info);
    DeleteObject(hbm_mask);
    DeleteObject(hbm_color);
    hicon
}
