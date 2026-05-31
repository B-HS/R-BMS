use crate::Judge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeKind {
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    Hazard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearType {
    NoPlay,
    Failed,
    AssistEasy,
    Easy,
    Normal,
    Hard,
    ExHard,
    FullCombo,
    Perfect,
    Max,
}

enum Modifier {
    Total,
    LimitIncrement,
    None,
}

struct Spec {
    modifier: Modifier,
    min: f32,
    max: f32,
    init: f32,
    border: f32,
    deltas: [f32; 6],
    guts: &'static [(f32, f32)],
}

const HARD_GUTS: &[(f32, f32)] = &[(10.0, 0.4), (20.0, 0.5), (30.0, 0.6), (40.0, 0.7), (50.0, 0.8)];

fn spec(kind: GaugeKind) -> Spec {
    use GaugeKind::*;
    use Modifier::*;
    match kind {
        AssistEasy => Spec { modifier: Total, min: 2.0, max: 100.0, init: 20.0, border: 60.0, deltas: [1.0, 1.0, 0.5, -1.5, -3.0, -0.5], guts: &[] },
        Easy => Spec { modifier: Total, min: 2.0, max: 100.0, init: 20.0, border: 80.0, deltas: [1.0, 1.0, 0.5, -1.5, -4.5, -1.0], guts: &[] },
        Normal => Spec { modifier: Total, min: 2.0, max: 100.0, init: 20.0, border: 80.0, deltas: [1.0, 1.0, 0.5, -3.0, -6.0, -2.0], guts: &[] },
        Hard => Spec { modifier: LimitIncrement, min: 0.0, max: 100.0, init: 100.0, border: 0.0, deltas: [0.15, 0.12, 0.03, -5.0, -10.0, -5.0], guts: HARD_GUTS },
        ExHard => Spec { modifier: LimitIncrement, min: 0.0, max: 100.0, init: 100.0, border: 0.0, deltas: [0.15, 0.06, 0.0, -8.0, -16.0, -8.0], guts: &[] },
        Hazard => Spec { modifier: Modifier::None, min: 0.0, max: 100.0, init: 100.0, border: 0.0, deltas: [0.15, 0.06, 0.0, -100.0, -100.0, -10.0], guts: &[] },
    }
}

/// One groove gauge (beatoraja `GrooveGauge`). Deltas are pre-modified at construction
/// by the gauge's modifier (TOTAL scales gains by chart total/notes; LIMIT_INCREMENT
/// caps the gain). A note's judge adds `deltas[judge]`, with HARD "guts" softening damage
/// at low values. Cleared when `value >= border` and `> 0` at the end.
#[derive(Debug, Clone)]
pub struct Gauge {
    kind: GaugeKind,
    value: f32,
    min: f32,
    max: f32,
    border: f32,
    deltas: [f32; 6],
    guts: &'static [(f32, f32)],
}

impl Gauge {
    pub fn new(kind: GaugeKind, total: f64, notes: usize) -> Self {
        let s = spec(kind);
        let notes = notes.max(1) as f64;
        let total = if total > 0.0 { total } else { 200.0 };
        let mut deltas = s.deltas;
        match s.modifier {
            Modifier::Total => {
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d = (*d as f64 * total / notes) as f32;
                    }
                }
            }
            Modifier::LimitIncrement => {
                let pg = (((2.0 * total - 320.0) / notes).min(0.15)).max(0.0) as f32;
                for d in deltas.iter_mut() {
                    if *d > 0.0 {
                        *d *= pg / 0.15;
                    }
                }
            }
            Modifier::None => {}
        }
        Gauge { kind, value: s.init, min: s.min, max: s.max, border: s.border, deltas, guts: s.guts }
    }

    pub fn update(&mut self, judge: Judge) {
        if self.value <= 0.0 {
            return;
        }
        let mut inc = self.deltas[judge as usize];
        if inc < 0.0 {
            for g in self.guts {
                if self.value < g.0 {
                    inc *= g.1;
                    break;
                }
            }
        }
        self.value = (self.value + inc).clamp(self.min, self.max);
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn kind(&self) -> GaugeKind {
        self.kind
    }

    pub fn is_cleared(&self) -> bool {
        self.value > 0.0 && self.value >= self.border
    }
}

/// Resolve the clear lamp from the final gauge state and judge tally.
pub fn clear_lamp(gauge: &Gauge, counts: &[u32; 6], max_combo: u32, total_notes: u32) -> ClearType {
    if total_notes == 0 {
        return ClearType::NoPlay;
    }
    if !gauge.is_cleared() {
        return ClearType::Failed;
    }
    let broke = counts[3] + counts[4] + counts[5] > 0;
    if !broke && max_combo == total_notes {
        if counts[1] == 0 && counts[2] == 0 {
            return ClearType::Max;
        }
        if counts[2] == 0 {
            return ClearType::Perfect;
        }
        return ClearType::FullCombo;
    }
    match gauge.kind {
        GaugeKind::AssistEasy => ClearType::AssistEasy,
        GaugeKind::Easy => ClearType::Easy,
        GaugeKind::Normal => ClearType::Normal,
        GaugeKind::Hard => ClearType::Hard,
        GaugeKind::ExHard | GaugeKind::Hazard => ClearType::ExHard,
    }
}
