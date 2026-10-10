local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")

local layout = {}

local LEFT = { column = 0, span = 7 }
local RIGHT = { column = 7, span = 5 }
local SECTION_STEP = 100
local SECTION_GAP = 32
local LABEL_GAP = 10
local HEAD = { top = 136, height = 168 }
local ROWS = { gap = 26, sizes = { 64, 32, 44, 32 } }
local DIVIDER = { above = 15, below = 16 }
local MARK = { text = "NEW RECORD", at = 400 }
local STAGE_TITLE_LIMIT = 10

layout.order = { head = 0, rank = 1, score = 2, judges = 3, graphs = 4 }

local function column_box(column)
    local x, width = theme.column(column.column, column.span)
    return { x = x, w = width }
end

function layout.left()
    return column_box(LEFT)
end

function layout.right()
    return column_box(RIGHT)
end

function layout.appear(order, opts)
    return motion.fade_in(order * SECTION_STEP, theme.time.fade, opts)
end

function layout.value_baseline(label_top, size)
    return label_top + theme.type.caption + LABEL_GAP + theme.cap_height(size)
end

function layout.head()
    return { top = HEAD.top, h = HEAD.height }
end

function layout.row(index)
    local top = HEAD.top + HEAD.height + SECTION_GAP
    for earlier = 1, index - 1 do
        top = layout.value_baseline(top, ROWS.sizes[earlier]) + ROWS.gap
    end
    return top, layout.value_baseline(top, ROWS.sizes[index])
end

function layout.divider_top()
    local _, baseline = layout.row(#ROWS.sizes)
    return baseline + DIVIDER.above
end

function layout.table_top()
    return layout.divider_top() + theme.grid.rule + DIVIDER.below
end

function layout.signed_digits(value)
    return ui.digit_count(value) + 1
end

function layout.record_mark(s, box, conditions)
    local ops = { props.option.ENABLE_SAVE_SCORE }
    for _, op in ipairs(conditions or {}) do
        table.insert(ops, op)
    end
    local opts = motion.fade_in(MARK.at, theme.time.fade, { timer = props.timer.RESULT_UPDATESCORE, op = ops, color = theme.color.accent })
    s:text({ text = MARK.text, size = theme.type.caption, align = "right" }, box, opts)
end

function layout.stage_titles(main_state)
    local last = 0
    for index = 1, STAGE_TITLE_LIMIT do
        if main_state.text(props.text.COURSE1_TITLE + index - 1) ~= "" then
            last = index
        end
    end
    local titles = {}
    for index = 1, last do
        titles[index] = props.text.COURSE1_TITLE + index - 1
    end
    return titles
end

function layout.stage_limit()
    return STAGE_TITLE_LIMIT
end

function layout.caption(s, label, box, order, opts)
    return parts.caption(s, label, box, layout.appear(order, opts))
end

return layout
