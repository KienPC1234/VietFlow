// VietFlow Windows TSF - Cross-Process Shared Configuration
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS, FILE_MAP_READ,
    PAGE_READWRITE,
};

const SHARED_MEM_NAME: &str = "Local\\VietFlow_TSF_Config\0";

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TsfSharedConfig {
    pub enabled: u32,
    pub method: u32,
    pub charset: u32,
    pub modern_style: u32,
    pub spell_check: u32,
    pub free_marking: u32,
    pub auto_restore: u32,
    pub consonant_shortcuts: u32,
    pub version: u32,
    pub last_active_tick: u64,
}

impl Default for TsfSharedConfig {
    fn default() -> Self {
        Self {
            enabled: 1,
            method: 0, // Telex
            charset: 0, // Unicode
            modern_style: 1,
            spell_check: 1,
            free_marking: 1,
            auto_restore: 1,
            consonant_shortcuts: 0,
            version: 1,
            last_active_tick: 0,
        }
    }
}

pub struct SharedConfigReader {
    mapping: HANDLE,
    view: *const TsfSharedConfig,
}

impl SharedConfigReader {
    pub fn new() -> Self {
        let wide: Vec<u16> = SHARED_MEM_NAME.encode_utf16().collect();
        let size = std::mem::size_of::<TsfSharedConfig>() as u32;

        unsafe {
            let mapping = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                size,
                PCWSTR(wide.as_ptr()),
            )
            .unwrap_or(HANDLE::default());

            let view = if !mapping.is_invalid() {
                let p = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS | FILE_MAP_READ, 0, 0, size as usize);
                if !p.Value.is_null() {
                    let ptr = p.Value as *mut TsfSharedConfig;
                    if (*ptr).version == 0 {
                        *ptr = TsfSharedConfig::default();
                    }
                    ptr as *const TsfSharedConfig
                } else {
                    ptr::null()
                }
            } else {
                ptr::null()
            };

            Self { mapping, view }
        }
    }

    pub fn get(&self) -> TsfSharedConfig {
        if !self.view.is_null() {
            unsafe { *self.view }
        } else {
            TsfSharedConfig::default()
        }
    }
}

impl Drop for SharedConfigReader {
    fn drop(&mut self) {
        unsafe {
            if !self.view.is_null() {
                let _ = UnmapViewOfFile(windows::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.view as *mut _,
                });
            }
            if !self.mapping.is_invalid() {
                let _ = CloseHandle(self.mapping);
            }
        }
    }
}
