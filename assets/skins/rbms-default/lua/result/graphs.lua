local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local parts = require("lua.parts")
local layout = require("lua.result.layout")
local i18n = require("lua.i18n")

local graphs = {}

local CAPTION_GAP = 28
local OPAQUE = 255
local BAND_SHARE = 0.1

local GAUGE = {
    caption_top = 136,
    height = 258,
    name_offset = 80,
    sprite = "num18",
    whole_digits = 3,
    unit_width = 18,
    dot = 2,
    dot_gap = 2,
    unit_gap = 3,
}
local GAUGE_COLORS = {
    assistClearBGColor = theme.color.panel,
    assistAndEasyFailBGColor = theme.color.panel,
    grooveFailBGColor = theme.color.panel,
    grooveClearAndHardBGColor = theme.color.panel,
    exHardBGColor = theme.color.panel,
    hazardBGColor = theme.color.panel,
    borderColor = theme.color.raised,
    assistClearLineColor = theme.lamp.assist,
    assistAndEasyFailLineColor = theme.lamp.easy,
    grooveFailLineColor = theme.lamp.normal,
    borderlineColor = theme.color.accent,
    grooveClearAndHardLineColor = theme.lamp.hard,
    exHardLineColor = theme.lamp.exhard,
    hazardLineColor = theme.tone.red,
}

local TIMELINE = { caption_top = 446, height = 130, tempo_offset = 192, tempo_width = 80 }
local SIDES = { caption_top = 628, height = 130 }
local BPM_LINE = {
    main = theme.color.sub,
    min = theme.color.sub,
    max = theme.color.sub,
    other = theme.color.sub,
    stop = theme.color.sub,
    transition = theme.color.muted,
    width = theme.bpm_line.width,
}

local KIND_LEGEND = parts.judge_graph_legend
local SIDE_LEGEND = {
    beat = {
        { "PG", 25, { 68, 255, 68 } },
        { "FAST", 47, { 0, 136, 255 } },
        { "SLOW", 54, { 255, 136, 0 } },
    },
    popn = {
        { "PG", 25, { 255, 94, 176 } },
        { "FAST", 47, { 0, 136, 255 } },
        { "SLOW", 54, { 255, 136, 0 } },
    },
}

local TIMING = {
    caption_top = 810,
    height = 130,
    stats_offset = 232,
    sprite = "num18",
    fraction_digits = 2,
    label_gap = 8,
    dot = 2,
    dot_gap = 2,
    unit_gap = 6,
    unit_width = 28,
    item_gap = 24,
    labels = {
        fast = { i18n.t("result_average_fast"), 90 },
        slow = { i18n.t("result_average_slow"), 97 },
        even = { i18n.t("result_average"), 38 },
        spread = { "SD", 25 },
    },
}
local TIMING_LINES = { graphColor = theme.color.text, averageColor = theme.color.accent, devColor = theme.color.muted }
local TIMING_BANDS = { "PGColor", "GRColor", "GDColor", "BDColor", "PRColor" }

local STAGES = { caption_gap = 32, pitch = 36, size = theme.type.label, title_offset = 48, gauge_height = 386 }

local function section(right, block, height)
    return { x = right.x, top = block.caption_top + CAPTION_GAP, w = right.w, h = height or block.height }
end

local function appear(opts)
    return layout.appear(layout.order.graphs, opts)
end

local function caption(s, right, block, label)
    parts.caption(s, label, { x = right.x, top = block.caption_top, w = right.w }, appear())
end

local function gauge_known(main_state)
    return function()
        return ui.has_value(main_state.number(props.number.GROOVEGAUGE))
    end
end

