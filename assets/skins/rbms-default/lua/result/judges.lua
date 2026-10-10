local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")
local layout = require("lua.result.layout")

local judges = {}

local TABLE = {
    first_row = 30,
    pitch = 38,
    baseline = 26,
    name_width = 136,
    sprite = "num24",
    count = { offset = 144, w = 120 },
    fast = { offset = 280, w = 96 },
    slow = { offset = 392, w = 96 },
    bar = { offset = 520, top = 14, h = 10, sprite = "bar", least = 2, grow_at = 500, grow = 500 },
    total = { rule = 260, baseline = 283, label = "TOTAL EXCL. PGREAT" },
    notes = { label = "TOTAL NOTES", sprite = "num18", gap = 8, label_width = 160 },
}

local function cell(left, column)
    return { x = left.x + column.offset, w = column.w }
end

local function divider(s, left)
    local rule = { x = left.x, top = layout.divider_top(), w = left.w, h = theme.grid.rule }
    s:rect(rule, theme.color.line, layout.appear(layout.order.judges))
end

local function note_total(s, main_state, left, top)
    local order = layout.order.judges
    local notes = main_state.number(props.number.TOTALNOTES)
    if not ui.has_value(notes) then
        return
    end
    local size = theme.type.caption
    local spec = { sprite = TABLE.notes.sprite, ref = props.number.TOTALNOTES, known = notes, align = "right" }
    local box = { x = left.x, w = left.w, baseline = top + theme.cap_height(size) }
    local number_x = left.x + left.w - s:number_box(spec, box).span
    s:number(spec, box, layout.appear(order, { color = theme.color.sub }))
    local label = { x = number_x - TABLE.notes.gap - TABLE.notes.label_width, top = top, w = TABLE.notes.label_width }
    s:text({ text = TABLE.notes.label, size = size, align = "right" }, label, layout.appear(order, { color = theme.color.muted }))
end

local function heading(s, main_state, left, top, course)
    local order = layout.order.judges
    layout.caption(s, "JUDGE", { x = left.x, top = top, w = TABLE.name_width }, order)
    for _, column in ipairs({ { "COUNT", TABLE.count }, { "FAST", TABLE.fast }, { "SLOW", TABLE.slow } }) do
        local box = cell(left, column[2])
        local opts = layout.appear(order, { color = theme.color.muted })
        s:text({ text = column[1], size = theme.type.caption, align = "right" }, { x = box.x, top = top, w = box.w }, opts)
    end
    local bar_x = left.x + TABLE.bar.offset
    layout.caption(s, "SHARE OF NOTES", { x = bar_x, top = top, w = left.x + left.w - bar_x }, order)
    if not course then
        note_total(s, main_state, left, top)
    end
end

local function count(s, main_state, ref, box, baseline, color)
    local value = main_state.number(ref)
    if not ui.has_value(value) then
        return
    end
    local spec = { sprite = TABLE.sprite, ref = ref, known = value, fit = box.w, align = "right" }
    s:number(spec, { x = box.x, w = box.w, baseline = baseline }, layout.appear(layout.order.judges, { color = color }))
end

local function share_of(main_state, ref, least)
    return function()
        local notes = main_state.number(props.number.TOTALNOTES)
        local judged = main_state.number(ref)
        if not ui.has_value(notes) or not ui.has_value(judged) or notes <= 0 or judged <= 0 then
            return 0
        end
        return math.max(least, math.min(1, judged / notes))
    end
end

local function bar(s, main_state, left, top, row)
    local source = s:sprite_source(TABLE.bar.sprite)
    local x = left.x + TABLE.bar.offset
    local box = { x = x, top = top + TABLE.bar.top, w = left.x + left.w - x, h = TABLE.bar.h }
    s:rect(box, theme.color.panel, layout.appear(layout.order.judges))
    local graph = ui.merge(source, { angle = props.graph_angle.RIGHT, value = share_of(main_state, row.count, TABLE.bar.least / source.w) })
    local grow = motion.grow_right(TABLE.bar.grow_at, TABLE.bar.grow, { timer = props.timer.RESULTGRAPH_BEGIN, color = row.color })
    s:object("graph", graph, box, grow)
end

local function row(s, main_state, left, top, entry)
    local order = layout.order.judges
    local baseline = top + TABLE.baseline
    local size = theme.type.label
    local name = { x = left.x, top = theme.text_top(baseline, size), w = TABLE.name_width }
    s:text({ text = entry.name, size = size, overflow = props.text_overflow.SHRINK }, name, layout.appear(order, { color = entry.color }))
    count(s, main_state, entry.count, cell(left, TABLE.count), baseline, theme.color.text)
    count(s, main_state, entry.fast, cell(left, TABLE.fast), baseline, theme.color.sub)
    count(s, main_state, entry.slow, cell(left, TABLE.slow), baseline, theme.color.sub)
    bar(s, main_state, left, top, entry)
end

local function totals(s, main_state, left, top)
    local order = layout.order.judges
    local fast = cell(left, TABLE.fast)
    local slow = cell(left, TABLE.slow)
    local baseline = top + TABLE.total.baseline
    local rule = { x = fast.x, top = top + TABLE.total.rule, w = slow.x + slow.w - fast.x, h = theme.grid.rule }
    s:rect(rule, theme.color.line, layout.appear(order))
    local size = theme.type.caption
    local label = { x = left.x, top = theme.text_top(baseline, size), w = fast.x - left.x }
    s:text({ text = TABLE.total.label, size = size, overflow = props.text_overflow.SHRINK }, label, layout.appear(order, { color = theme.color.muted }))
    count(s, main_state, props.number.TOTAL_EARLY, fast, baseline, theme.color.text)
    count(s, main_state, props.number.TOTAL_LATE, slow, baseline, theme.color.text)
end

function judges.build(s, main_state, course)
    local left = layout.left()
    local top = layout.table_top()
    divider(s, left)
    heading(s, main_state, left, top, course)
    for index, entry in ipairs(parts.judges) do
        row(s, main_state, left, top + TABLE.first_row + (index - 1) * TABLE.pitch, entry)
    end
    totals(s, main_state, left, top)
end

return judges
