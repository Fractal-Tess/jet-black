#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    net::{SocketAddr, TcpStream},
    thread,
    time::{Duration, Instant},
};

const LOCAL_ADDRESS: &str = "127.0.0.1:4317";

fn main() {
    thread::spawn(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("desktop backend runtime");
        if let Err(error) = runtime.block_on(jet_black::run_desktop()) {
            eprintln!("Jet Black desktop backend stopped: {error}");
        }
    });
    wait_for_backend(
        LOCAL_ADDRESS
            .parse()
            .expect("static desktop backend address"),
        Duration::from_secs(15),
    );
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("run Jet Black desktop shell");
}

fn wait_for_backend(address: SocketAddr, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    eprintln!("Jet Black desktop backend did not become ready within {timeout:?}");
}
