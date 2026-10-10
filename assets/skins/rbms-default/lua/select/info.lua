local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")
local kit = require("lua.select.kit")

local info = {}

local OPTION = { STAGE_SHOW = parts.stage_image.show }
local ON_CHART = props.option.SONGBAR
local ON_FOLDER = props.option.FOLDERBAR
local ON_COURSE = props.option.GRADEBAR
local UNPLAYED = props.option.BAR_NOT_PLAYED
local NOTHING = "-"

local REGION = { x = 64, width = 1032 }

local HEAD = { top = 136, height = 200, bar_width = 8, text_gap = 32 }
local HEAD_LINES = {
    { ref = props.text.DIRECTORY, top = 136, size = theme.type.caption, color = theme.color.muted },
    { ref = props.text.GENRE, top = 162, size = theme.type.label, color = theme.color.sub },
    { ref = props.text.TITLE, top = 196, size = theme.type.title, color = theme.color.text },
    { ref = props.text.SUBTITLE, top = 270, size = theme.type.label, color = theme.color.sub },
    { ref = props.text.FULLARTIST, top = 302, size = theme.type.body, color = theme.color.text },
}
local HEAD_COURSE = { top = 196, size = theme.type.title, label = "COURSE" }

local STAGE = { x = 64, top = 368, width = 424, height = 318, fade = 200, missing_at = 300 }

local FACTS = {
    beside_stage = { { x = 520, width = 176 }, { x = 712, width = 176 }, { x = 904, width = 192 } },
    alone = { { x = 64, width = 312 }, { x = 408, width = 312 }, { x = 752, width = 344 } },
    rows = { 368, 485, 602 },
    drop = 64,
    sprite = "num44",
    level_digits = 3,
    notes_digits = 5,
    total_digits = 4,
}

local RECORD = {
    rule_top = 702,
    rows = { 718, 808, 898 },
    drop = 64,
    sprite = "num44",
    digits = 5,
    rank = { x = 64, width = 272, caption_top = 718, top = 752, size = theme.type.display },
    clear = { x = 64, width = 272, row = 3 },
    left = { x = 368, width = 196 },
    right = { x = 580, width = 212 },
}
local CLEAR_OPTIONS = {
    [0] = props.option.BAR_NOT_PLAYED,
    props.option.BAR_FAILED,
    props.option.BAR_ASSIST_EASY,
    props.option.BAR_LIGHT_ASSIST_EASY,
    props.option.BAR_EASY,
    props.option.BAR_NORMAL,
    props.option.BAR_HARD,
    props.option.BAR_EXHARD,
    props.option.BAR_FULL_COMBO,
    props.option.BAR_PERFECT,
    props.option.BAR_MAX,
}

local JUDGES = {
    x = 824,
    width = 272,
    top = 718,
    pitch = 50,
    sprite = "num24",
    digits = 5,
    bar_drop = 26,
    bar_height = 6,
    grow = 300,
    rates = { props.rate.RATE_PGREAT, props.rate.RATE_GREAT, props.rate.RATE_GOOD, props.rate.RATE_BAD, props.rate.RATE_POOR },
}

local FOLDER = {
    top = 368,
    count_baseline = 448,
    count_sprite = "num64",
    digits = 5,
    lamps_top = 480,
    first_row = 512,
    pitch = 72,
    row_height = 40,
    per_column = 6,
    columns = { 64, 580 },
    width = 484,
    chip_width = 8,
    name_x = 24,
    size = theme.type.body,
    sprite = "num32",
}

local COURSE = { top = 368, first_row = 400, pitch = 56, stages = 5, constraints_top = 898, constraints_value_top = 930 }
local CONSTRAINTS = {
    { op = 1002, label = "CLASS" },
    { op = 1003, label = "MIRROR" },
    { op = 1004, label = "RANDOM" },
    { op = 1005, label = "NO SPEED" },
    { op = 1006, label = "NO GOOD" },
    { op = 1007, label = "NO GREAT" },
    { op = 1010, label = "LR2 GAUGE" },
    { op = 1011, label = "5 KEYS GAUGE" },
    { op = 1012, label = "7 KEYS GAUGE" },
    { op = 1013, label = "9 KEYS GAUGE" },
    { op = 1014, label = "24 KEYS GAUGE" },
    { op = 1015, label = "LN" },
    { op = 1016, label = "CN" },
    { op = 1017, label = "HCN" },
}
local CONSTRAINT_JOINT = "   /   "
local NO_CONSTRAINT = "NONE"

