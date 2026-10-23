local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")
local layout = require("lua.result.layout")
local i18n = require("lua.i18n")

local score = {}

local RANK = {
    column = 0,
    span = 3,
    half_gap = 32,
    letter_size = 160,
    letter_at = 200,
    next_sprite = "num32",
}

local ROW = { score = 1, best = 2, combo = 3, option = 4 }
local FIRST = { column = 3, span = 2 }
local SECOND = { column = 5, span = 2 }
local DIFF_GAP = 16
local TARGET = { label = i18n.t("common_target"), name_offset = 82 }

local RATE = { whole = "num64", fraction = "num44", fraction_digits = 2, dot = 6, dot_gap = 4, unit_gap = 8, unit_size = theme.type.body }

local HIGHER = 1
local LOWER = -1
local FAILED_CLEAR = 1

local REPLAY_STATES = {
    { op = props.option.REPLAYDATA_SAVED, name = i18n.t("result_replay_saved"), color = theme.color.accent },
    { op = props.option.REPLAYDATA, name = i18n.t("result_replay_stored"), color = theme.color.sub },
    { op = props.option.NO_REPLAYDATA, name = i18n.t("result_replay_missing"), color = theme.color.muted },
}

local function half(x, width, index)
    local w = (width - RANK.half_gap) / 2
    return { x = x + index * (w + RANK.half_gap), w = w }
end

local function rank_letters(s, x, width)
    local _, baseline = layout.row(ROW.best)
    local top = theme.text_top(baseline, RANK.letter_size)
    for index, name in ipairs(theme.rank_name) do
        local color = index == #theme.rank_name and theme.color.muted or theme.color.text
        local op = props.option.RESULT_RANK_AAA + index - 1
        local opts = motion.fade_in(RANK.letter_at, theme.time.fade, { timer = props.timer.RESULTGRAPH_BEGIN, op = { op }, color = color })
        s:text({ text = name, size = RANK.letter_size, overflow = props.text_overflow.SHRINK }, { x = x, top = top, w = width }, opts)
    end
end

local function lamp(s, main_state, x, width, course)
    local order = layout.order.rank
    local top, baseline = layout.row(ROW.combo)
    local now = main_state.event_index(props.index.CLEAR)
    local entry = theme.clear[now]
    layout.caption(s, i18n.t("common_clear"), { x = x, top = top, w = width }, order)
    if entry == nil then
        return
    end
    local size = theme.type.heading
    local box = { x = x, top = theme.text_top(baseline, size), w = width }
    local function place(text, color, op)
        local name = { text = text, size = size, overflow = props.text_overflow.SHRINK }
        s:text(name, box, layout.appear(order, { color = color, op = op }))
    end
    if now == theme.graded_clear then
        parts.lamp_names(props.extension.RESULT_ASSIST_GRADE_FIRST, place)
    else
        place(entry.name, theme.clear_text(entry))
    end
    if course then
        return
    end
    if now > FAILED_CLEAR and now > main_state.event_index(props.index.TARGET_CLEAR) then
        layout.record_mark(s, { x = x, top = top, w = width })
    end
end

local function next_rank(s, main_state, box)
    local order = layout.order.rank
    local top, baseline = layout.row(ROW.option)
    local top_rank = props.option.RESULT_RANK_AAA
    layout.caption(s, i18n.t("result_to_next_rank"), { x = box.x, top = top, w = box.w }, order, { op = { -top_rank } })
    layout.caption(s, i18n.t("result_to_max"), { x = box.x, top = top, w = box.w }, order, { op = { top_rank } })
    local short = main_state.number(props.number.DIFF_NEXTRANK)
    if not ui.has_value(short) then
        return
    end
    local spec = { sprite = RANK.next_sprite, ref = props.number.DIFF_NEXTRANK, known = short, fit = box.w }
    s:number(spec, { x = box.x, baseline = baseline }, layout.appear(order, { color = theme.color.text }))
end

