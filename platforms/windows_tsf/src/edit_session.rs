// VietFlow Windows TSF - Edit Session for Direct Document Text Insertion
// Licensed under GNU General Public License v3.0 (GPLv3)

use windows::core::{implement, Interface, Result};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition, ITfEditSession,
    ITfEditSession_Impl, TF_ANCHOR_END, TF_SELECTION, TF_SELECTIONSTYLE,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditAction {
    StartOrUpdate,
    EndComposition,
    ClearAndEndComposition,
}

#[implement(ITfEditSession)]
pub struct CompositionEditSession {
    context: ITfContext,
    sink: ITfCompositionSink,
    composition_holder: *mut Option<ITfComposition>,
    text: Vec<u16>,
    action: EditAction,
}

unsafe impl Send for CompositionEditSession {}
unsafe impl Sync for CompositionEditSession {}

impl CompositionEditSession {
    pub fn new(
        context: ITfContext,
        sink: ITfCompositionSink,
        composition_holder: *mut Option<ITfComposition>,
        text: &str,
        action: EditAction,
    ) -> Self {
        let wide: Vec<u16> = text.encode_utf16().collect();
        Self {
            context,
            sink,
            composition_holder,
            text: wide,
            action,
        }
    }
}

impl ITfEditSession_Impl for CompositionEditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        unsafe {
            if self.action == EditAction::EndComposition {
                if let Some(holder) = self.composition_holder.as_mut() {
                    if let Some(comp) = holder.take() {
                        let _ = comp.EndComposition(ec);
                    }
                }
                return Ok(());
            }

            if self.action == EditAction::ClearAndEndComposition {
                if let Some(holder) = self.composition_holder.as_mut() {
                    if let Some(comp) = holder.take() {
                        if let Ok(range) = comp.GetRange() {
                            let _ = range.SetText(ec, 0, &[]);
                            let _ = range.Collapse(ec, TF_ANCHOR_END);
                            let new_sel = [TF_SELECTION {
                                range: core::mem::ManuallyDrop::new(Some(range)),
                                style: TF_SELECTIONSTYLE::default(),
                            }];
                            let _ = self.context.SetSelection(ec, &new_sel);
                        }
                        let _ = comp.EndComposition(ec);
                    }
                }
                return Ok(());
            }

            if let Some(holder) = self.composition_holder.as_mut() {
                if holder.is_none() {
                    // Start composition on current caret selection
                    let mut selections = [TF_SELECTION::default(); 1];
                    let mut fetched = 0;
                    if self.context.GetSelection(ec, 0, &mut selections, &mut fetched).is_ok() && fetched > 0 {
                        if let Some(range) = selections[0].range.as_ref() {
                            let comp_ctx: Result<ITfContextComposition> = self.context.cast();
                            if let Ok(comp_mgr) = comp_ctx {
                                if let Ok(comp) = comp_mgr.StartComposition(ec, range, &self.sink) {
                                    let _ = range.SetText(ec, 0, &self.text);
                                    let _ = range.Collapse(ec, TF_ANCHOR_END);
                                    let new_sel = [TF_SELECTION {
                                        range: core::mem::ManuallyDrop::new(Some(range.clone())),
                                        style: TF_SELECTIONSTYLE::default(),
                                    }];
                                    let _ = self.context.SetSelection(ec, &new_sel);
                                    *holder = Some(comp);
                                }
                            }
                        }
                    }
                } else if let Some(comp) = holder.as_ref() {
                    // Update existing composition text
                    if let Ok(range) = comp.GetRange() {
                        let _ = range.SetText(ec, 0, &self.text);
                        let _ = range.Collapse(ec, TF_ANCHOR_END);
                        let new_sel = [TF_SELECTION {
                            range: core::mem::ManuallyDrop::new(Some(range)),
                            style: TF_SELECTIONSTYLE::default(),
                        }];
                        let _ = self.context.SetSelection(ec, &new_sel);
                    }
                }
            }
        }
        Ok(())
    }
}
