// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // V3-05: the same binary doubles as the detached proxy helper, so there is
    // no second artifact to build, sign, ship or trust. Nothing graphical starts
    // in that mode; the helper only listens on loopback and sleeps.
    if std::env::args().any(|a| a == claude_config_manager_lib::helper::HELPER_ARG) {
        std::process::exit(claude_config_manager_lib::helper::run_helper());
    }
    claude_config_manager_lib::run()
}
