#[derive(Debug, Clone, Copy)]
enum Frame {
    Random { value: u32 },
    If { active: bool, matched: bool },
    Switch { value: u32, active: bool, matched: bool, skipped: bool },
}

/// Whether a frame lets the lines nested inside it through. `#RANDOM` is a value
/// container only, so it never gates on its own.
fn frame_gate(frame: &Frame) -> bool {
    match frame {
        Frame::Random { .. } => true,
        Frame::If { active, .. } | Frame::Switch { active, .. } => *active,
    }
}

/// `#RANDOM`/`#IF` and `#SWITCH`/`#CASE` control-flow resolver. A single chart is
/// materialised by drawing a value per `#RANDOM n`/`#SWITCH n` (deterministic from the
/// seed) and emitting only the lines inside matching `#IF` branches and `#CASE` blocks.
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

    fn last_switch_pos(&self) -> Option<usize> {
        self.stack.iter().rposition(|f| matches!(f, Frame::Switch { .. }))
    }

    fn active_excluding(&self, pos: usize) -> bool {
        self.stack.iter().enumerate().all(|(i, f)| i == pos || frame_gate(f))
    }

    pub fn active(&self) -> bool {
        self.stack.iter().all(frame_gate)
    }

    /// Applies `#CASE k` (`wanted = Some(k)`) or `#DEF` (`wanted = None`) to the innermost
    /// `#SWITCH` frame. A block that is already active falls through unchanged, and a block
    /// closed by `#SKIP` stays inactive until `#ENDSW`. An orphan label is ignored.
    ///
    /// Labels are resolved in file order, one line at a time. This differs from C `switch`
    /// on one point: C picks the jump target when the block is entered, so a `default:`
    /// placed before a matching `case` never runs, while here a `#DEF` that appears before
    /// the matching `#CASE` wins because nothing has matched yet at that line. A `#CASE`
    /// whose label does not parse as a number is treated as a label that never matches.
    fn select_case(&mut self, wanted: Option<u32>) {
        let Some(pos) = self.last_switch_pos() else { return };
        let Frame::Switch { value, active, matched, skipped } = self.stack[pos] else { return };
        if skipped || active {
            return;
        }
        let selected = match wanted {
            Some(k) => value == k,
            None => !matched,
        };
        let now = selected && self.active_excluding(pos);
        self.stack[pos] = Frame::Switch { value, active: now, matched: matched || now, skipped };
    }

    /// Consumes one `#`-line body as a control command, returning whether it was one. Control
    /// commands are seen even inside branches that emit nothing, so `#SKIP` checks the whole
    /// frame stack: a `#SKIP` sitting in a dead nested branch must not close the live `#CASE`
    /// that encloses it.
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
            "SWITCH" => {
                let n = rest.split_whitespace().next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                let v = self.rand_range(n);
                self.stack.push(Frame::Switch { value: v, active: false, matched: false, skipped: false });
                true
            }
            "SETSWITCH" => {
                let v = rest.split_whitespace().next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                self.stack.push(Frame::Switch { value: v, active: false, matched: false, skipped: false });
                true
            }
            "CASE" => {
                if let Some(k) = rest.split_whitespace().next().and_then(|s| s.parse::<u32>().ok()) {
                    self.select_case(Some(k));
                }
                true
            }
            "DEF" => {
                self.select_case(None);
                true
            }
            "SKIP" => {
                if self.active()
                    && let Some(pos) = self.last_switch_pos()
                    && let Frame::Switch { value, active: true, matched, .. } = self.stack[pos]
                {
                    self.stack[pos] = Frame::Switch { value, active: false, matched, skipped: true };
                }
                true
            }
            "ENDSW" => {
                if let Some(pos) = self.last_switch_pos() {
                    self.stack.truncate(pos);
                }
                true
            }
            _ => false,
        }
    }
}
