local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local parts = require("lua.parts")
local layout = require("lua.play.layout")

local info = {}

local EX_PER_NOTE = 2
local SIGN_PLACES = 1
local STAGE_LABELS = {
    { op = props.option.COURSE_STAGE1, label = "STAGE 1" },
    { op = props.option.COURSE_STAGE1 + 1, label = "STAGE 2" },
    { op = props.option.COURSE_STAGE1 + 2, label = "STAGE 3" },
    { op = props.option.COURSE_STAGE1 + 3, label = "STAGE 4" },
    { op = props.option.COURSE_STAGE_FINAL, label = "FINAL STAGE" },
}

local HEAD = {
    bar = 8,
    gap = 24,
    title = theme.type.body,
    line = theme.type.caption,
    name_widths = { [0] = 100, 92, 81, 61, 90, 68 },
    level_gap = 8,
    artist_gap = 24,
    stage_width = 200,
    stage = theme.type.label,
    narrow_below = 480,
    narrow_stage_width = 128,
}
local BGA = { inset = 24 }
local SCORE = {
    most_pitch = 72,
    first = 32,
    below_last = 22,
    label_width = 120,
    wide_from = 560,
    column_gap = 32,
    big = "num44",
    plain = "num32",
    signed = "snum32",
    combo_digits = 5,
    rate_digits = 3,
    fraction_digits = 2,
    percent_width = 20,
    dot = 3,
    dot_gap = 4,
}
local COUNTS = {
    head = 26,
    rise = 20,
    most_pitch = 36,
    count = "num24",
    split = "num18",
    count_share = 0.52,
    fast_share = 0.76,
    unbounded_digits = 4,
}
local OPAQUE = 255
local TIMING = {
    columns = 3,
    value_baseline = 49,
    sprite = "num32",
    signed = "snum24",
    signed_digits = 4,
    minute_digits = 2,
    long_from_minute = 9,
    colon = 3,
    colon_gap = 5,
    colon_rows = { 17, 6 },
    seconds_gap = 13,
    strip_top = 64,
    strip_height = 16,
    legend_top = 86,
    graph_top = 106,
    column_room = 24,
    caption_room = 8,
    auto_on = 1,
}
local HIT_ERROR = { judgeWidthMillis = 150, lineWidth = 2, colorMode = 1, hiterrorMode = 1, emaMode = 1, alpha = 0.1, windowLength = 24, transparent = 1, drawDecay = 1 }
local HIT_ERROR_COLORS = {
    lineColor = theme.color.sub,
    centerColor = theme.color.muted,
    PGColor = theme.judge.pgreat,
    GRColor = theme.judge.great,
    GDColor = theme.judge.good,
    BDColor = theme.judge.bad,
    PRColor = theme.judge.poor,
    emaColor = theme.color.text,
}
local GAUGE = { parts = 50, number_room = 72, animation = 0, range = 3, cycle = 33, kinds = 6, states = 6, whole = "num24", tenth = "num18", whole_digits = 3, baseline = 21, dot = 2, dot_gap = 3 }
local FOOT = {
    columns = 4,
    shade_column = 2,
    value_baseline = 44,
    sprite = "num24",
    size = theme.type.label,
    green = theme.tone.green,
    green_digits = 4,
    cover_digits = 4,
    status_top = 68,
}
local PACE = {
    head = 28,
    legend = 140,
    bar = 40,
    gap = 8,
    swatch = 8,
    label_gap = 16,
    rows = { 16, 84 },
    value_drop = 44,
    sprite = "snum24",
    bars = {
        { type = props.rate.SCORE_RATE, color = theme.color.accent },
        { type = props.rate.BESTSCORE_RATE_NOW, color = theme.color.sub, label = "BEST", diff = props.number.DIFF_HIGHSCORE },
        { type = props.rate.TARGETSCORE_RATE_NOW, color = theme.tone.yellow, label = "TARGET", diff = props.number.DIFF_EXSCORE },
    },
}

