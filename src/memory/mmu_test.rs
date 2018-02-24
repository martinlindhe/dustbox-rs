use memory::mmu::MemoryAddress;

#[test]
fn can_handle_real_mode_addressing() {
    let ma = MemoryAddress::RealSegmentOffset(0xC000, 0x0000);
    assert_eq!(0xC0000, ma.value());
    assert_eq!(0xC000, ma.segment());
    assert_eq!(0x0000, ma.offset());

    assert_eq!(0x0086FF, MemoryAddress::RealSegmentOffset(0x085F, 0x10F).value());
    assert_eq!(0x008700, MemoryAddress::RealSegmentOffset(0x085F, 0x110).value());
}

#[test]
fn can_convert_to_long_pair() {
    let ma = MemoryAddress::LongSegmentOffset(0xC000, 0x0000);
    assert_eq!(0xC0000000, ma.value());
    assert_eq!(0xC000, ma.segment());
    assert_eq!(0x0000, ma.offset());
}
