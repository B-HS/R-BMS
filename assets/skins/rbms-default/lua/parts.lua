local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")

local parts = {}

local DIGIT_BASE = 10

local KEY_MODE_LABELS = {
    { op = props.option.KEYS_5, label = "5 KEYS" },
    { op = props.option.KEYS_7, label = "7 KEYS" },
    { op = props.option.KEYS_9, label = "9 KEYS" },
    { op = props.option.KEYS_10, label = "10 KEYS" },
    { op = props.option.KEYS_14, label = "14 KEYS" },
}

local JUDGE_RANK_LABELS = {
    { op = props.option.JUDGE_VERYHARD, label = "VERY HARD" },
    { op = props.option.JUDGE_HARD, label = "HARD" },
    { op = props.option.JUDGE_NORMAL, label = "NORMAL" },
    { op = props.option.JUDGE_EASY, label = "EASY" },
    { op = props.option.JUDGE_VERYEASY, label = "VERY EASY" },
}

parts.judges = {
    { name = "PGREAT", color = theme.judge.pgreat, count = { props.number.PERFECT }, fast = { props.number.EARLY_PERFECT }, slow = { props.number.LATE_PERFECT } },
    { name = "GREAT", color = theme.judge.great, count = { props.number.GREAT }, fast = { props.number.EARLY_GREAT }, slow = { props.number.LATE_GREAT } },
    { name = "GOOD", color = theme.judge.good, count = { props.number.GOOD }, fast = { props.number.EARLY_GOOD }, slow = { props.number.LATE_GOOD } },
    { name = "BAD", color = theme.judge.bad, count = { props.number.BAD }, fast = { props.number.EARLY_BAD }, slow = { props.number.LATE_BAD } },
    {
        name = "POOR",
        color = theme.judge.poor,
        count = { props.number.POOR_PLUS_MISS },
        fast = { props.number.EARLY_POOR, props.number.EARLY_MISS },
        slow = { props.number.LATE_POOR, props.number.LATE_MISS },
        unbounded = true,
    },
}

parts.combo_break = { name = "COMBO BREAK", count = { props.number.COMBOBREAK } }
parts.empty_poor = { name = "EMPTY POOR", count = { props.number.MISS } }

parts.judge_graph_legend = {
    beat = {
        { "PG", 25, { 0, 136, 255 } },
        { "GR", 25, { 0, 255, 136 } },
        { "GD", 27, { 255, 255, 0 } },
        { "BD", 26, { 255, 136, 0 } },
        { "PR", 24, { 255, 0, 0 } },
    },
    popn = {
        { "PG", 25, { 255, 94, 176 } },
        { "GR", 25, { 255, 190, 50 } },
        { "GD", 27, { 220, 70, 60 } },
        { "BD", 26, { 108, 198, 255 } },
        { "PR", 24, { 108, 198, 255 } },
    },
}

local LEGEND = { chip = 10, chip_gap = 6, chip_drop = 2, item_gap = 20 }

parts.stage_image = {
    name = "Stage Image",
    show = props.option.CUSTOM_FIRST,
    hide = props.option.CUSTOM_FIRST + 1,
    missing = "NO STAGE IMAGE",
}

local function with_op(opts, op)
    local ops = { op }
    for _, existing in ipairs((opts or {}).op or {}) do
        table.insert(ops, existing)
    end
    return ui.merge(opts, { op = ops })
end

function parts.sum(main_state, refs)
    local total = 0
    for _, ref in ipairs(refs) do
        local value = main_state.number(ref)
        if not ui.has_value(value) then
            return value
        end
        total = total + value
    end
    return total
end

function parts.tally(main_state, refs, places)
    local most = DIGIT_BASE ^ places - 1
    return function()
        local total = parts.sum(main_state, refs)
        if not ui.has_value(total) then
            return total
        end
        return math.min(most, total)
    end
end

function parts.backdrop(s)
    s:rect({ x = 0, top = 0, w = theme.width, h = theme.height }, theme.color.bg)
end

function parts.rule(s, top, opts)
    local grid = theme.grid
    return s:rect({ x = grid.margin, top = top, w = theme.content_width(), h = grid.rule }, theme.color.line, opts)
end

