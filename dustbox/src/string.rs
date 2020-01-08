use std::num::ParseIntError;

#[cfg(test)]
#[path = "./string_test.rs"]
mod string_test;

pub fn right_pad(s: &str, len: usize) -> String {
    let mut res = String::new();
    res.push_str(s);
    if s.len() < len {
        let padding_len = len - s.len();
        for _ in 0..padding_len {
            res.push_str(" ");
        }
    }
    res
}

/// parses string to a integer. unprefixed values assume base 10, and "0x" prefix indicates base 16.
pub fn parse_number_string(s: &str) -> Result<u32, ParseIntError> {
    let x = &s.replace("_", "");
    if x.len() >= 2 && &x[0..2] == "0x" {
        // hex
        u32::from_str_radix(&x[2..], 16)
    } else {
        // decimal
        x.parse::<u32>()
    }
}

pub fn bytes_to_ascii(data: &[u8]) -> String {
    data.iter().map(|b| if *b < 128 && *b > 30 {
        *b as char
    } else {
        '.'
    }).collect()
}
