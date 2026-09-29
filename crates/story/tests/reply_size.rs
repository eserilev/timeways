use timeways_story::reply_size::Size;

#[test]
fn plain_ascii_takes_one_byte_in_the_slot() {
    assert_eq!(Size::of_json(b"abc"), Size { line: 3, slot: 3 });
}

#[test]
fn a_pipe_takes_two_bytes_because_the_bridge_doubles_it() {
    assert_eq!(Size::of_json(b"|").slot, 2);
}

#[test]
fn a_quote_a_backslash_and_each_byte_past_ascii_take_four() {
    assert_eq!(Size::of_json(b"\"\\").slot, 8);
    assert_eq!(Size::of_json("é".as_bytes()), Size { line: 2, slot: 8 });
}
