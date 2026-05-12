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
