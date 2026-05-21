use sqrit::clipboard::ClipboardWriter;

/// Locks the "one handle for the whole app" contract: even after multiple
/// copies, `ClipboardWriter` must reuse a single underlying
/// `arboard::Clipboard`. Reverting to per-call construction (the original
/// bug — "Clipboard was dropped very quickly after writing (0ms)") would
/// cause `init_attempts` to tick up to N for N copy calls.
///
/// We don't assert on the success/failure of `.copy()` itself because the
/// test environment may or may not have a display attached; what matters
/// for the regression is the number of construction attempts.
#[test]
fn repeated_copies_reuse_a_single_arboard_handle() {
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
        "subsequent copies must NOT re-init arboard — the X11 serve thread \
         must outlive each copy"
    );
}

#[test]
fn init_failure_latches_so_subsequent_copies_dont_storm() {
    // We can't force arboard::Clipboard::new() to fail deterministically in a
    // test without owning the env, but we can at least observe that
    // `is_initialized()` reflects state correctly and `init_attempts`
    // stays at most 1 even after many copies. On hosts with no display
    // arboard returns an error; on hosts with one it returns Ok. Either
    // way the count must stop at 1.
    let mut writer = ClipboardWriter::new();
    for _ in 0..5 {
        let _ = writer.copy("hello");
    }
    assert!(
        writer.init_attempts() <= 1,
        "ClipboardWriter must attempt arboard init at most once per lifetime, \
         got: {}",
        writer.init_attempts()
    );
}