function parts.frame(s, caption)
    local grid = theme.grid
    parts.rule(s, grid.header_rule)
    parts.rule(s, grid.footer_rule)
    if caption then
        local size = theme.type.caption
        local top = grid.header_rule - grid.unit * 2 - size
        s:text({ text = caption, size = size }, { x = grid.margin, top = top, w = theme.content_width() }, { color = theme.color.muted })
    end
end

function parts.panel(s, box, opts)
    return s:rect(box, theme.color.panel, opts)
end

function parts.caption(s, label, box, opts)
    return s:text({ text = label, size = theme.type.caption }, box, ui.merge(opts, { color = (opts or {}).color or theme.color.muted }))
end

function parts.by_difficulty(place)
    for index = 0, props.option.DIFFICULTY_LAST - props.option.DIFFICULTY_FIRST do
        place(index, props.option.DIFFICULTY_FIRST + index, theme.difficulty[index], theme.difficulty_name[index])
    end
end

function parts.difficulty_rect(s, box, opts)
    parts.by_difficulty(function(_, op, color)
        s:rect(box, color, with_op(opts, op))
    end)
end

function parts.difficulty_text(s, spec, box, opts)
    parts.by_difficulty(function(_, op, color, name)
        s:text(ui.merge(spec, { text = name }), box, ui.merge(with_op(opts, op), { color = color }))
    end)
end

local function labelled(s, labels, spec, box, opts)
    for _, entry in ipairs(labels) do
        s:text(ui.merge(spec, { text = entry.label }), box, with_op(opts, entry.op))
    end
end

function parts.key_mode_text(s, spec, box, opts)
    labelled(s, KEY_MODE_LABELS, spec, box, opts)
end

function parts.judge_rank_text(s, spec, box, opts)
    labelled(s, JUDGE_RANK_LABELS, spec, box, opts)
end

function parts.shown_or_hidden(name, show, hide)
    return { name = name, item = { { name = "Show", op = show }, { name = "Hide", op = hide } }, def = "Show" }
end

function parts.stage_image_property()
    local option = parts.stage_image
    return parts.shown_or_hidden(option.name, option.show, option.hide)
end

function parts.stage_image_shown()
    return skin_config.option[parts.stage_image.name] ~= parts.stage_image.hide
end

function parts.choice(main_state, id, names, missing)
    return function()
        return names[main_state.event_index(id)] or missing
    end
end

function parts.assist_grade_drawn(main_state, first, grade)
    if grade > 0 then
        return function()
            return main_state.option(first + grade)
        end
    end
    return function()
        for other = 1, theme.assist_grade_last do
            if main_state.option(first + other) then
                return false
            end
        end
        return true
    end
end

function parts.lamp_names(main_state, first, place)
    for grade = 0, theme.assist_grade_last do
        local entry = theme.assist_grade[grade]
        place(entry.name, entry.color, parts.assist_grade_drawn(main_state, first, grade))
    end
end

function parts.legend(s, right_edge, top, items, opts)
    local size = theme.type.caption
    local x = right_edge
    for index = #items, 1, -1 do
        local label, width, color = table.unpack(items[index])
        local label_x = x - width
        local chip = { x = label_x - LEGEND.chip_gap - LEGEND.chip, top = top + LEGEND.chip_drop, w = LEGEND.chip, h = LEGEND.chip }
        s:text({ text = label, size = size }, { x = label_x, top = top, w = width }, ui.merge(opts, { color = theme.color.muted }))
        s:rect(chip, color, opts)
        x = chip.x - LEGEND.item_gap
    end
end

function parts.bpm_graph(lines)
    local line = lines or theme.bpm_line
    return {
        lineWidth = line.width,
        mainBPMColor = theme.hex(line.main),
        minBPMColor = theme.hex(line.min),
        maxBPMColor = theme.hex(line.max),
        otherBPMColor = theme.hex(line.other),
        stopLineColor = theme.hex(line.stop),
        transitionLineColor = theme.hex(line.transition),
    }
end

function parts.fade_from_black(s, duration)
    local cover = { x = 0, top = 0, w = theme.width, h = theme.height }
    s:rect(cover, theme.color.bg, motion.fade_out(0, duration))
end

function parts.fade_to_black(s, start, duration)
    local cover = { x = 0, top = 0, w = theme.width, h = theme.height }
    s:rect(cover, theme.color.bg, motion.fade_in(start, duration, { timer = props.timer.FADEOUT }))
end

return parts
