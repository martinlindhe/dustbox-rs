use crate::string::parse_number_string;

#[test]
fn test_parse_number_string() {
    assert_eq!(1234, parse_number_string("1234").unwrap());
    assert_eq!(0xFFFF, parse_number_string("0xFFFF").unwrap());
}