local function head(s)
    local bar = { x = REGION.x, top = HEAD.top, w = HEAD.bar_width, h = HEAD.height }
    s:rect(bar, theme.color.line)
    parts.difficulty_rect(s, bar, { op = { ON_CHART } })
    s:rect(bar, theme.color.accent, { op = { ON_FOLDER } })
    s:rect(bar, theme.color.accent, { op = { ON_COURSE } })

    local x = REGION.x + HEAD.bar_width + HEAD.text_gap
    local width = REGION.x + REGION.width - x
    for _, line in ipairs(HEAD_LINES) do
        local spec = { ref = line.ref, size = line.size, overflow = props.text_overflow.SHRINK }
        s:text(spec, { x = x, top = line.top, w = width }, kit.sliding({ color = line.color }))
    end
    local course_title = { x = x, top = HEAD_COURSE.top, w = width }
    s:text({ text = HEAD_COURSE.label, size = HEAD_COURSE.size }, course_title, kit.sliding({ color = theme.color.text, op = { ON_COURSE } }))
end

local function stage(s)
    local box = { x = STAGE.x, top = STAGE.top, w = STAGE.width, h = STAGE.height }
    local shown = { OPTION.STAGE_SHOW, ON_CHART }
    parts.panel(s, box, { op = shown })
    local arriving = motion.fade_in(0, STAGE.fade, { op = shown, timer = props.timer.SONGBAR_CHANGE, stretch = props.stretch.FIT_INNER })
    s:reference(props.image.STAGEFILE, box, arriving)
    local size = theme.type.caption
    local unpictured = { op = { OPTION.STAGE_SHOW, ON_CHART, props.option.NO_STAGEFILE }, timer = props.timer.SONGBAR_CHANGE, color = theme.color.muted }
    local label = { x = box.x, top = theme.centered_top(box, size), w = box.w }
    s:text({ text = parts.stage_image.missing, size = size, align = "center" }, label, motion.fade_in(STAGE.missing_at, STAGE.fade, unpictured))
end

local fact_columns = FACTS.beside_stage

local function fact(s, column, row, label)
    local cell = fact_columns[column]
    local top = FACTS.rows[row]
    parts.caption(s, label, { x = cell.x, top = top, w = cell.width }, { op = { ON_CHART } })
    return cell.x, top + FACTS.drop, cell.width
end

