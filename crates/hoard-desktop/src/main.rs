// Prevent the console window from popping up alongside the GUI on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    hoard_desktop_lib::run();
}