local function zero_until_scored(main_state, ref)
    return function()
        local value = main_state.number(ref)
        return ui.has_value(value) and value or 0
    end
end

local function caption_on(s, label, x, baseline, width, opts)
    local size = theme.type.caption
    return parts.caption(s, label, { x = x, top = theme.text_top(baseline, size), w = width }, opts)
end

local function head(s, view, main_state)
    local box = view.head
    local narrow = box.w < HEAD.narrow_below
    parts.difficulty_rect(s, { x = box.x, top = box.top, w = HEAD.bar, h = box.h })
    local x = box.x + HEAD.bar + HEAD.gap
    local right = box.x + box.w
    local baseline = box.top + box.h
    local top = theme.text_top(baseline, HEAD.line)
    local stage_width = narrow and HEAD.narrow_stage_width or HEAD.stage_width
    local stage_box = { x = right - stage_width, top = narrow and top or box.top, w = stage_width }
    local title_end = narrow and right or stage_box.x - HEAD.gap
    local title = { ref = props.text.FULLTITLE, size = HEAD.title, overflow = props.text_overflow.SHRINK }
    local in_course = props.option.MODE_COURSE
    s:text(title, { x = x, top = box.top, w = right - x }, { op = { -in_course }, color = theme.color.text })
    s:text(title, { x = x, top = box.top, w = title_end - x }, { op = { in_course }, color = theme.color.text })
    for _, stage in ipairs(STAGE_LABELS) do
        local label = { text = stage.label, size = narrow and HEAD.line or HEAD.stage, align = "right" }
        s:text(label, stage_box, { op = { stage.op }, color = theme.color.accent })
    end

    local level = { sprite = "num18", ref = props.number.PLAYLEVEL, known = main_state.number(props.number.PLAYLEVEL) }
    parts.by_difficulty(function(index, op, color, name)
        local shown = { op = { op }, color = color }
        s:text({ text = name, size = HEAD.line }, { x = x, top = top, w = HEAD.name_widths[index] }, shown)
        local level_at = { x = x + HEAD.name_widths[index] + HEAD.level_gap, baseline = baseline }
        local placed = s:number(level, level_at, shown)
        local artist_x = level_at.x + placed.span + HEAD.artist_gap
        local artist = { x = artist_x, top = top, w = right - artist_x }
        local credited = { op = narrow and { op, -in_course } or { op }, color = theme.color.sub }
        s:text({ ref = props.text.FULLARTIST, size = HEAD.line, overflow = props.text_overflow.SHRINK }, artist, credited)
    end)
end

local function bga(s, view)
    local box = view.bga
    parts.panel(s, box)
    s:single("bga", {}, box, { op = { props.option.BGA, props.option.BGA_ON } })
    local inner = ui.inset(box, BGA.inset)
    local size = theme.type.caption
    local label = { x = box.x, top = theme.centered_top(box, size), w = box.w }
    local absent = { { ops = { props.option.NO_BGA }, label = "NO BGA" }, { ops = { props.option.BGA, props.option.BGA_OFF }, label = "BGA OFF" } }
    for _, case in ipairs(absent) do
        local pictured = { props.option.STAGEFILE, table.unpack(case.ops) }
        s:reference(props.image.STAGEFILE, inner, { op = pictured, stretch = props.stretch.FIT_INNER })
        local plain = { props.option.NO_STAGEFILE, table.unpack(case.ops) }
        s:text({ text = case.label, size = size, align = "center" }, label, { op = plain, color = theme.color.muted })
    end
end

