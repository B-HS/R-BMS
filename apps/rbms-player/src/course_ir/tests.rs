use rbms_course::{Course, CourseChart, CourseConstraint, CourseRun, StageResult};
use rbms_ir::{
    ChartId, ChartRankingQuery, ClearLamp, CourseSubmission, IrError, PlayerId, PlayerProfile, ReplayData, ScoreRecord, ScoreServer, ScoreSubmission,
    ServerInfo, SubmitResponse,
};

use super::{UNRELEASED_COURSE_REASON, build_course_submission, course_block_reason, submit};

const TEST_PLAYER: &str = "gkn";
const TEST_NOTES: u32 = 1_000;
const TEST_EX: u32 = 1_400;
const TEST_LAMP_HARD: u8 = 6;

fn chart(md5: &str, sha256: &str) -> CourseChart {
    CourseChart { md5: md5.to_string(), sha256: sha256.to_string(), title: "Stage".to_string() }
}

fn stage(ex: u32, gauge: f32, survived: bool) -> StageResult {
    StageResult {
        ex_score: ex,
        max_ex_score: TEST_NOTES * 2,
        notes: TEST_NOTES,
        counts: [500, 400, 70, 10, 15, 5],
        empty_poor: 4,
        fast: 120,
        slow: 140,
        combo_breaks: 25,
        max_combo: 620,
        combo_at_end: 33,
        gauge_value: gauge,
        clear: if survived { TEST_LAMP_HARD } else { rbms_course::CLEAR_FAILED },
        survived,
    }
}

fn two_stage_run(constraints: Vec<CourseConstraint>) -> CourseRun {
    let course = Course { name: "Grade".to_string(), charts: vec![chart("aa", "11"), chart("bb", "22")], constraints, ..Course::default() };
    let mut run = CourseRun::new(course, 100.0);
    run.advance(&stage(TEST_EX, 70.0, true));
    run.advance(&stage(TEST_EX, 64.0, true));
    run
}

/// A server that answers `submit_course` with whatever the test wants and nothing else.
struct CourseOnlyServer {
    answer: fn() -> Result<SubmitResponse, IrError>,
}

impl ScoreServer for CourseOnlyServer {
    fn health(&self) -> Result<ServerInfo, IrError> {
        Err(IrError::NotConfigured)
    }
    fn submit_score(&self, _sub: &ScoreSubmission) -> Result<SubmitResponse, IrError> {
        Err(IrError::NotConfigured)
    }
    fn chart_ranking(&self, _chart: &ChartId, _limit: u32) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::NotConfigured)
    }
    fn player_best(&self, _chart: &ChartId, _player: &PlayerId) -> Result<Option<ScoreRecord>, IrError> {
        Err(IrError::NotConfigured)
    }
    fn player_profile(&self, _player: &PlayerId) -> Result<PlayerProfile, IrError> {
        Err(IrError::NotConfigured)
    }
    fn rivals(&self, _player: &PlayerId) -> Result<Vec<PlayerProfile>, IrError> {
        Err(IrError::NotConfigured)
    }
    fn submit_course(&self, _sub: &CourseSubmission) -> Result<SubmitResponse, IrError> {
        (self.answer)()
    }
    fn upload_replay(&self, _chart: &ChartId, _replay: &ReplayData) -> Result<String, IrError> {
        Err(IrError::NotConfigured)
    }
    fn course_ranking_page(&self, _course_hash: &str, _query: &ChartRankingQuery) -> Result<Vec<ScoreRecord>, IrError> {
        Err(IrError::Unsupported)
    }
}

#[test]
fn a_submission_carries_the_course_totals_and_every_chart() {
    let run = two_stage_run(vec![]);
    let sub = build_course_submission(&run, TEST_PLAYER);

    assert_eq!(sub.api_version, rbms_ir::API_VERSION);
    assert_eq!(sub.course_hash, run.course.hash());
    assert_eq!(sub.player, PlayerId { id: TEST_PLAYER.to_string() });
    assert_eq!(sub.clear, ClearLamp::Hard, "the lamp the course finished on");
    assert_eq!(sub.ex_score, TEST_EX * 2);
    assert_eq!(sub.max_ex_score, TEST_NOTES * 4);
    assert_eq!(sub.max_combo, 620, "the best combo over the whole course");
    assert_eq!(sub.minbp, 60, "bad plus poor plus miss, over both stages");
    assert_eq!(sub.gauge_value, 64.0, "the gauge the course ended on");
    assert_eq!(sub.charts, vec![ChartId { md5: "aa".to_string(), sha256: "11".to_string() }, ChartId { md5: "bb".to_string(), sha256: "22".to_string() },]);
    assert!(sub.played_at > 0, "a real clock reading");
    assert_eq!(sub.trophy, None, "the course declares no trophies");
}

