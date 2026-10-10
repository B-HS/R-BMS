local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")

local parts = {}

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

local function with_op(opts, op)
    local ops = { op }
    for _, existing in ipairs((opts or {}).op or {}) do
        table.insert(ops, existing)
    end
    return ui.merge(opts, { op = ops })
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

function parts.fade_from_black(s, duration)
    local cover = { x = 0, top = 0, w = theme.width, h = theme.height }
    s:rect(cover, theme.color.bg, motion.fade_out(0, duration))
end

function parts.fade_to_black(s, start, duration)
    local cover = { x = 0, top = 0, w = theme.width, h = theme.height }
    s:rect(cover, theme.color.bg, motion.fade_in(start, duration, { timer = props.timer.FADEOUT }))
end

return parts
