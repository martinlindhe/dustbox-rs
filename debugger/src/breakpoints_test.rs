use breakpoints::Breakpoints;

#[test]
fn sorted_breakpoints() {
    let mut bps = Breakpoints::default();
    bps.add(3);
    bps.add(1);
    bps.add(2);

    assert_eq!(vec![1,2,3], bps.get());
}
