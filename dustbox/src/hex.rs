pub fn hex_bytes(data: &[u8]) -> String {
    let strs: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();
    strs.join("")
}

pub fn hex_bytes_separated(data: &[u8], sep: char) -> String {
    let strs: Vec<String> = data.iter().map(|b| format!("{:02X}{}", b, sep)).collect();
    strs.join("")
}