local function gauge_value(s, main_state, right, baseline)
    local size = theme.type.caption
    local shown = appear({ color = theme.color.text })
    local known = gauge_known(main_state)
    local unit_x = right.x + right.w - GAUGE.unit_width
    local unit = { x = unit_x, top = theme.text_top(baseline, size), w = GAUGE.unit_width }
    s:text({ text = "%", size = size }, unit, appear({ draw = known, color = theme.color.sub }))
    local tenth = { sprite = GAUGE.sprite, ref = props.number.GROOVEGAUGE_AFTERDOT, digit = 1 }
    local tenth_x = unit_x - GAUGE.unit_gap - s:number_box(tenth, { x = 0, baseline = baseline }).span
    s:number(tenth, { x = tenth_x, baseline = baseline }, shown)
    local dot_x = tenth_x - GAUGE.dot_gap - GAUGE.dot
    s:rect({ x = dot_x, top = baseline - GAUGE.dot, w = GAUGE.dot, h = GAUGE.dot }, theme.color.text, appear({ draw = known }))
    local whole = { sprite = GAUGE.sprite, ref = props.number.GROOVEGAUGE, digit = GAUGE.whole_digits, align = "right" }
    local whole_width = s:number_box(whole, { x = 0, baseline = baseline }).span
    s:number(whole, { x = dot_x - GAUGE.dot_gap - whole_width, w = whole_width, baseline = baseline }, shown)
end

local function gauge(s, main_state, right, height)
    caption(s, right, GAUGE, i18n.t("common_gauge"))
    local size = theme.type.caption
    local name = { x = right.x + GAUGE.name_offset, top = GAUGE.caption_top, w = right.w / 2 }
    s:text({ value = parts.choice(main_state, props.index.GAUGE_1P, theme.gauge_name, ""), size = size }, name, appear({ color = theme.color.text }))
    gauge_value(s, main_state, right, GAUGE.caption_top + theme.cap_height(size))
    local box = section(right, GAUGE, height)
    local colors = {}
    for field, color in pairs(GAUGE_COLORS) do
        colors[field] = theme.hex(color)
    end
    s:object("gaugegraph", colors, box, appear())
    return box
end

local function legend(s, right, top, table_of)
    local edge = right.x + right.w
    parts.legend(s, edge, top, table_of.beat, appear({ op = { -props.option.KEYS_9 } }))
    parts.legend(s, edge, top, table_of.popn, appear({ op = { props.option.KEYS_9 } }))
end

local function timeline(s, right)
    caption(s, right, TIMELINE, i18n.t("result_judges_over_time"))
    legend(s, right, TIMELINE.caption_top, KIND_LEGEND)
    local box = section(right, TIMELINE)
    parts.panel(s, box, appear())
    s:object("judgegraph", { type = props.judge_graph.JUDGES, backTexOff = props.back_texture.OFF }, box, appear())
    local tempo = { op = { props.option.BPMCHANGE }, color = theme.color.sub }
    local tag = { x = right.x + TIMELINE.tempo_offset, top = TIMELINE.caption_top, w = TIMELINE.tempo_width }
    s:text({ text = "+ BPM", size = theme.type.caption }, tag, appear(tempo))
    s:object("bpmgraph", parts.bpm_graph(BPM_LINE), box, appear({ op = tempo.op }))
end

local function sides(s, right)
    caption(s, right, SIDES, i18n.t("result_fast_slow_over_time"))
    legend(s, right, SIDES.caption_top, SIDE_LEGEND)
    local box = section(right, SIDES)
    parts.panel(s, box, appear())
    s:object("judgegraph", { type = props.judge_graph.FAST_SLOW, backTexOff = props.back_texture.OFF }, box, appear())
end

local function decimal(s, x, baseline, whole_ref, fraction_ref, whole_value)
    local shown = appear({ color = theme.color.text })
    local whole = s:number({ sprite = TIMING.sprite, ref = whole_ref, known = whole_value }, { x = x, baseline = baseline }, shown)
    local dot_x = x + whole.span + TIMING.dot_gap
    s:rect({ x = dot_x, top = baseline - TIMING.dot, w = TIMING.dot, h = TIMING.dot }, theme.color.text, appear())
    local fraction = { sprite = TIMING.sprite, ref = fraction_ref, digit = TIMING.fraction_digits, padding = props.padding.ZEROS }
    local after = s:number(fraction, { x = dot_x + TIMING.dot + TIMING.dot_gap, baseline = baseline }, shown)
    local unit_x = dot_x + TIMING.dot + TIMING.dot_gap + after.span + TIMING.unit_gap
    local size = theme.type.caption
    s:text({ text = "MS", size = size }, { x = unit_x, top = theme.text_top(baseline, size), w = TIMING.unit_width }, appear({ color = theme.color.sub }))
    return unit_x + TIMING.unit_width
