use memory::mmu::MMU;

#[test]
fn can_convert_to_flat() {
    assert_eq!(0xC0000, MMU::to_flat(0xC000, 0x0000));
}

#[test]
fn can_convert_to_long_pair() {
    let long = MMU::to_long_pair(0xC000, 0x0000);
    assert_eq!(0xC0000000, long);
    assert_eq!(0xC000, MMU::segment_from_long_pair(long));
    assert_eq!(0x0000, MMU::offset_from_long_pair(long));
}
