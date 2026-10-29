// VietFlow Unified Configuration GUI Entrypoint
// Licensed under GNU General Public License v3.0 (GPLv3)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().collect();
    let convert_mode = args.iter().any(|a| a == "--convert" || a == "-c");
    vietflow_gui::run_gui(convert_mode)
}
