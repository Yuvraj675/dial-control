// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

/// Sends `cmd` to an already-running instance and returns true only if it answered "ok",
/// so an unrelated program that happens to hold the Windows loopback port is never mistaken
/// for Dial Control.
fn send_to_running_instance(cmd: &str) -> bool {
    fn exchange<S: std::io::Read + Write>(mut stream: S, cmd: &str) -> bool {
        if stream.write_all(format!("{cmd}\n").as_bytes()).is_err() || stream.flush().is_err() {
            return false;
        }
        let mut reply = String::new();
        BufReader::new(&mut stream).read_line(&mut reply).is_ok() && reply.trim() == "ok"
    }

    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        if let Ok(stream) = UnixStream::connect("/tmp/dial-control.sock") {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));
            return exchange(stream, cmd);
        }
    }
    #[cfg(windows)]
    {
        use std::net::{SocketAddr, TcpStream};
        let addr: SocketAddr = "127.0.0.1:41829".parse().expect("valid loopback address");
        if let Ok(stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));
            return exchange(stream, cmd);
        }
    }
    false
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--toggle" || a == "-t" || a == "toggle") {
        if send_to_running_instance("toggle") {
            println!("[CLI] Dispatched toggle signal to running daemon.");
            return;
        }
    } else if send_to_running_instance("show") {
        // Single instance: a second launch (application menu, autostart + manual start)
        // just brings the existing instance's settings window forward.
        println!("[CLI] Dial Control is already running; showing its settings window.");
        return;
    }

    dial_control_lib::run();
}
