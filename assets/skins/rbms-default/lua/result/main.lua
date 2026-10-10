local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")
local layout = require("lua.result.layout")
local score = require("lua.result.score")
local judges = require("lua.result.judges")
local graphs = require("lua.result.graphs")

local result = {}

local TIME = { input = 500, scene = 3600000, fadeout = 500 }

local HEAD = {
    bar_width = 8,
    text_gap = 32,
    grow = 350,
    title_drop = { alone = 48, subtitled = 30 },
    subtitle_drop = 98,
    baseline_drop = 160,
    notice_drop = 4,
    keys_width = 120,
    level = { sprite = "num32", width = 80, gap = 24 },
    name = { width = 136, gap = 12 },
}

local VERDICT = {
    chart = { cleared = "STAGE CLEARED", failed = "STAGE FAILED" },
    course = { cleared = "COURSE CLEARED", failed = "COURSE FAILED" },
}

local FOOTER = { hint_top = 1022, hint_fade = 200 }
local HINTS = {
    chart = "ENTER  CONTINUE        R  RETRY        N  NEXT CHART",
    course = "ENTER  CONTINUE",
    gauge = {
        { modes = { props.option.KEYS_7, props.option.KEYS_14, props.option.KEYS_9 }, text = "        KEY 6  GAUGE" },
        { modes = { props.option.KEYS_5, props.option.KEYS_10 }, text = "        TURNTABLE  GAUGE" },
    },
}
local CLOSING = { cover = 400 }

function result.header(course)
    return {
        type = course and props.skin_type.COURSE_RESULT or props.skin_type.RESULT,
        name = theme.name .. (course and " (Course Result)" or " (Result)"),
        author = theme.author,
        w = theme.width,
        h = theme.height,
        input = TIME.input,
        scene = TIME.scene,
        fadeout = TIME.fadeout,
    }
end

local function verdict(s, left, text_x, course)
    local order = layout.order.head
    local area = layout.head()
    local words = course and VERDICT.course or VERDICT.chart
    local size = theme.type.label
    local box = { x = text_x, top = area.top, w = left.x + left.w - text_x }
    local bar = { x = left.x, top = area.top, w = HEAD.bar_width, h = area.h }
    local outcomes = {
        { op = props.option.RESULT_CLEAR, word = words.cleared, color = theme.color.accent },
        { op = props.option.RESULT_FAIL, word = words.failed, color = theme.judge.poor },
    }
    for _, outcome in ipairs(outcomes) do
        s:text({ text = outcome.word, size = size }, box, layout.appear(order, { op = { outcome.op }, color = outcome.color }))
        if course then
            s:rect(bar, outcome.color, motion.grow_down(0, HEAD.grow, { op = { outcome.op } }))
        end
    end
    if not course then
        parts.difficulty_rect(s, bar, motion.grow_down(0, HEAD.grow))
    end
    local notice = { x = text_x, top = area.top + HEAD.notice_drop, w = box.w }
    local unsaved = layout.appear(order, { op = { props.option.DISABLE_SAVE_SCORE }, color = theme.color.muted })
    s:text({ text = "SCORE NOT SAVED", size = theme.type.caption, align = "right" }, notice, unsaved)
end

local function keys(s, left, baseline)
    local size = theme.type.label
    local box = { x = left.x + left.w - HEAD.keys_width, top = theme.text_top(baseline, size), w = HEAD.keys_width }
    parts.key_mode_text(s, { size = size, align = "right" }, box, layout.appear(layout.order.head, { color = theme.color.sub }))
    return box.x
end