#[test]
fn the_judgment_breakdown_sums_every_stage() {
    let sub = build_course_submission(&two_stage_run(vec![]), TEST_PLAYER);
    assert_eq!(sub.judge.pgreat, 1000);
    assert_eq!(sub.judge.great, 800);
    assert_eq!(sub.judge.good, 140);
    assert_eq!(sub.judge.bad, 20);
    assert_eq!(sub.judge.poor, 30);
    assert_eq!(sub.judge.miss, 10);
    assert_eq!(sub.judge.fast, 240);
    assert_eq!(sub.judge.slow, 280);
    assert_eq!(sub.judge.combobreak, 50, "the caller's per-stage count, summed");
    assert_eq!(sub.judge.empty_poor, 8);
    assert_eq!(sub.judge.epg, 0, "the early/late split is per chart and is not summed");
    assert_eq!(sub.judge.avgjudge, 0);
}

#[test]
fn the_long_note_constraint_decides_the_reported_lntype() {
    assert_eq!(build_course_submission(&two_stage_run(vec![]), TEST_PLAYER).lntype, 0, "no constraint reports the default");
    assert_eq!(build_course_submission(&two_stage_run(vec![CourseConstraint::Ln]), TEST_PLAYER).lntype, 0);
    assert_eq!(build_course_submission(&two_stage_run(vec![CourseConstraint::Cn]), TEST_PLAYER).lntype, 1);
    assert_eq!(build_course_submission(&two_stage_run(vec![CourseConstraint::Hcn]), TEST_PLAYER).lntype, 2);
}

#[test]
fn a_failed_run_submits_the_failed_lamp_and_the_partial_totals() {
    let course = Course { name: "Grade".to_string(), charts: vec![chart("aa", "11"), chart("bb", "22")], ..Course::default() };
    let mut run = CourseRun::new(course, 100.0);
    run.advance(&stage(TEST_EX, 40.0, true));
    run.advance(&stage(200, 0.0, false));

    let sub = build_course_submission(&run, TEST_PLAYER);
    assert_eq!(sub.clear, ClearLamp::Failed);
    assert_eq!(sub.ex_score, TEST_EX + 200);
    assert_eq!(sub.gauge_value, 0.0);
}

#[test]
fn a_non_finite_gauge_is_clamped_before_it_leaves_the_client() {
    let course = Course { name: "Grade".to_string(), charts: vec![chart("aa", "11")], ..Course::default() };
    let mut run = CourseRun::new(course, 100.0);
    let mut only = stage(TEST_EX, 0.0, true);
    only.gauge_value = f32::NAN;
    run.advance(&only);

    let sub = build_course_submission(&run, TEST_PLAYER);
    assert!(sub.gauge_value.is_finite(), "a NaN gauge would serialise to null and be rejected");
}

#[test]
fn a_course_is_submittable_only_when_every_stage_was() {
    assert_eq!(course_block_reason(&[None, None, None]), None);
    assert_eq!(course_block_reason(&[]), None, "a course with no stages recorded blocks nothing by itself");
    assert_eq!(course_block_reason(&[None, Some("autoplay".to_string()), None]), Some("autoplay".to_string()));
    assert_eq!(
        course_block_reason(&[Some("replay playback".to_string()), Some("autoplay".to_string())]),
        Some("replay playback".to_string()),
        "the first stage that blocked is the reason reported"
    );
    assert_eq!(
        course_block_reason(&[None, Some(UNRELEASED_COURSE_REASON.to_string())]),
        Some(UNRELEASED_COURSE_REASON.to_string()),
        "the caller folds the release flag in as one more reason"
    );
}

#[test]
fn a_server_without_a_course_endpoint_is_not_an_error() {
    let server = CourseOnlyServer { answer: || Err(IrError::Unsupported) };
    let sub = build_course_submission(&two_stage_run(vec![]), TEST_PLAYER);
    let response = submit(&server, &sub).expect("an IR that predates courses is not a failure");
    assert!(!response.accepted, "nothing was stored, so nothing is claimed");
    assert_eq!(response.rank, None);
}

#[test]
fn a_real_submission_failure_is_reported() {
    let server = CourseOnlyServer { answer: || Err(IrError::NotConfigured) };
    let sub = build_course_submission(&two_stage_run(vec![]), TEST_PLAYER);
    assert!(matches!(submit(&server, &sub), Err(IrError::NotConfigured)));
}

#[test]
fn an_accepted_submission_passes_the_response_through() {
    let server = CourseOnlyServer { answer: || Ok(SubmitResponse { accepted: true, rank: Some(3), ..Default::default() }) };
    let sub = build_course_submission(&two_stage_run(vec![]), TEST_PLAYER);
    let response = submit(&server, &sub).expect("the server accepted it");
    assert!(response.accepted);
    assert_eq!(response.rank, Some(3));
}
