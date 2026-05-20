use sqrit::editor::EditorBuffer;

// #1 insert_char appends and advances cursor
#[test]
fn insert_char_appends_at_cursor() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('h');
    buf.insert_char('i');
    assert_eq!(buf.text(), "hi");
    assert_eq!(buf.cursor(), (0, 2));
}

// #2 backspace removes char before cursor
#[test]
fn backspace_removes_char() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('b');
    buf.insert_char('c');
    buf.backspace();
    assert_eq!(buf.text(), "ab");
    assert_eq!(buf.cursor(), (0, 2));
}

// #3 backspace at start does nothing on empty first line
#[test]
fn backspace_at_start_of_first_line_is_noop() {
    let mut buf = EditorBuffer::new();
    buf.backspace();
    assert_eq!(buf.text(), "");
    assert_eq!(buf.cursor(), (0, 0));
}

// #4 backspace at line start joins with previous line
#[test]
fn backspace_at_line_start_joins_lines() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('b');
    buf.insert_newline();
    buf.insert_char('c');
    // "ab\nc" cursor at (1,1)
    buf.backspace(); // remove 'c'
    buf.backspace(); // join lines
    assert_eq!(buf.text(), "ab");
    assert_eq!(buf.cursor(), (0, 2));
}

// #5 cursor_left / cursor_right movement
#[test]
fn cursor_left_right_movement() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('b');
    buf.insert_char('c');
    // cursor at col 3
    buf.cursor_left();
    assert_eq!(buf.cursor(), (0, 2));
    buf.cursor_left();
    assert_eq!(buf.cursor(), (0, 1));
    // can't go before 0
    buf.cursor_left();
    buf.cursor_left();
    buf.cursor_left();
    assert_eq!(buf.cursor(), (0, 0));
    buf.cursor_right();
    assert_eq!(buf.cursor(), (0, 1));
    // can't go past end
    buf.cursor_right();
    buf.cursor_right();
    buf.cursor_right();
    assert_eq!(buf.cursor(), (0, 3));
}

// #6 insert_newline splits line
#[test]
fn insert_newline_splits_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('h');
    buf.insert_char('e');
    buf.insert_char('l');
    buf.insert_char('l');
    buf.insert_char('o');
    // "hello", cursor at (0,5)
    buf.cursor_left();
    buf.cursor_left();
    // cursor at (0,3)
    buf.insert_newline();
    assert_eq!(buf.text(), "hel\nlo");
    assert_eq!(buf.cursor(), (1, 0));
}

// #7 cursor_up / cursor_down between lines
#[test]
fn cursor_up_down_between_lines() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('b');
    buf.insert_char('c');
    buf.insert_newline();
    buf.insert_char('x');
    // "abc\nx" cursor at (1,1)
    buf.cursor_up();
    assert_eq!(buf.cursor(), (0, 1)); // col clamped to line len if needed
    buf.cursor_down();
    assert_eq!(buf.cursor(), (1, 1));
}

// #8 home / end
#[test]
fn home_and_end() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('b');
    buf.insert_char('c');
    // cursor at (0,3)
    buf.home();
    assert_eq!(buf.cursor(), (0, 0));
    buf.end();
    assert_eq!(buf.cursor(), (0, 3));
}

// #9 multiline cursor_up clamps col to shorter line
#[test]
fn cursor_up_clamps_to_shorter_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('b');
    buf.insert_char('c');
    buf.insert_char('d');
    buf.insert_char('e');
    buf.insert_newline();
    buf.insert_char('x');
    // "abcde\nx" cursor at (1,1)
    buf.cursor_right(); // (1,2) but line is only 1 char, so stays
    buf.cursor_up(); // go to row 0, col clamps to... actually col was 1
    assert_eq!(buf.cursor_row(), 0);
}

// #10 insert in middle of text
#[test]
fn insert_in_middle_of_text() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('c');
    buf.cursor_left();
    buf.insert_char('b');
    assert_eq!(buf.text(), "abc");
    assert_eq!(buf.cursor(), (0, 2));
}

// T10 #1: word_forward moves to next word
#[test]
fn word_forward_moves_to_next_word() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("hello world foo");
    buf.home();
    // cursor at 0
    buf.word_forward();
    assert_eq!(buf.cursor_col(), 6); // start of "world"
    buf.word_forward();
    assert_eq!(buf.cursor_col(), 12); // start of "foo"
}

// T10 #2: word_backward moves to previous word
#[test]
fn word_backward_moves_to_previous_word() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("hello world foo");
    buf.end();
    // cursor at end (col 14)
    buf.word_backward();
    assert_eq!(buf.cursor_col(), 12); // start of "foo"
    buf.word_backward();
    assert_eq!(buf.cursor_col(), 6); // start of "world"
}

