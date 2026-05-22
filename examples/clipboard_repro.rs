//! Clipboard repro harness for #46 follow-up.
//!
//! Runs four variants of "copy X to clipboard" against arboard, verifies
//! what actually landed via `wl-paste` (Wayland) and `xclip -o` (X11),
//! and prints PASS/FAIL per variant. Standalone — does not depend on the
//! rest of sqrit.
//!
//! Run on the affected Wayland host:
//!   cargo run --example clipboard_repro
//!
//! Paste the output back so we can pick the right production fix.

use arboard::{Clipboard, SetExtLinux};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

fn env_dump() {
    for k in [
        "XDG_SESSION_TYPE",
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "XDG_CURRENT_DESKTOP",
        "TMUX",
        "SSH_CONNECTION",
    ] {
        let v = std::env::var(k).unwrap_or_else(|_| "(unset)".to_string());
        println!("  {}={}", k, v);
    }
}

fn paste_via(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn read_back_both() -> (Option<String>, Option<String>) {
    let wl = if std::env::var("WAYLAND_DISPLAY").is_ok() {
        paste_via("wl-paste", &["--no-newline"])
    } else {
        None
    };
    let xc = paste_via("xclip", &["-selection", "clipboard", "-o"]);
    (wl, xc)
}

fn fmt_seen(label: &str, expected: &str, got: &Option<String>) -> String {
    match got {
        Some(s) if s == expected => format!("{}=PASS", label),
        Some(s) => {
            let snippet: String = s.chars().take(40).collect();
            format!("{}=stale({:?})", label, snippet)
        }
        None => format!("{}=empty", label),
    }
}

fn check(sentinel: &str, variant: &str) {
    // Give compositor / clipboard manager a moment to sample.
    thread::sleep(Duration::from_millis(500));
    let (wl, xc) = read_back_both();
    let wl_summary = fmt_seen("wl-paste", sentinel, &wl);
    let xc_summary = fmt_seen("xclip", sentinel, &xc);
    let any_pass = wl.as_deref() == Some(sentinel) || xc.as_deref() == Some(sentinel);
    let tag = if any_pass { "PASS" } else { "FAIL" };
    println!("[{}] {}  {}  {}", variant, tag, wl_summary, xc_summary);
}

fn variant_a_drop_immediately(sentinel: &str) {
    let mut clip = Clipboard::new().expect("Clipboard::new A");
    clip.set_text(sentinel.to_string()).expect("set_text A");
    drop(clip);
    check(sentinel, "A drop-immediately");
}

fn variant_b_long_lived(sentinel: &str) -> Clipboard {
    let mut clip = Clipboard::new().expect("Clipboard::new B");
    clip.set_text(sentinel.to_string()).expect("set_text B");
    check(sentinel, "B long-lived");
    // Return the live handle so the caller can drop it at end of program.
    clip
}

fn variant_c_set_wait_threaded(sentinel: &str) {
    // arboard's documented Linux-persistence path: `.set().wait().text(s)`
    // blocks until the selection is overwritten. Wrap in a thread so the
    // main thread can move on; the thread exits naturally when variant D
    // takes ownership of the clipboard.
    let s = sentinel.to_string();
    let (ready_tx, ready_rx) = mpsc::channel::<()>();
    let _handle = thread::spawn(move || {
        let mut clip = Clipboard::new().expect("Clipboard::new C");
        let _ = ready_tx.send(()); // signal: handle constructed
        let _ = clip.set().wait().text(s);
        // Falls through when another process overwrites the clipboard.
    });
    // Wait briefly for the thread to actually set the clipboard.
    let _ = ready_rx.recv_timeout(Duration::from_secs(2));
    thread::sleep(Duration::from_millis(200));
    check(sentinel, "C set-wait-thread");
}

fn variant_c_wait_until_threaded(sentinel: &str) {
    // Same shape as C but bounded via `wait_until` so the thread cannot
    // outlive the program. If wait_until works on the user's compositor
    // this is the cleanest non-blocking persistence option for sqrit.
    let s = sentinel.to_string();
    let deadline = Instant::now() + Duration::from_secs(5);
    let (ready_tx, ready_rx) = mpsc::channel::<()>();
    let _handle = thread::spawn(move || {
        let mut clip = Clipboard::new().expect("Clipboard::new C2");
        let _ = ready_tx.send(());
        let _ = clip.set().wait_until(deadline).text(s);
    });
    let _ = ready_rx.recv_timeout(Duration::from_secs(2));
    thread::sleep(Duration::from_millis(200));
    check(sentinel, "C2 wait-until-thread");
}

fn variant_d_wl_copy(sentinel: &str) {
    use std::io::Write;
    let mut child = match Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            println!("[D wl-copy] SKIP  wl-copy not available: {}", e);
            return;
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(sentinel.as_bytes());
    }
    let _ = child.wait();
    check(sentinel, "D wl-copy");
}

fn main() {
    println!("=== sqrit clipboard repro (PR #46 Wayland follow-up) ===");
    println!("env:");
    env_dump();
    println!("\nrunning variants. Each writes a unique sentinel then reads it back.");
    println!("PASS  = read-back matched the sentinel.");
    println!("FAIL  = clipboard was empty or held something else.\n");

    variant_a_drop_immediately("sqrit-clip-A-7f3e");
    let _hold_b = variant_b_long_lived("sqrit-clip-B-7f3e");
    variant_c_set_wait_threaded("sqrit-clip-C-7f3e");
    variant_c_wait_until_threaded("sqrit-clip-C2-7f3e");
    variant_d_wl_copy("sqrit-clip-D-7f3e");

    println!("\nDone. Paste this entire output back.");
    // _hold_b drops here — last sentinel still in clipboard from variant D.
    drop(_hold_b);
}
