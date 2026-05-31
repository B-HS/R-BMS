#[derive(Debug, Clone, Copy)]
enum Frame {
    Random { value: u32 },
    If { active: bool, matched: bool },
}

/// `#RANDOM`/`#IF` control-flow resolver. A single chart is materialised by drawing
/// a value per `#RANDOM n` (deterministic from the seed) and emitting only the lines
/// inside matching `#IF` branches.
pub struct Control {
    stack: Vec<Frame>,
    rng: u64,
}

impl Control {
    pub fn new(seed: u64) -> Self {
        Control { stack: Vec::new(), rng: seed.wrapping_add(0x9E37_79B9_7F4A_7C15) }
    }

    fn next_rng(&mut self) -> u64 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.rng >> 33
    }

    fn rand_range(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 1;
        }
        (self.next_rng() as u32 % n) + 1
    }

    fn current_random(&self) -> u32 {
        for f in self.stack.iter().rev() {
            if let Frame::Random { value } = f {
                return *value;
            }
        }
        0
    }

    fn last_if_pos(&self) -> Option<usize> {
        self.stack.iter().rposition(|f| matches!(f, Frame::If { .. }))
    }

    fn active_excluding(&self, pos: usize) -> bool {
        self.stack.iter().enumerate().all(|(i, f)| match f {
            Frame::If { active, .. } => i == pos || *active,
            _ => true,
        })
    }

    pub fn active(&self) -> bool {
        self.stack.iter().all(|f| match f {
            Frame::If { active, .. } => *active,
            _ => true,
        })
    }

    pub fn handle(&mut self, body: &str) -> bool {
        let (head, rest) = match body.find(char::is_whitespace) {
            Some(i) => (&body[..i], body[i..].trim()),
            None => (body, ""),
        };
        match head.to_ascii_uppercase().as_str() {
            "RANDOM" | "RONDAM" => {
                let n = rest.split_whitespace().next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                let v = self.rand_range(n);
                self.stack.push(Frame::Random { value: v });
                true
            }
            "SETRANDOM" => {
                let v = rest.split_whitespace().next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                self.stack.push(Frame::Random { value: v });
                true
            }
            "ENDRANDOM" => {
                while let Some(f) = self.stack.pop() {
                    if matches!(f, Frame::Random { .. }) {
                        break;
                    }
                }
                true
            }
            "IF" => {
                let v = rest.parse::<u32>().unwrap_or(0);
                let active = self.active() && self.current_random() == v;
                self.stack.push(Frame::If { active, matched: active });
                true
            }
            "ELSEIF" => {
                let v = rest.parse::<u32>().unwrap_or(0);
                if let Some(pos) = self.last_if_pos() {
                    let Frame::If { matched, .. } = self.stack[pos] else { unreachable!() };
                    let active = self.active_excluding(pos) && !matched && self.current_random() == v;
                    self.stack[pos] = Frame::If { active, matched: matched || active };
                }
                true
            }
            "ELSE" => {
                if let Some(pos) = self.last_if_pos() {
                    let Frame::If { matched, .. } = self.stack[pos] else { unreachable!() };
                    let active = self.active_excluding(pos) && !matched;
                    self.stack[pos] = Frame::If { active, matched: true };
                }
                true
            }
            "ENDIF" => {
                if let Some(pos) = self.last_if_pos() {
                    self.stack.truncate(pos);
                }
                true
            }
            _ => false,
        }
    }
}
