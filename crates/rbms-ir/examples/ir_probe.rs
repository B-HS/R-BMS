use std::collections::HashMap;

use rbms_ir::*;

fn main() {
    let url = std::env::args().nth(1).expect("usage: ir_probe <base-url>");
    let s = HttpScoreServer::new(url, None);

    match s.health() {
        Ok(i) => println!("health OK: {} v{} ir_compat={} caps={:?}", i.name, i.version, i.ir_compat, i.capabilities),
        Err(e) => println!("health ERR: {e}"),
    }

    let sub = ScoreSubmission {
        api_version: API_VERSION,
        chart: ChartId { md5: "6c4288d29b9870c88c744f7a95348819".into(), sha256: "980459...".into() },
        player: PlayerId { id: "guest".into() },
        mode: "BEAT_7K".into(),
        clear: ClearLamp::Hard,
        ex_score: 1488,
        max_ex_score: 1624,
        judge: JudgeBreakdown { pgreat: 712, great: 64, good: 21, bad: 8, poor: 5, miss: 2, fast: 30, slow: 40, combobreak: 15, ..Default::default() },
        max_combo: 540,
        total_notes: 812,
        passnotes: 812,
        minbp: 15,
        gauge_value: 86.0,
        options: PlayOptions {
            gauge: GaugeType::Hard,
            random: RandomOption::Random,
            random_p2: None,
            scratch_auto: false,
            lntype: 1,
            input_device: "keyboard".into(),
            assist: vec![],
            option: 0,
            judge_rate: 100,
            offset_ms: 0,
            constant: false,
            hispeed: 3.0,
            lift: 0.0,
            lane_cover: 0.0,
            total_override: 0.0,
            autoplay: false,
            auto_offset: false,
            scratch_left: false,
            green_number: 0.0,
        },
        played_at: 1_700_000_000_000,
        client: "rbms/0.1".into(),
        replay_id: None,
        seed: 0,
        judge_algorithm: String::new(),
        rule: String::new(),
        skin: "NORMAL".into(),
        client_build_sha256: None,
        client_platform: None,
        extra: HashMap::new(),
    };
    match s.submit_score(&sub) {
        Ok(r) => println!("submit OK: accepted={} rank={:?} msg={:?}", r.accepted, r.rank, r.message),
        Err(e) => println!("submit ERR: {e}"),
    }
}