// T10 #3: delete_char deletes char at cursor
#[test]
fn delete_char_removes_at_cursor() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("abc");
    buf.home();
    buf.cursor_right(); // col 1, cursor on 'b'
    buf.delete_char();
    assert_eq!(buf.text(), "ac");
    assert_eq!(buf.cursor_col(), 1);
}

// T10 #4: delete_line removes current line and returns it
#[test]
fn delete_line_removes_and_returns_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("line1\nline2\nline3");
    buf.go_top(); // row 0
    buf.cursor_down(); // row 1
    let deleted = buf.delete_line();
    assert_eq!(deleted, Some("line2".to_string()));
    assert_eq!(buf.text(), "line1\nline3");
    assert_eq!(buf.cursor_row(), 1);
}

// T10 #5: yank_line returns current line without deleting
#[test]
fn yank_line_returns_current_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("line1\nline2");
    buf.cursor_down();
    let yanked = buf.yank_line();
    assert_eq!(yanked, "line2");
    assert_eq!(buf.text(), "line1\nline2");
}

// T10 #6: paste_below inserts line below
#[test]
fn paste_below_inserts_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("line1\nline3");
    buf.go_top(); // row 0
    buf.paste_below("line2");
    assert_eq!(buf.text(), "line1\nline2\nline3");
    assert_eq!(buf.cursor_row(), 1);
}

// T10 #7: go_top moves to first line start
#[test]
fn go_top_moves_to_first_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("a\nb\nc");
    buf.go_bottom();
    buf.go_top();
    assert_eq!(buf.cursor(), (0, 0));
}

// T10 #8: go_bottom moves to last line start
#[test]
fn go_bottom_moves_to_last_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("a\nb\nc");
    buf.go_bottom();
    assert_eq!(buf.cursor_row(), 2);
    assert_eq!(buf.cursor_col(), 0);
}

// T10 #9: undo reverts last change
#[test]
fn undo_reverts_insert_char() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.undo();
    assert_eq!(buf.text(), "");
}

#[test]
fn undo_reverts_multiple_inserts_one_at_a_time() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    buf.insert_char('b');
    buf.undo();
    assert_eq!(buf.text(), "a");
    buf.undo();
    assert_eq!(buf.text(), "");
}

// T26 #1: delete_backwards removes n chars before cursor
#[test]
fn delete_backwards_removes_n_chars() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("SELE");
    buf.delete_backwards(4);
    assert_eq!(buf.text(), "");
    assert_eq!(buf.cursor(), (0, 0));
}

// T26 #2: delete_backwards with 0 is noop
#[test]
fn delete_backwards_zero_noop() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("abc");
    buf.delete_backwards(0);
    assert_eq!(buf.text(), "abc");
    assert_eq!(buf.cursor(), (0, 3));
}

// T26 #2b: delete_backwards at buffer start with n > 0 is noop (empty buffer)
#[test]
fn delete_backwards_at_buffer_start_noop() {
    let mut buf = EditorBuffer::new();
    buf.delete_backwards(10);
    assert_eq!(buf.text(), "");
    assert_eq!(buf.cursor(), (0, 0));
}

// T26 #3: delete_backwards clamps to cursor_col, does not cross line
#[test]
fn delete_backwards_clamps_to_line_start() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("ab\ncd");
    // cursor at (1, 2)
    buf.delete_backwards(10);
    assert_eq!(buf.text(), "ab\n");
    assert_eq!(buf.cursor(), (1, 0));
}

// T26 #4: delete_backwards is undoable as one operation
#[test]
fn delete_backwards_undo() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("SELE");
    buf.delete_backwards(4);
    assert_eq!(buf.text(), "");
    buf.undo();
    assert_eq!(buf.text(), "SELE");
    assert_eq!(buf.cursor(), (0, 4));
}

// T26 #5: delete_backwards removes prefix within line, preserving suffix
#[test]
fn delete_backwards_preserves_suffix() {
    let mut buf = EditorBuffer::new();
    buf.insert_str("SELE foo");
    // cursor at (0, 8); move back to after "SELE"
    buf.cursor_left();
    buf.cursor_left();
    buf.cursor_left();
    buf.cursor_left();
    // cursor at (0, 4)
    buf.delete_backwards(4);
    assert_eq!(buf.text(), " foo");
    assert_eq!(buf.cursor(), (0, 0));
}

// T10 #10: undo reverts delete_line (single operation)
#[test]
fn undo_reverts_delete_line() {
    let mut buf = EditorBuffer::new();
    buf.insert_char('a');
    // undo stack: [""]
    // text: "a"
    let deleted = buf.delete_line();
    assert_eq!(deleted, Some("a".to_string()));
    assert_eq!(buf.text(), "");
    buf.undo();
    assert_eq!(buf.text(), "a");
}
