use debugger::{parse_number_string, parse_segment_offset_pair};

#[test]
fn test_parse_number_string() {
    assert_eq!(1234, parse_number_string("1234").unwrap());
    assert_eq!(0xFFFF, parse_number_string("0xFFFF").unwrap());
}

#[test]
fn test_parse_segment_offset_pair() {
    assert_eq!(0x008731, parse_segment_offset_pair("085F:0141").unwrap());
}