local function fact_number(s, column, row, label, ref, digits)
    local x, baseline = fact(s, column, row, label)
    local spec = { sprite = FACTS.sprite, ref = ref, digit = digits, saturate = true }
    s:number(spec, { x = x, baseline = baseline }, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
end

local function fact_text(s, column, row, label, size, spec, span)
    local x, baseline, width = fact(s, column, row, label)
    local box = { x = x, top = theme.text_top(baseline, size), w = span or width }
    local written = ui.merge(spec, { size = size, overflow = props.text_overflow.SHRINK })
    s:text(written, box, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
end

local function bpm_range(main_state)
    return function()
        local low = main_state.number(props.number.MINBPM)
        local high = main_state.number(props.number.MAXBPM)
        if not ui.has_value(high) then
            return ""
        end
        if not ui.has_value(low) or low == high then
            return string.format("%d", high)
        end
        return string.format("%d - %d", low, high)
    end
end

local function chart_length(main_state)
    return function()
        local minutes = main_state.number(props.number.SONGLENGTH_MINUTE)
        local seconds = main_state.number(props.number.SONGLENGTH_SECOND)
        if not (ui.has_value(minutes) and ui.has_value(seconds)) then
            return ""
        end
        return string.format("%d:%02d", minutes, seconds)
    end
end

local function facts(s, main_state)
    local word = { size = theme.type.body, overflow = props.text_overflow.SHRINK }
    local x, baseline = fact(s, 1, 1, "LEVEL")
    local level = { sprite = FACTS.sprite, ref = props.number.PLAYLEVEL, digit = FACTS.level_digits, saturate = true }
    local name_x, name_baseline, name_width = fact(s, 2, 3, "DIFFICULTY")
    local name = { x = name_x, top = theme.text_top(name_baseline, word.size), w = name_width }
    parts.by_difficulty(function(_, op, color, label)
        s:number(level, { x = x, baseline = baseline }, kit.arriving({ color = color, op = { ON_CHART, op } }))
        s:text(ui.merge(word, { text = label }), name, kit.arriving({ color = color, op = { ON_CHART, op } }))
    end)

    fact_text(s, 2, 1, "BPM", theme.type.heading, { value = bpm_range(main_state) })
    fact_number(s, 3, 1, "NOTES", props.number.TOTALNOTES, FACTS.notes_digits)
    fact_text(s, 1, 2, "LENGTH", theme.type.heading, { value = chart_length(main_state) })

    local judge_x, judge_baseline, judge_width = fact(s, 2, 2, "JUDGE")
    local judge = { x = judge_x, top = theme.text_top(judge_baseline, word.size), w = judge_width }
    parts.judge_rank_text(s, word, judge, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))

    fact_number(s, 3, 2, "TOTAL", props.number.SONGGAUGE_TOTAL, FACTS.total_digits)

    local keys_x, keys_baseline, keys_width = fact(s, 1, 3, "KEYS")
    local keys = { x = keys_x, top = theme.text_top(keys_baseline, word.size), w = keys_width }
    parts.key_mode_text(s, word, keys, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
end

local function score_rate(main_state)
    return function()
        local whole = main_state.number(props.number.SCORE_RATE)
        local rest = main_state.number(props.number.SCORE_RATE_AFTERDOT)
        if not (ui.has_value(whole) and ui.has_value(rest)) then
            return ""
        end
        return string.format("%d.%02d%%", whole, rest)
    end
end

local function record_cell(s, column, row, label)
    local top = RECORD.rows[row]
    parts.caption(s, label, { x = column.x, top = top, w = column.width }, { op = { ON_CHART } })
    local baseline = top + RECORD.drop
    local size = theme.type.heading
    local dash = { x = column.x, top = theme.text_top(baseline, size), w = column.width }
    s:text({ text = NOTHING, size = size }, dash, { color = theme.color.muted, op = { ON_CHART, UNPLAYED } })
    return baseline
end

local function record_number(s, column, row, label, ref)
    local baseline = record_cell(s, column, row, label)
    local spec = { sprite = RECORD.sprite, ref = ref, digit = RECORD.digits, saturate = true }
    s:number(spec, { x = column.x, baseline = baseline }, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
end

local function rank(s)
    local cell = RECORD.rank
    parts.caption(s, "RANK", { x = cell.x, top = cell.caption_top, w = cell.width }, { op = { ON_CHART } })
    local box = { x = cell.x, top = cell.top, w = cell.width }
    for index, label in ipairs(theme.rank_name) do
        local op = { ON_CHART, -UNPLAYED, props.option.RANK_AAA + index - 1 }
        s:text({ text = label, size = cell.size }, box, kit.arriving({ color = theme.color.text, op = op }))
    end
    s:text({ text = NOTHING, size = cell.size }, box, { color = theme.color.muted, op = { ON_CHART, UNPLAYED } })
end

local function clear(s)
    local cell = RECORD.clear
    local top = RECORD.rows[cell.row]
    parts.caption(s, "CLEAR", { x = cell.x, top = top, w = cell.width }, { op = { ON_CHART } })
    local size = theme.type.body
    local box = { x = cell.x, top = theme.text_top(top + RECORD.drop, size), w = cell.width }
    for index = 0, theme.clear_last do
        local lamp = theme.clear[index]
        local spec = { text = lamp.name, size = size, overflow = props.text_overflow.SHRINK }
        s:text(spec, box, kit.arriving({ color = theme.clear_text(lamp), op = { ON_CHART, CLEAR_OPTIONS[index] } }))
    end
end

local function judges(s)
    for index, rate in ipairs(JUDGES.rates) do
        local row = parts.judges[index]
        local top = JUDGES.top + (index - 1) * JUDGES.pitch
        local size = theme.type.caption
        s:text({ text = row.name, size = size }, { x = JUDGES.x, top = top, w = JUDGES.width }, { color = row.color, op = { ON_CHART } })
        local spec = { sprite = JUDGES.sprite, ref = row.count, digit = JUDGES.digits, saturate = true, align = "right" }
        local baseline = top + theme.cap_height(size)
        s:number(spec, { x = JUDGES.x, baseline = baseline, w = JUDGES.width }, kit.arriving({ color = theme.color.text, op = { ON_CHART, -UNPLAYED } }))
        local dash = { x = JUDGES.x, top = top, w = JUDGES.width }
        s:text({ text = NOTHING, size = size, align = "right" }, dash, { color = theme.color.muted, op = { ON_CHART, UNPLAYED } })
        local track = { x = JUDGES.x, top = top + JUDGES.bar_drop, w = JUDGES.width, h = JUDGES.bar_height }
        s:rect(track, theme.color.panel, { op = { ON_CHART } })
        local grown = motion.grow_right(0, JUDGES.grow, { color = row.color, op = { ON_CHART }, timer = props.timer.SONGBAR_CHANGE })
        s:object("graph", kit.meter(s, rate), track, grown)
    end
end

local function record(s, main_state)
    s:rect({ x = REGION.x, top = RECORD.rule_top, w = REGION.width, h = theme.grid.rule }, theme.color.line, { op = { ON_CHART } })
    rank(s)
    clear(s)
    record_number(s, RECORD.left, 1, "EX SCORE", props.number.SCORE)
    record_number(s, RECORD.left, 2, "MAX COMBO", props.number.MAXCOMBO)
    record_number(s, RECORD.left, 3, "PLAY COUNT", props.number.PLAYCOUNT)
    local baseline = record_cell(s, RECORD.right, 1, "RATE")
    local size = theme.type.heading
    local rate = { x = RECORD.right.x, top = theme.text_top(baseline, size), w = RECORD.right.width }
    local spec = { value = score_rate(main_state), size = size, overflow = props.text_overflow.SHRINK }
    s:text(spec, rate, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
    record_number(s, RECORD.right, 2, "MISS COUNT", props.number.MISSCOUNT)
    record_number(s, RECORD.right, 3, "CLEAR COUNT", props.number.CLEARCOUNT)
    judges(s)
end

local function folder(s)
    local on = { op = { ON_FOLDER } }
    parts.caption(s, "CHARTS", { x = REGION.x, top = FOLDER.top, w = REGION.width }, on)
    local total = { sprite = FOLDER.count_sprite, ref = props.number.FOLDER_TOTALSONGS, digit = FOLDER.digits, saturate = true }
    s:number(total, { x = REGION.x, baseline = FOLDER.count_baseline }, kit.arriving({ color = theme.color.text, op = { ON_FOLDER } }))
    parts.caption(s, "CLEAR LAMPS", { x = REGION.x, top = FOLDER.lamps_top, w = REGION.width }, on)
    for index = 1, theme.clear_last + 1 do
        local lamp = theme.clear[theme.clear_last + 1 - index]
        local column = math.floor((index - 1) / FOLDER.per_column)
        local x = FOLDER.columns[column + 1]
        local top = FOLDER.first_row + ((index - 1) % FOLDER.per_column) * FOLDER.pitch
        local row = { x = x, top = top, w = FOLDER.width, h = FOLDER.row_height }
        local text_top = theme.centered_top(row, FOLDER.size)
        s:rect({ x = x, top = top, w = FOLDER.chip_width, h = FOLDER.row_height }, lamp.color, on)
        local name = { x = x + FOLDER.name_x, top = text_top, w = FOLDER.width - FOLDER.name_x }
        s:text({ text = lamp.name, size = FOLDER.size }, name, { color = theme.color.sub, op = { ON_FOLDER } })
        local ref = props.number.FOLDER_MAX - (index - 1)
        local count = { sprite = FOLDER.sprite, ref = ref, digit = FOLDER.digits, saturate = true, align = "right" }
        s:number(count, { x = x, top = text_top, w = FOLDER.width }, kit.arriving({ color = theme.color.text, op = { ON_FOLDER } }))
    end
end

local function constraints(main_state)
    return function()
        local held = {}
        for _, constraint in ipairs(CONSTRAINTS) do
            if main_state.option(constraint.op) then
                table.insert(held, constraint.label)
            end
        end
        if #held == 0 then
            return NO_CONSTRAINT
        end
        return table.concat(held, CONSTRAINT_JOINT)
    end
end

local function course(s, main_state)
    local on = { op = { ON_COURSE } }
    parts.caption(s, "STAGES", { x = REGION.x, top = COURSE.top, w = REGION.width }, on)
    for stage_index = 1, COURSE.stages do
        local spec = { ref = props.text.COURSE1_TITLE + stage_index - 1, size = theme.type.body, overflow = props.text_overflow.SHRINK }
        local box = { x = REGION.x, top = COURSE.first_row + (stage_index - 1) * COURSE.pitch, w = REGION.width }
        s:text(spec, box, kit.sliding({ color = theme.color.text, op = { ON_COURSE } }))
    end
    parts.caption(s, "CONSTRAINTS", { x = REGION.x, top = COURSE.constraints_top, w = REGION.width }, on)
    local spec = { value = constraints(main_state), size = theme.type.label, overflow = props.text_overflow.SHRINK }
    local box = { x = REGION.x, top = COURSE.constraints_value_top, w = REGION.width }
    s:text(spec, box, kit.arriving({ color = theme.color.sub, op = { ON_COURSE } }))
end

function info.build(s, main_state)
    fact_columns = parts.stage_image_shown() and FACTS.beside_stage or FACTS.alone
    head(s)
    stage(s)
    facts(s, main_state)
    record(s, main_state)
    folder(s)
    course(s, main_state)
end

return info
