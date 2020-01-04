use crate::debug::memory_breakpoints::MemoryBreakpoints;

#[test]
fn sorted_memory_breakpoints() {
    let mut bps = MemoryBreakpoints::default();
    bps.add(3);
    bps.add(1);
    bps.add(2);

    assert_eq!(vec![1,2,3], bps.get());
}

#[test]
fn memory_breakpoints_has_changed() {
    let mut bps = MemoryBreakpoints::default();

    assert_eq!(false, bps.has_changed(0x1234, 1));
    assert_eq!(false, bps.has_changed(0x1234, 1));
    assert_eq!(true, bps.has_changed(0x1234, 2));
    assert_eq!(false, bps.has_changed(0x1234, 2));
}
