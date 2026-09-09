use rbms_judge::GaugeKind;

/// Parse a persisted gauge token back into the engine gauge, defaulting to NORMAL.
pub fn gauge_from_name(s: &str) -> GaugeKind {
    match s.to_ascii_lowercase().as_str() {
        "assist" | "assisteasy" => GaugeKind::AssistEasy,
        "easy" => GaugeKind::Easy,
        "hard" => GaugeKind::Hard,
        "exhard" => GaugeKind::ExHard,
        "hazard" => GaugeKind::Hazard,
        _ => GaugeKind::Normal,
    }
}

/// The settings-file token for an engine gauge. Lowercase and free of separators so it survives a
/// round trip through the configuration file and the account sync blob.
pub fn gauge_token(g: GaugeKind) -> &'static str {
    match g {
        GaugeKind::AssistEasy => "assist",
        GaugeKind::Easy => "easy",
        GaugeKind::Normal => "normal",
        GaugeKind::Hard => "hard",
        GaugeKind::ExHard => "exhard",
        GaugeKind::Hazard => "hazard",
    }
}
