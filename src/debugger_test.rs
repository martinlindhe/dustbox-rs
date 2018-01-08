#[test]
fn test_parse_number_string() {
    use debugger::parse_number_string;
    assert_eq!(1234, parse_number_string("1234").unwrap());
    assert_eq!(0xFFFF, parse_number_string("0xFFFF").unwrap());
}

#[test]
fn test_parse_hex_string() {
    use debugger::parse_hex_string;
    assert_eq!(0x1234, parse_hex_string("1234").unwrap());
    assert_eq!(0xFFFF, parse_hex_string("FFFF").unwrap());
    assert_eq!(0xFFFF, parse_hex_string("0xFFFF").unwrap());
}

#[test]
fn test_parse_segment_offset_pair() {
    use debugger::parse_segment_offset_pair;
    assert_eq!(0x008731, parse_segment_offset_pair("085F:0141").unwrap());
    assert_eq!(0x008731, parse_segment_offset_pair("0x085F:0x0141").unwrap());
    assert_eq!(0x00873F, parse_segment_offset_pair("873F").unwrap());
}
