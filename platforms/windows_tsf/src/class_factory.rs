// VietFlow Windows TSF - COM Class Factory
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::tip::VietFlowTip;
use windows::core::{implement, BOOL, Interface, Result, GUID, Ref};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE};
use windows::Win32::System::Com::IClassFactory_Impl;

#[implement(windows::Win32::System::Com::IClassFactory)]
pub struct VietFlowClassFactory;

impl IClassFactory_Impl for VietFlowClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<'_, windows::core::IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        unsafe {
            if ppvobject.is_null() {
                return Err(E_NOINTERFACE.into());
            }
            *ppvobject = std::ptr::null_mut();

            let tip = VietFlowTip::new();
            let unk: windows::core::IUnknown = tip.into();
            unk.query(riid, ppvobject).ok()
        }
    }

    fn LockServer(&self, _flock: BOOL) -> Result<()> {
        Ok(())
    }
}
