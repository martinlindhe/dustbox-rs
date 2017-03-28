pub fn lookup_opcode<'a>(op: u8) -> &'a str {
    match op {
        _ => "XXX Unknown",
    }
}
