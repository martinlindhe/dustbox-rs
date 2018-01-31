use cpu::Flags;

#[test]
fn can_pack_unpack_flags() {
    let mut flags = Flags::new();
    flags.set_u16(0xFFFF);
    assert_eq!(0x7FD7, flags.u16());
}