local function previous_lamp(s, main_state, box)
    local order = layout.order.rank
    local top, baseline = layout.row(ROW.option)
    local previous = main_state.event_index(props.index.TARGET_CLEAR)
    local entry = theme.clear[previous]
    layout.caption(s, i18n.t("result_previous_clear"), { x = box.x, top = top, w = box.w }, order)
    if entry == nil then
        return
    end
    local size = theme.type.label
    local line = { x = box.x, top = theme.text_top(baseline, size), w = box.w }
    local function place(text, color, op)
        local name = { text = text, size = size, overflow = props.text_overflow.SHRINK }
        s:text(name, line, layout.appear(order, { color = color, op = op }))
    end
    if previous == theme.graded_clear then
        parts.lamp_names(props.extension.RESULT_PREVIOUS_ASSIST_GRADE_FIRST, place)
    else
        place(entry.name, theme.clear_text(entry))
    end
end

local function rank(s, main_state, course)
    local x, width = theme.column(RANK.column, RANK.span)
    layout.caption(s, i18n.t("common_rank"), { x = x, top = layout.row(ROW.score), w = width }, layout.order.rank)
    rank_letters(s, x, width)
    lamp(s, main_state, x, width, course)
    next_rank(s, main_state, half(x, width, 0))
    if not course then
        previous_lamp(s, main_state, half(x, width, 1))
    end
end

local function diff_color(delta, better)
    if delta == 0 then
        return theme.color.sub
    end
    if delta * better > 0 then
        return theme.color.accent
    end
    return theme.tone.orange
end

local function figure(s, main_state, column, row, item)
    local order = layout.order.score
    local x, width = theme.column(column.column, column.span)
    local top, baseline = layout.row(row)
    layout.caption(s, item.label, { x = x, top = top, w = width }, order)
    if item.detail then
        local detail = { text = item.detail.text, size = theme.type.caption, overflow = props.text_overflow.SHRINK }
        local box = { x = x + item.detail.offset, top = top, w = width - item.detail.offset }
        s:text(detail, box, layout.appear(order, { color = theme.color.muted }))
    end
    if item.mark then
        layout.record_mark(s, { x = x, top = top, w = width }, { item.mark })
    end
    local value = main_state.number(item.ref)
    if not ui.has_value(value) then
        return
    end
    local shown = layout.appear(order, { color = item.color or theme.color.text })
    local spec = { sprite = item.sprite, ref = item.ref, known = value }
    local at = { x = x, baseline = baseline }
    local delta = item.diff and main_state.number(item.diff.ref)
    if delta == nil or not ui.has_value(delta) then
        s:number(ui.merge(spec, { fit = width }), at, shown)
        return
    end
    local trailing = { sprite = item.diff.sprite, ref = item.diff.ref, digit = layout.signed_digits(delta) }
    local natural = s:number_box(spec, at).span + s:number_box(trailing, at).span
    local scale = math.min(1, (width - DIFF_GAP) / natural)
    local placed = s:number(ui.merge(spec, { size = s:sprite(spec.sprite).size * scale }), at, shown)
    local after = { x = x + placed.span + DIFF_GAP, baseline = baseline }
    local sized = ui.merge(trailing, { size = s:sprite(trailing.sprite).size * scale })
    s:number(sized, after, layout.appear(order, { color = diff_color(delta, item.diff.better) }))
end

local function rate(s, main_state, column, row)
    local order = layout.order.score
    local x, width = theme.column(column.column, column.span)
    local top, baseline = layout.row(row)
    layout.caption(s, i18n.t("common_rate"), { x = x, top = top, w = width }, order)
    local whole = main_state.number(props.number.SCORE_RATE)
    if not ui.has_value(whole) then
        return
    end
    local shown = layout.appear(order, { color = theme.color.text })
    local placed = s:number({ sprite = RATE.whole, ref = props.number.SCORE_RATE, known = whole }, { x = x, baseline = baseline }, shown)
    local dot_x = x + placed.span + RATE.dot_gap
    s:rect({ x = dot_x, top = baseline - RATE.dot, w = RATE.dot, h = RATE.dot }, theme.color.text, layout.appear(order))
    local fraction = { sprite = RATE.fraction, ref = props.number.SCORE_RATE_AFTERDOT, digit = RATE.fraction_digits, padding = props.padding.ZEROS }
    local after = s:number(fraction, { x = dot_x + RATE.dot + RATE.dot_gap, baseline = baseline }, shown)
    local unit_x = dot_x + RATE.dot + RATE.dot_gap + after.span + RATE.unit_gap
    local unit = { x = unit_x, top = theme.text_top(baseline, RATE.unit_size), w = x + width - unit_x }
    s:text({ text = "%", size = RATE.unit_size }, unit, layout.appear(order, { color = theme.color.sub }))
