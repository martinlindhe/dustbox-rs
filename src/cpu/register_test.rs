use cpu::register::{R, RegisterSnapshot};

#[test]
fn can_access_gpr() {
    let mut r = RegisterSnapshot::default();
    r.set_r32(&R::ECX, 0xFFFF_FFFF);
    assert_eq!(0xFFFF_FFFF, r.get_r32(&R::ECX));

    r.set_r16(&R::CX, 0x1616);
    assert_eq!(0x1616, r.get_r16(&R::CX));
    assert_eq!(0xFFFF_1616, r.get_r32(&R::ECX));

    r.set_r8(&R::CL, 0x08);
    assert_eq!(0x08, r.get_r8(&R::CL));
    assert_eq!(0xFFFF_1608, r.get_r32(&R::ECX));
}
