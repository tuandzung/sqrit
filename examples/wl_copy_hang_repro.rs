//! Hang repro for the post-merge `yy` hang. Two variants of how the
//! `wl-copy` child is awaited.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

fn variant_wait_with_output(text: &str) -> std::io::Result<std::time::Duration> {
    let start = Instant::now();
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let _ = child.wait_with_output()?;
    Ok(start.elapsed())
}

fn variant_wait(text: &str) -> std::io::Result<std::time::Duration> {
    let start = Instant::now();
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let _ = child.wait()?;
    Ok(start.elapsed())
}

fn timed(label: &str, f: impl FnOnce() -> std::io::Result<std::time::Duration> + Send) {
    let started = Instant::now();
    let res = std::thread::scope(|s| {
        let h = s.spawn(f);
        let mut waited = std::time::Duration::ZERO;
        loop {
            if h.is_finished() {
                return h.join().unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            waited += std::time::Duration::from_millis(100);
            if waited > std::time::Duration::from_secs(3) {
                println!(
                    "[{}] HANG  (still running after 3s, elapsed total {:?})",
                    label,
                    started.elapsed()
                );
                std::process::exit(0);
            }
        }
    });
    match res {
        Ok(d) => println!("[{}] OK   in {:?}", label, d),
        Err(e) => println!("[{}] ERR  {}", label, e),
    }
}

fn main() {
    println!("=== wl-copy await variant hang repro ===");
    timed("wait (post-fix)", || variant_wait("hang-repro-wait"));
    timed("wait_with_output (regression)", || {
        variant_wait_with_output("hang-repro-wait_with_output")
    });
}
