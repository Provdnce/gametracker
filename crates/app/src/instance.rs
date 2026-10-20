//! Single instance: only one GameTracker may run (otherwise key presses would
//! be counted twice). Starting it a second time just asks the running
//! instance to show its window.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

const PORT: u16 = 47_613;
const MAGIC: &[u8] = b"gametracker:show";

pub enum Instance {
    /// We are the first instance. The receiver gets a message whenever
    /// another start asks us to show the window.
    Primary(Receiver<()>),
    /// Another instance is already running and was told to show itself.
    AlreadyRunning,
}

pub fn acquire() -> Instance {
    match TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)) {
        Ok(listener) => {
            let (tx, rx) = channel();
            std::thread::Builder::new()
                .name("single-instance".into())
                .spawn(move || {
                    for stream in listener.incoming().flatten() {
                        let mut stream = stream;
                        stream.set_read_timeout(Some(Duration::from_secs(1))).ok();
                        let mut buf = [0u8; 32];
                        let n = stream.read(&mut buf).unwrap_or(0);
                        if &buf[..n] == MAGIC && tx.send(()).is_err() {
                            break;
                        }
                    }
                })
                .ok();
            Instance::Primary(rx)
        }
        Err(_) => {
            if let Ok(mut s) = TcpStream::connect_timeout(
                &(Ipv4Addr::LOCALHOST, PORT).into(),
                Duration::from_secs(1),
            ) {
                s.write_all(MAGIC).ok();
                Instance::AlreadyRunning
            } else {
                // Port taken by something else: run anyway rather than not at all.
                let (_tx, rx) = channel();
                std::mem::forget(_tx);
                Instance::Primary(rx)
            }
        }
    }
}