local function score_rate(s, main_state, x, width, baseline)
    local right = x + width - SCORE.percent_width
    local size = theme.type.caption
    s:text({ text = "%", size = size, align = "right" }, { x = right, top = theme.text_top(baseline, size), w = SCORE.percent_width }, { color = theme.color.sub })
    local fraction = {
        sprite = SCORE.plain,
        value = zero_until_scored(main_state, props.number.SCORE_RATE_AFTERDOT),
        digit = SCORE.fraction_digits,
        padding = props.padding.ZEROS,
        align = "right",
    }
    local placed = s:number(fraction, { x = x, baseline = baseline, w = right - x }, { color = theme.color.text })
    local dot_x = right - placed.span - SCORE.dot_gap - SCORE.dot
    s:rect({ x = dot_x, top = baseline - SCORE.dot, w = SCORE.dot, h = SCORE.dot }, theme.color.text)
    local whole = { sprite = SCORE.plain, value = zero_until_scored(main_state, props.number.SCORE_RATE), digit = SCORE.rate_digits, align = "right" }
    s:number(whole, { x = x, baseline = baseline, w = dot_x - SCORE.dot_gap - x }, { color = theme.color.text })
end

local function score(s, view, main_state)
    local box = view.score
    local total = main_state.number(props.number.TOTALNOTES)
    local most = ui.has_value(total) and total * EX_PER_NOTE or 0
    local ex_digits = ui.digit_count(most)
    local wide = box.w >= SCORE.wide_from
    local width = wide and (box.w - SCORE.column_gap) / 2 or box.w
    local rows = {
        function(x, baseline)
            local spec = { sprite = SCORE.big, value = zero_until_scored(main_state, props.number.EXSCORE), digit = ex_digits, align = "right", fit = width - SCORE.label_width }
            s:number(spec, { x = x, baseline = baseline, w = width }, { color = theme.color.text })
            return "EX SCORE"
        end,
        function(x, baseline)
            score_rate(s, main_state, x, width, baseline)
            return "RATE"
        end,
        function(x, baseline)
            local spec = { sprite = SCORE.plain, ref = props.number.MAXCOMBO_NOW, digit = SCORE.combo_digits, saturate = true, align = "right" }
            s:number(spec, { x = x, baseline = baseline, w = width }, { color = theme.color.text })
            return "MAX COMBO"
        end,
    }
    if view.graph == nil then
        table.insert(rows, function(x, baseline)
            local spec = { sprite = SCORE.signed, ref = props.number.DIFF_EXSCORE, digit = ex_digits + SIGN_PLACES, align = "right", fit = width - SCORE.label_width }
            s:number(spec, { x = x, baseline = baseline, w = width }, { color = theme.color.sub })
            return "TARGET"
        end)
    end
    local per_column = wide and math.ceil(#rows / 2) or #rows
    local pitch = math.min(SCORE.most_pitch, math.floor((box.h - SCORE.first - SCORE.below_last) / (per_column - 1)))
    for index, place in ipairs(rows) do
        local column = math.floor((index - 1) / per_column)
        local x = box.x + column * (width + SCORE.column_gap)
        local baseline = box.top + SCORE.first + (index - 1 - column * per_column) * pitch
        caption_on(s, place(x, baseline), x, baseline, SCORE.label_width)
    end
end

local function counts(s, view, main_state)
    local box = view.counts
    local total = main_state.number(props.number.TOTALNOTES)
    local digits = ui.digit_count(ui.has_value(total) and total or 0)
    local pitch = math.min(COUNTS.most_pitch, math.floor((box.h - COUNTS.head) / #parts.judges))
    local count_end = box.x + math.floor(box.w * COUNTS.count_share)
    local fast_end = box.x + math.floor(box.w * COUNTS.fast_share)
    local slow_end = box.x + box.w
    local size = theme.type.caption
    local split_width = slow_end - fast_end - theme.grid.unit * 2
    parts.caption(s, "JUDGE", { x = box.x, top = box.top, w = box.w })
    s:text({ text = "FAST", size = size, align = "right" }, { x = box.x, top = box.top, w = fast_end - box.x }, { color = theme.color.muted })
    s:text({ text = "SLOW", size = size, align = "right" }, { x = box.x, top = box.top, w = slow_end - box.x }, { color = theme.color.muted })
    for index, row in ipairs(parts.judges) do
        local baseline = box.top + COUNTS.head + COUNTS.rise + (index - 1) * pitch
        s:text({ text = row.name, size = size }, { x = box.x, top = theme.text_top(baseline, size), w = box.w }, { color = row.color })
        local sized = function(sprite, refs, width)
            if row.unbounded then
                local places = math.max(digits, COUNTS.unbounded_digits)
                return { sprite = sprite, value = parts.tally(main_state, refs, places), digit = places, align = "right", fit = width }
            end
            return { sprite = sprite, ref = refs[1], digit = digits, align = "right", fit = width }
        end
        local count_width = count_end - box.x - theme.grid.unit * 10
        s:number(sized(COUNTS.count, row.count, count_width), { x = box.x, baseline = baseline, w = count_end - box.x }, { color = theme.color.text })
        s:number(sized(COUNTS.split, row.fast, split_width), { x = box.x, baseline = baseline, w = fast_end - box.x }, { color = theme.color.sub })
        s:number(sized(COUNTS.split, row.slow, split_width), { x = box.x, baseline = baseline, w = slow_end - box.x }, { color = theme.color.sub })
    end
end

local function time_left(s, main_state, x, baseline)
    local minutes = { sprite = TIMING.sprite, ref = props.number.TIMELEFT_MINUTE, digit = TIMING.minute_digits, saturate = true, align = "right" }
    local cells = s:number_box(minutes, { x = x, baseline = baseline })
    local length = main_state.number(props.number.SONGLENGTH_MINUTE)
    local short = ui.has_value(length) and length < TIMING.long_from_minute
    local left = short and x - cells.w or x
    s:number(minutes, { x = left, baseline = baseline, w = cells.span }, { color = theme.color.text })
    local colon_x = left + cells.span + TIMING.colon_gap
    for _, rise in ipairs(TIMING.colon_rows) do
        s:rect({ x = colon_x, top = baseline - rise, w = TIMING.colon, h = TIMING.colon }, theme.color.sub)
    end
    local seconds = { sprite = TIMING.sprite, ref = props.number.TIMELEFT_SECOND, digit = 2, padding = props.padding.ZEROS }
    s:number(seconds, { x = left + cells.span + TIMING.seconds_gap, baseline = baseline }, { color = theme.color.text })
end

local function timing(s, view, main_state)
    local box = view.timing
    local column = math.floor(box.w / TIMING.columns)
    local baseline = box.top + TIMING.value_baseline
    local at = function(index)
        return box.x + index * column
    end
    parts.caption(s, "BPM", { x = at(0), top = box.top, w = column })
    local fastest = main_state.number(props.number.MAXBPM)
    local bpm = { sprite = TIMING.sprite, ref = props.number.NOWBPM, digit = ui.digit_count(ui.has_value(fastest) and fastest or 0), fit = column - TIMING.column_room }
    s:number(bpm, { x = at(0), baseline = baseline }, { color = theme.color.text })

    parts.caption(s, "TIME LEFT", { x = at(1), top = box.top, w = column })
    time_left(s, main_state, at(1), baseline)

    local adjusting = function()
        return main_state.event_index(props.index.TIMING_AUTO_ADJUST) == TIMING.auto_on and "TIMING AUTO" or "TIMING"
    end
    local heading = { value = adjusting, size = theme.type.caption, overflow = props.text_overflow.SHRINK }
    s:text(heading, { x = at(2), top = box.top, w = column - TIMING.caption_room }, { color = theme.color.muted })
    local offset = { sprite = TIMING.signed, ref = props.number.JUDGETIMING, digit = TIMING.signed_digits, saturate = true }
    s:number(offset, { x = at(2), baseline = baseline }, { color = theme.color.sub })

    local strip = { x = box.x, top = box.top + TIMING.strip_top, w = box.w, h = TIMING.strip_height }
    parts.panel(s, strip)
    local meter = ui.merge(HIT_ERROR, { width = box.w })
    for field, color in pairs(HIT_ERROR_COLORS) do
        meter[field] = theme.hex(color, OPAQUE)
    end
    s:object("hiterrorvisualizer", meter, strip)
end

local function distribution(s, view)
    local box = view.timing
    local legend = view.mode.scratch and parts.judge_graph_legend.beat or parts.judge_graph_legend.popn
    parts.legend(s, box.x + box.w, box.top + TIMING.legend_top, legend)
    local graph = { x = box.x, top = box.top + TIMING.graph_top, w = box.w, h = box.h - TIMING.graph_top }
    parts.panel(s, graph)
    s:object("judgegraph", { type = props.judge_graph.JUDGES, backTexOff = props.back_texture.OFF }, graph)
    s:object("bpmgraph", parts.bpm_graph(), graph)
end

local function gauge(s, view)
    local box = view.gauge
    local part = math.floor((box.w - GAUGE.number_room) / GAUGE.parts)
    local nodes = {}
    for kind = 0, GAUGE.kinds - 1 do
        for state = 0, GAUGE.states - 1 do
            table.insert(nodes, s:define_image(theme.sprite("gauge_" .. kind .. "_" .. state)))
        end
    end
    local bar = { x = box.x, top = box.top, w = part * GAUGE.parts, h = box.h }
    s:single("gauge", { nodes = nodes, parts = GAUGE.parts, type = GAUGE.animation, range = GAUGE.range, cycle = GAUGE.cycle }, bar)

    local baseline = box.top + GAUGE.baseline
    local right = box.x + box.w
    local shades = { { color = theme.color.text }, { color = theme.color.accent, timer = props.timer.GAUGE_MAX_1P } }
    for _, shade in ipairs(shades) do
        local tenth = { sprite = GAUGE.tenth, ref = props.number.GROOVEGAUGE_AFTERDOT, digit = 1, align = "right" }
        local placed = s:number(tenth, { x = box.x, baseline = baseline, w = box.w }, shade)
        local dot_x = right - placed.span - GAUGE.dot_gap - GAUGE.dot
        s:rect({ x = dot_x, top = baseline - GAUGE.dot, w = GAUGE.dot, h = GAUGE.dot }, shade.color, { timer = shade.timer })
        local whole = { sprite = GAUGE.whole, ref = props.number.GROOVEGAUGE, digit = GAUGE.whole_digits, align = "right" }
        s:number(whole, { x = box.x, baseline = baseline, w = dot_x - GAUGE.dot_gap - box.x }, shade)
    end
end

local function foot_column(box, index)
    local width = math.floor(box.w / FOOT.columns)
    return { x = box.x + index * width, top = box.top, w = width }
end

local function foot_speed(s, box, main_state)
    local baseline = box.top + FOOT.value_baseline
    local first = foot_column(box, 0)
    parts.caption(s, "HI-SPEED", first)
    local reading = function()
        local whole = main_state.number(props.number.HISPEED)
        local after = main_state.number(props.number.HISPEED_AFTERDOT)
        if not ui.has_value(whole) or not ui.has_value(after) then
            return ""
        end
        return string.format("%d.%02d", whole, after)
    end
    s:text({ value = reading, size = FOOT.size }, { x = first.x, top = theme.text_top(baseline, FOOT.size), w = first.w }, { color = theme.color.text })

    local second = foot_column(box, 1)
    parts.caption(s, "GREEN", second)
    local green = { sprite = FOOT.sprite, ref = props.number.DURATION_GREEN, digit = FOOT.green_digits, saturate = true }
    s:number(green, { x = second.x, baseline = baseline }, { color = FOOT.green })
end

local function foot_state(s, box, status_top)
    local changing = { op = { props.option.LANECOVER1_CHANGING } }
    local shades = { { label = "COVER", ref = props.number.LANECOVER1 }, { label = "LIFT", ref = props.number.LIFT1 } }
    for index, shade in ipairs(shades) do
        local column = foot_column(box, FOOT.shade_column + index - 1)
        parts.caption(s, shade.label, column, changing)
        local figure = { sprite = FOOT.sprite, ref = shade.ref, digit = FOOT.cover_digits }
        s:number(figure, { x = column.x, baseline = box.top + FOOT.value_baseline }, ui.merge(changing, { color = theme.color.text }))
    end

    local status = { x = box.x, top = box.top + status_top, w = box.w }
    parts.caption(s, "AUTOPLAY", status, { op = { props.option.AUTOPLAY_ON }, color = theme.color.accent })
    parts.caption(s, "REPLAY", status, { op = { props.option.REPLAY_PLAYING }, color = theme.color.accent })
end

local function foot(s, view, main_state)
    local first = view.foot[1]
    local last = view.foot[#view.foot]
    foot_speed(s, first, main_state)
    foot_state(s, last, first == last and FOOT.status_top or 0)
end

local function progress(s, view)
    local bar = s:object("graph", ui.merge(s:sprite_source("bar_v"), { type = props.rate.MUSIC_PROGRESS_BAR, angle = props.graph_angle.UP }))
    for _, box in ipairs(view.progress) do
        s:rect(box, theme.color.raised)
        s:place(bar, box, { color = theme.color.accent })
    end
end

local function pace(s, view, main_state)
    local box = view.graph
    if box == nil then
        return
    end
    local total = main_state.number(props.number.TOTALNOTES)
    local digits = ui.digit_count(ui.has_value(total) and total * EX_PER_NOTE or 0) + SIGN_PLACES
    parts.caption(s, "SCORE GRAPH", { x = box.x, top = box.top, w = box.w })
    local bars_top = box.top + PACE.head
    local bars_height = box.h - PACE.head - PACE.legend
    local legend_top = bars_top + bars_height
    local row = 0
    for index, entry in ipairs(PACE.bars) do
        local track = { x = box.x + (index - 1) * (PACE.bar + PACE.gap), top = bars_top, w = PACE.bar, h = bars_height }
        parts.panel(s, track)
        s:object("graph", ui.merge(s:sprite_source("bar_v"), { type = entry.type, angle = props.graph_angle.UP }), track, { color = entry.color })
        if entry.label then
            row = row + 1
            local top = legend_top + PACE.rows[row]
            local size = theme.type.caption
            s:rect({ x = box.x, top = top, w = PACE.swatch, h = theme.cap_height(size) }, entry.color)
            parts.caption(s, entry.label, { x = box.x + PACE.label_gap, top = top, w = box.w - PACE.label_gap })
            local diff = { sprite = PACE.sprite, ref = entry.diff, digit = digits, fit = box.w }
            s:number(diff, { x = box.x, baseline = top + PACE.value_drop }, { color = theme.color.text })
        end
    end
end

function info.build(s, view, main_state)
    local sections = {
        { layout.PART.SONG_HEADER, head },
        { layout.PART.BGA, bga },
        { layout.PART.SCORE, score },
        { layout.PART.JUDGE_COUNT, counts },
        { layout.PART.TIMING, timing },
        { layout.PART.DISTRIBUTION_GRAPH, distribution },
        { layout.PART.GAUGE, gauge },
        { layout.PART.SPEED_DISPLAY, foot },
        { layout.PART.PROGRESS_BAR, progress },
        { layout.PART.SCORE_GRAPH, pace },
    }
    for _, section in ipairs(sections) do
        local part, build = table.unpack(section)
        s:within(part, function()
            build(s, view, main_state)
        end)
    end
end

return info
