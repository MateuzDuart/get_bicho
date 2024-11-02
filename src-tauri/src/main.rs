// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod modules;

#[tokio::main] // Inicia o runtime tokio
async fn main() {
    get_bicho_lib::run();
}
