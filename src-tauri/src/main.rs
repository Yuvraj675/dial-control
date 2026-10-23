// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // On Linux, use X11/XWayland if available so that the HUD and floating status pill
        // can be precisely positioned on screen (e.g. bottom-right corner for scroll indicator).
        // Standard Wayland xdg_toplevel explicitly ignores client-side positioning requests.
        if std::env::var("GDK_BACKEND").is_err() {
            std::env::set_var("GDK_BACKEND", "x11,wayland");
        }
    }

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
        #[cfg(windows)]
        {
            use std::io::Write;
            use std::net::TcpStream;
            if let Ok(mut stream) = TcpStream::connect("127.0.0.1:41829") {
                let _ = stream.write_all(b"toggle\n");
                let _ = stream.flush();
                println!("[CLI] Dispatched toggle signal to running daemon via TCP loopback.");
                return;
            }
        }
    }

    dial_control_lib::run();
}