end

local function word(s, column, row, label, place)
    local order = layout.order.score
    local x, width = theme.column(column.column, column.span)
    local top, baseline = layout.row(row)
    layout.caption(s, label, { x = x, top = top, w = width }, order)
    local size = theme.type.label
    place({ size = size, overflow = props.text_overflow.SHRINK }, { x = x, top = theme.text_top(baseline, size), w = width }, order)
end

local function random_option(s, main_state, column, row)
    local name = theme.random_name[main_state.event_index(props.index.RANDOM_1P)]
    word(s, column, row, "RANDOM", function(spec, box, order)
        if name ~= nil then
            s:text(ui.merge(spec, { text = name }), box, layout.appear(order, { color = theme.color.text }))
        end
    end)
end

local function replay(s)
    word(s, SECOND, ROW.option, i18n.t("common_replay"), function(spec, box, order)
        for _, state in ipairs(REPLAY_STATES) do
            s:text(ui.merge(spec, { text = state.name }), box, layout.appear(order, { op = { state.op }, color = state.color }))
        end
    end)
end

local function target(s, main_state)
    local name = main_state.text(props.text.TARGET_NAME)
    if name == "" then
        return
    end
    figure(s, main_state, SECOND, ROW.best, {
        label = TARGET.label,
        detail = { text = string.upper(name), offset = TARGET.name_offset },
        sprite = "num32",
        ref = props.number.TARGET_SCORE,
        color = theme.color.sub,
        diff = { sprite = "snum32", ref = props.number.DIFF_TARGETSCORE, better = HIGHER },
    })
end

local function best(s, main_state)
    figure(s, main_state, FIRST, ROW.best, {
        label = i18n.t("common_best"),
        sprite = "num32",
        ref = props.number.HIGHSCORE_BEFORE,
        color = theme.color.sub,
        diff = { sprite = "snum32", ref = props.number.DIFF_HIGHSCORE_AFTER, better = HIGHER },
        mark = props.option.UPDATE_SCORE,
    })
end

local function figures(s, main_state, course)
    figure(s, main_state, FIRST, ROW.score, { label = "EX SCORE", sprite = "num64", ref = props.number.SCORE_AFTER })
    rate(s, main_state, SECOND, ROW.score)
    if course then
        figure(s, main_state, FIRST, ROW.best, { label = i18n.t("common_total_notes"), sprite = "num32", ref = props.number.TOTALNOTES, color = theme.color.sub })
        random_option(s, main_state, SECOND, ROW.best)
    else
        best(s, main_state)
        target(s, main_state)
        random_option(s, main_state, FIRST, ROW.option)
        replay(s)
    end
    local combo = { label = "MAX COMBO", sprite = "num44", ref = props.number.MAXCOMBO_AFTER }
    local miss = { label = "MISS COUNT", sprite = "num44", ref = props.number.MISSCOUNT_AFTER }
    if not course then
        combo.diff = { sprite = "snum24", ref = props.number.DIFF_MAXCOMBO, better = HIGHER }
        combo.mark = props.option.UPDATE_MAXCOMBO
        miss.diff = { sprite = "snum24", ref = props.number.DIFF_MISSCOUNT, better = LOWER }
        miss.mark = props.option.UPDATE_MISSCOUNT
    end
    figure(s, main_state, FIRST, ROW.combo, combo)
    figure(s, main_state, SECOND, ROW.combo, miss)
end

function score.build(s, main_state, course)
    rank(s, main_state, course)
    figures(s, main_state, course)
end

return score
