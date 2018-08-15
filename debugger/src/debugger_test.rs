use debugger;
use debugger::parse_number_string;
use dustbox::cpu::R;

#[test]
fn test_parse_number_string() {
    assert_eq!(1234, parse_number_string("1234").unwrap());
    assert_eq!(0xFFFF, parse_number_string("0xFFFF").unwrap());
}

#[test]
fn test_parse_hex_string() {
    let mut dbg = debugger::Debugger::default();
    dbg.machine.cpu.set_r16(R::CS, 0x085F);
    assert_eq!(0x1234, dbg.parse_register_hex_string("1234").unwrap());
    assert_eq!(0xFFFF, dbg.parse_register_hex_string("FFFF").unwrap());
    assert_eq!(0xFFFF, dbg.parse_register_hex_string("0xFFFF").unwrap());
    assert_eq!(0xFFFF, dbg.parse_register_hex_string("0XFFFF").unwrap());
    assert_eq!(0x085F, dbg.parse_register_hex_string("CS").unwrap());
}

#[test]
fn test_parse_segment_offset_pair() {
    let mut dbg = debugger::Debugger::default();
    dbg.machine.cpu.set_r16(R::CS, 0x085F);
    assert_eq!(0x008731, dbg.parse_segment_offset_pair("085F:0141").unwrap());
    assert_eq!(0x008731, dbg.parse_segment_offset_pair("0x085F:0x0141").unwrap());
    assert_eq!(0x008731, dbg.parse_segment_offset_pair("CS:0141").unwrap());
    assert_eq!(0x00873F, dbg.parse_segment_offset_pair("873F").unwrap());
}
