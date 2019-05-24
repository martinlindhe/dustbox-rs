use crate::debug::Debugger;
use crate::cpu::R;

#[test]
fn test_parse_hex_string() {
    let mut dbg = Debugger::default();
    dbg.machine.cpu.set_r16(R::CS, 0x085F);
    assert_eq!(0x1234, dbg.parse_register_hex_string("1234").unwrap());
    assert_eq!(0xFFFF, dbg.parse_register_hex_string("FFFF").unwrap());
    assert_eq!(0xFFFF, dbg.parse_register_hex_string("0xFFFF").unwrap());
    assert_eq!(0xFFFF, dbg.parse_register_hex_string("0XFFFF").unwrap());
    assert_eq!(0x085F, dbg.parse_register_hex_string("CS").unwrap());
}

#[test]
fn test_parse_segment_offset_pair() {
    let mut dbg = Debugger::default();
    dbg.machine.cpu.set_r16(R::CS, 0x085F);
    assert_eq!(0x8731, dbg.parse_segment_offset_pair("085F:0141").unwrap());
    assert_eq!(0x8731, dbg.parse_segment_offset_pair("0x085F:0x0141").unwrap());
    assert_eq!(0x8731, dbg.parse_segment_offset_pair("CS:0141").unwrap());
    assert_eq!(0x873F, dbg.parse_segment_offset_pair("873F").unwrap());
}


#[test]
fn test_dis_toml_file() {
    // XXX make use of this

    #[derive(Debug, Deserialize)]
    struct DisNote {
        offset: usize,
        text: String,
        extra: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct DisToml {
        notes: Option<Vec<DisNote>>,
    }

    let toml_str = r#"
        notes = [
            { offset = 0x0185, text = "push cs + 0x20", extra = "085F:0185"},
            { offset = 0x0189, text = "???", extra = "085F:0189 (mov word [ds:0x0201], ax)"},
            { offset = 0x018E, text = "push 0", extra = "085F:018E"},
            { offset = 0x018F, text = "set cs:ip to 087F:0000", extra = "085F:018F"},
        ]
    "#;

    let decoded: DisToml = toml::from_str(toml_str).unwrap();
    println!("{:#?}", decoded);

}