local function chart_line(s, main_state, left, text_x, baseline)
    local order = layout.order.head
    local keys_x = keys(s, left, baseline)
    local level = main_state.number(props.number.PLAYLEVEL)
    local spec = { sprite = HEAD.level.sprite, ref = props.number.PLAYLEVEL, known = level, fit = HEAD.level.width, align = "right" }
    local box = { x = keys_x - HEAD.level.gap - HEAD.level.width, w = HEAD.level.width, baseline = baseline }
    local level_x = box.x + box.w
    if ui.has_value(level) then
        level_x = level_x - s:number_box(spec, box).span
        parts.by_difficulty(function(_, op, color)
            s:number(spec, box, layout.appear(order, { op = { op }, color = color }))
        end)
    end
    local size = theme.type.label
    local name = { x = level_x - HEAD.name.gap - HEAD.name.width, top = theme.text_top(baseline, size), w = HEAD.name.width }
    parts.difficulty_text(s, { size = size, align = "right" }, name, layout.appear(order))
    local width = name.x - theme.grid.gutter - text_x
    local artist = { ref = props.text.FULLARTIST, size = size, overflow = props.text_overflow.SHRINK }
    s:text(artist, { x = text_x, top = theme.text_top(baseline, size), w = width }, layout.appear(order, { color = theme.color.text }))
    return width
end

local function course_line(s, main_state, left, text_x, baseline)
    local keys_x = keys(s, left, baseline)
    local count = #layout.stage_titles(main_state)
    if count == 0 then
        return
    end
    local size = theme.type.label
    local box = { x = text_x, top = theme.text_top(baseline, size), w = keys_x - theme.grid.gutter - text_x }
    local word = count == 1 and " STAGE" or " STAGES"
    s:text({ text = count .. word, size = size }, box, layout.appear(layout.order.head, { color = theme.color.sub }))
end

local function entering(index, color)
    return motion.slide_in(motion.staggered(index), theme.time.slide, -theme.time.slide_distance, 0, { color = color })
end

local function head(s, main_state, course)
    local left = layout.left()
    local area = layout.head()
    local text_x = left.x + HEAD.bar_width + HEAD.text_gap
    local baseline = area.top + HEAD.baseline_drop
    verdict(s, left, text_x, course)
    local subtitled = not course and main_state.text(props.text.SUBTITLE) ~= ""
    local title_top = area.top + (subtitled and HEAD.title_drop.subtitled or HEAD.title_drop.alone)
    local title = { ref = props.text.TITLE, size = theme.type.title, overflow = props.text_overflow.SHRINK }
    s:text(title, { x = text_x, top = title_top, w = left.x + left.w - text_x }, entering(0, theme.color.text))
    if course then
        course_line(s, main_state, left, text_x, baseline)
        return
    end
    local width = chart_line(s, main_state, left, text_x, baseline)
    if subtitled then
        local subtitle = { ref = props.text.SUBTITLE, size = theme.type.body, overflow = props.text_overflow.SHRINK }
        s:text(subtitle, { x = text_x, top = area.top + HEAD.subtitle_drop, w = width }, entering(1, theme.color.sub))
    end
end

local function footer(s, course)
    local grid = theme.grid
    local size = theme.type.caption
    local box = { x = grid.margin, top = FOOTER.hint_top, w = theme.content_width() }
    local plain = course and HINTS.course or HINTS.chart
    local elsewhere = {}
    for _, turning in ipairs(HINTS.gauge) do
        for _, op in ipairs(turning.modes) do
            table.insert(elsewhere, -op)
            local shown = motion.fade_in(0, FOOTER.hint_fade, { timer = props.timer.STARTINPUT, op = { op }, color = theme.color.muted })
            s:text({ text = plain .. turning.text, size = size }, box, shown)
        end
    end
    local shown = motion.fade_in(0, FOOTER.hint_fade, { timer = props.timer.STARTINPUT, op = elsewhere, color = theme.color.muted })
    s:text({ text = plain, size = size }, box, shown)
end

function result.build(spec)
    local course = spec.course
    local header = result.header(course)
    if skin_config == nil then
        return header
    end
    local main_state = require("main_state")
    local s = ui.new(header)
    s:atlas("common")
    s:atlas("result")
    parts.backdrop(s)
    parts.frame(s, course and "COURSE RESULT" or "RESULT")
    head(s, main_state, course)
    score.build(s, main_state, course)
    judges.build(s, main_state, course)
    graphs.build(s, main_state, course)
    footer(s, course)
    parts.fade_from_black(s, theme.time.fade)
    parts.fade_to_black(s, 0, CLOSING.cover)
    return s:finish()
end

return result
