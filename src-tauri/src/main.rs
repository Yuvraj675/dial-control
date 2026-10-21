// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--toggle" || a == "-t" || a == "toggle") {
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::net::UnixStream;
            let socket_path = "/tmp/dial-control.sock";
            if let Ok(mut stream) = UnixStream::connect(socket_path) {
                let _ = stream.write_all(b"toggle\n");
                let _ = stream.flush();
                println!("[CLI] Dispatched toggle signal to running daemon via UNIX socket.");
                return;
            }
        }
    }

    dial_control_lib::run();
}
