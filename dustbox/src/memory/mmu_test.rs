use crate::memory::mmu::MemoryAddress;

#[test]
fn can_handle_real_mode_addressing() {
    let ma = MemoryAddress::RealSegmentOffset(0xC000, 0x0000);
    assert_eq!(0xC_0000, ma.value());
    assert_eq!(0xC000, ma.segment());
    assert_eq!(0x0000, ma.offset());

    assert_eq!(0x86FF, MemoryAddress::RealSegmentOffset(0x085F, 0x10F).value());
    assert_eq!(0x8700, MemoryAddress::RealSegmentOffset(0x085F, 0x110).value());
}

#[test]
fn can_convert_to_long_pair() {
    let ma = MemoryAddress::LongSegmentOffset(0xC000, 0x0000);
    assert_eq!(0xC000_0000, ma.value());
    assert_eq!(0xC000, ma.segment());
    assert_eq!(0x0000, ma.offset());
}

#[test]
fn resolve_real_addressing() {
    let ma1 = MemoryAddress::RealSegmentOffset(0x0000, 0x046C);
    let ma2 = MemoryAddress::RealSegmentOffset(0x0040, 0x006C);
    assert_eq!(ma1.value(), ma2.value());
}