end

local function labelled_decimal(s, x, baseline, label, whole_ref, fraction_ref, whole_value)
    local size = theme.type.caption
    s:text({ text = label[1], size = size }, { x = x, top = theme.text_top(baseline, size), w = label[2] }, appear({ color = theme.color.sub }))
    return decimal(s, x + label[2] + TIMING.label_gap, baseline, whole_ref, fraction_ref, whole_value)
end

local function timing_stats(s, main_state, right)
    local size = theme.type.caption
    local baseline = TIMING.caption_top + theme.cap_height(size)
    local x = right.x + TIMING.stats_offset
    local average = main_state.number(props.number.AVERAGE_TIMING)
    local average_fraction = main_state.number(props.number.AVERAGE_TIMING_AFTERDOT)
    local spread = main_state.number(props.number.STDDEV_TIMING)
    if not ui.has_value(average) or not ui.has_value(spread) or spread < 0 then
        s:text({ text = i18n.t("result_no_judged_notes"), size = size, overflow = props.text_overflow.SHRINK }, { x = x, top = TIMING.caption_top, w = right.x + right.w - x }, appear({ color = theme.color.sub }))
        return
    end
    local label = TIMING.labels.even
    if average > 0 or average_fraction > 0 then
        label = TIMING.labels.fast
    elseif average < 0 or average_fraction < 0 then
        label = TIMING.labels.slow
    end
    local after = labelled_decimal(s, x, baseline, label, props.number.AVERAGE_TIMING, props.number.AVERAGE_TIMING_AFTERDOT, average)
    labelled_decimal(s, after + TIMING.item_gap, baseline, TIMING.labels.spread, props.number.STDDEV_TIMING, props.number.STDDEV_TIMING_AFTERDOT, spread)
end

local function timing(s, main_state, right)
    caption(s, right, TIMING, i18n.t("result_timing_distribution"))
    timing_stats(s, main_state, right)
    local box = section(right, TIMING)
    parts.panel(s, box, appear())
    local colors = { drawAverage = 1, drawDev = 1 }
    for field, color in pairs(TIMING_LINES) do
        colors[field] = theme.hex(color, OPAQUE)
    end
    for judge, field in ipairs(TIMING_BANDS) do
        colors[field] = theme.hex(theme.mix(KIND_LEGEND.beat[judge][3], theme.color.panel, BAND_SHARE), OPAQUE)
    end
    s:object("timingdistributiongraph", colors, box, appear())
end

local function stages(s, right, top, titles)
    if #titles == 0 then
        return
    end
    parts.caption(s, i18n.t("common_stages"), { x = right.x, top = top, w = right.w }, appear())
    local size = STAGES.size
    for index, ref in ipairs(titles) do
        local baseline = top + CAPTION_GAP + (index - 1) * STAGES.pitch + theme.cap_height(size)
        local number = { x = right.x, top = theme.text_top(baseline, size), w = STAGES.title_offset }
        s:text({ text = tostring(index), size = size }, number, appear({ color = theme.color.muted }))
        local title = { x = right.x + STAGES.title_offset, top = theme.text_top(baseline, size), w = right.w - STAGES.title_offset }
        s:text({ ref = ref, size = size, overflow = props.text_overflow.SHRINK }, title, appear({ color = theme.color.text }))
    end
end

function graphs.build(s, main_state, course)
    local right = layout.right()
    if course then
        local titles = layout.stage_titles(main_state)
        local height = STAGES.gauge_height + (layout.stage_limit() - #titles) * STAGES.pitch
        local box = gauge(s, main_state, right, height)
        stages(s, right, box.top + box.h + STAGES.caption_gap, titles)
        return
    end
    gauge(s, main_state, right)
    timeline(s, right)
    sides(s, right)
    timing(s, main_state, right)
end

return graphs
