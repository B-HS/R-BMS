local i18n = require("lua.i18n")

local words = {}

local t = i18n.t

local function listed(prefix, first, last)
    local entries = {}
    for index = first, last do
        entries[index] = t(prefix .. index)
    end
    return entries
end

local function lines(prefix, count)
    local entries = {}
    for line = 1, count do
        entries[line] = t(prefix .. "_" .. line)
    end
    return entries
end

words.switch = { [0] = "OFF", "ON" }
words.bga = { [0] = "ON", [2] = "OFF" }
words.fix = { [0] = "OFF", "START BPM", "MAX BPM", "MAIN BPM", "MIN BPM" }
words.shift = { [0] = "NONE", "CONTINUE", "SURVIVAL TO GROOVE", "BEST CLEAR", "SELECT TO UNDER" }
words.algorithm = { [0] = "COMBO", "DURATION", "LOWEST" }
words.algorithm_set_elsewhere = "SCORE"

words.random = listed("words_random_", 0, 7)

words.gauge = {}
for gauge = 0, 5 do
    words.gauge[gauge] = lines("words_gauge_" .. gauge, 2)
end

words.fix_notes = listed("words_fix_note_", 0, 4)

words.target = t("words_target")

words.ex_judge = listed("words_ex_judge_", 0, 1)
words.constant = listed("words_constant_", 0, 1)
words.scratch_auto = listed("words_scratch_auto_", 0, 1)
words.legacy = listed("words_legacy_", 0, 1)
words.keep_green = listed("words_keep_green_", 0, 1)
words.lane_cover = listed("words_lane_cover_", 0, 1)
words.lift = listed("words_lift_", 0, 1)
words.hidden = listed("words_hidden_", 0, 1)

words.bga_notes = { [0] = t("words_bga_shown"), [2] = t("words_bga_hidden") }
words.shift_notes = {
    [0] = lines("words_shift_note_0", 1),
    lines("words_shift_note_1", 2),
    lines("words_shift_note_2", 2),
    lines("words_shift_note_3", 2),
    lines("words_shift_note_4", 2),
}
words.auto_adjust = listed("words_auto_adjust_", 0, 1)
words.algorithm_notes = listed("words_algorithm_note_", 0, 2)
words.algorithm_notes[-1] = t("words_algorithm_note_score")

words.display_time = {
    name = t("sheet_note_display_time"),
    off = "OFF",
    off_note = t("words_display_off_note"),
    off_lines = { t("words_display_off_line"), "(SELECT + KEY 5)" },
    away_note = t("words_display_away_note"),
    away_lines = lines("words_display_away_line", 2),
    held_lines = lines("words_display_held_line", 2),
    green = "GREEN NUMBER %d",
}

words.judge_timing = {
    name = t("sheet_judge_timing"),
    zero = t("words_timing_zero"),
    earlier = i18n.template("words_timing_earlier"),
    later = i18n.template("words_timing_later"),
    advice = lines("words_timing_advice", 2),
}

words.guide = {
    { title = t("words_guide_1_title"), line = t("words_guide_1_line") },
    { title = t("words_guide_2_title"), line = t("words_guide_2_line") },
    { title = t("words_guide_3_title"), line = t("words_guide_3_line") },
}

words.assist_rule = {
    title = t("words_assist_title"),
    lead = lines("words_assist_lead", 2),
    rows = {
        { gauge = "EX HARD, HAZARD", grade = 4 },
        { gauge = "HARD", grade = 3 },
        { gauge = "NORMAL", grade = 2 },
        { gauge = "EASY", grade = 1 },
        { gauge = "ASSIST EASY", grade = 0 },
    },
    foot = lines("words_assist_foot", 2),
}

return words
