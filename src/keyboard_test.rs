use keyboard::StatusRegister;

#[test]
fn test_status_register() {
    let sr = StatusRegister::default();
    assert_eq!(0b001_0100, sr.as_u8()); // system 1, unknown4 1
}
