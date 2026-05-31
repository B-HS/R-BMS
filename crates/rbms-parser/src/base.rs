pub fn digit(c: u8, base: u32) -> Option<u32> {
    let v = match c {
        b'0'..=b'9' => (c - b'0') as u32,
        b'A'..=b'Z' => (c - b'A') as u32 + 10,
        b'a'..=b'z' => {
            if base == 62 {
                (c - b'a') as u32 + 36
            } else {
                (c - b'a') as u32 + 10
            }
        }
        _ => return None,
    };
    if v < base { Some(v) } else { None }
}

pub fn parse_pair(c0: u8, c1: u8, base: u32) -> u32 {
    digit(c0, base).unwrap_or(0) * base + digit(c1, base).unwrap_or(0)
}
