use sqrit::clipboard::ClipboardWriter;

// Running these tests on a host with WAYLAND_DISPLAY set would invoke
// `wl-copy` and pollute the user's actual clipboard with the sentinel
// strings. Skip the body in that case — the regression they encode
// (init counter stays at most 1) is still exercised on headless CI and
// X11 hosts. CI runs headless so it always exercises the latch path.
fn skip_if_wayland_dev() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        eprintln!("skipping: WAYLAND_DISPLAY set; would clobber user's clipboard");
        return true;
    }
    false
}

/// Locks the "one backend per writer" contract: even after multiple
/// copies, `ClipboardWriter` must reuse a single backend (one
/// `arboard::Clipboard`, or one decision to shell out to `wl-copy`).
/// Reverting to per-call construction (the original X11 bug —
/// "Clipboard was dropped very quickly after writing (0ms)") would cause
/// `init_attempts` to tick up to N for N copy calls.
#[test]
fn repeated_copies_reuse_a_single_backend() {
    if skip_if_wayland_dev() {
        return;
    }
    let mut writer = ClipboardWriter::new();
    assert_eq!(writer.init_attempts(), 0, "no init until first copy");

    let _ = writer.copy("first");
    let attempts_after_first = writer.init_attempts();
    assert_eq!(
        attempts_after_first, 1,
        "first copy must trigger exactly one init"
    );

    let _ = writer.copy("second");
    let _ = writer.copy("third");

    assert_eq!(
        writer.init_attempts(),
        attempts_after_first,
        "subsequent copies must NOT re-init the backend — the X11 serve \
         thread / Wayland decision must outlive each copy"
    );
}

#[test]
fn init_failure_latches_so_subsequent_copies_dont_storm() {
    if skip_if_wayland_dev() {
        return;
    }
    // We can't force the backend probe to fail deterministically in a
    // test without owning the env, but we can at least observe that
    // `init_attempts` stays at most 1 even after many copies. On hosts
    // with no display arboard returns an error and the writer latches
    // to `Failed`; on hosts with one it latches to `Arboard`. Either way
    // the count must stop at 1.
    let mut writer = ClipboardWriter::new();
    for _ in 0..5 {
        let _ = writer.copy("hello");
    }
    assert!(
        writer.init_attempts() <= 1,
        "ClipboardWriter must probe at most once per lifetime, got: {}",
        writer.init_attempts()
    );
}
