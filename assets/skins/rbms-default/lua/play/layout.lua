local theme = require("lua.theme")
local props = require("lua.props")

local layout = {}

layout.OPTION = {
    FIELD_LEFT = 900,
    FIELD_RIGHT = 901,
    SCRATCH_LEFT = 902,
    SCRATCH_RIGHT = 903,
    DETAIL_OFF = 904,
    DETAIL_FAST_SLOW = 905,
    DETAIL_MILLISECONDS = 906,
    GRAPH_ON = 907,
    GRAPH_OFF = 908,
}

layout.OPTION_NAME = {
    FIELD = "Field Side",
    SCRATCH = "Scratch Side",
    DETAIL = "Judge Detail",
    GRAPH = "Score Graph",
}

layout.FIELD = { top = 0, height = 840, line = 8, keys_top = 848, keys_height = 64, gauge_top = 928, gauge_height = 24, foot_top = 968 }

local LANE = { white = 58, black = 46, scratch = 100, button = 62, gap = 2 }
local NOTE_HEIGHT = { beat = 16, button = 24 }
local SCRATCH_SLOT = 0
local SIDE_GAP = 8
local PROGRESS_WIDTH = 4
local DOUBLE_FOOT_TOP = 928

local BEAT_KEYS = { "white", "black", "white", "black", "white", "black", "white" }
local BUTTON_COLORS = { "white", "yellow", "green", "blue", "red", "blue", "green", "yellow", "white" }
local KIND_COLOR = { white = "white", black = "cyan", scratch = "red" }
local KIND_TINT = { white = theme.color.text, black = theme.color.accent, scratch = theme.tone.red }
local BUTTON_TINT = {
    white = theme.color.text,
    yellow = theme.tone.yellow,
    green = theme.tone.green,
    blue = theme.tone.blue,
    red = theme.tone.red,
}

local MODES = {
    [5] = { type = props.skin_type.PLAY_5KEYS, name = "Play 5 Keys", keys = 5, sides = 1, scratch = true },
    [7] = { type = props.skin_type.PLAY_7KEYS, name = "Play 7 Keys", keys = 7, sides = 1, scratch = true },
    [9] = { type = props.skin_type.PLAY_9KEYS, name = "Play 9 Keys", keys = 9, sides = 1, scratch = false },
    [10] = { type = props.skin_type.PLAY_10KEYS, name = "Play 10 Keys", keys = 5, sides = 2, scratch = true },
    [14] = { type = props.skin_type.PLAY_14KEYS, name = "Play 14 Keys", keys = 7, sides = 2, scratch = true },
}

local SINGLE = {
    field_end = 548,
    graph = { x = 584, top = 120, w = 136, h = 720 },
    head = { x = 744, top = 40, w = 1112, h = 56 },
    bga = { x = 744, top = 120, w = 1112, h = 626 },
    score = { x = 744, top = 770, w = 352, h = 198 },
    counts = { x = 1128, top = 770, w = 352, h = 198 },
    timing = { x = 1512, top = 770, w = 344, h = 198 },
    without_graph = {
        head = { x = 584, top = 40, w = 1272, h = 56 },
        bga = { x = 584, top = 120, w = 1272, h = 626 },
        score = { x = 584, top = 770, w = 408, h = 198 },
        counts = { x = 1016, top = 770, w = 408, h = 198 },
        timing = { x = 1448, top = 770, w = 408, h = 198 },
    },
}

local NINE = {
    field_end = 1247,
    graph = { x = 1280, top = 120, w = 136, h = 720 },
    head = { x = 64, top = 40, w = 576, h = 56 },
    bga = { x = 64, top = 120, w = 576, h = 576 },
    score = { x = 64, top = 720, w = 576, h = 120 },
    counts = { x = 1448, top = 120, w = 408, h = 330 },
    timing = { x = 1448, top = 474, w = 408, h = 366 },
    without_graph = {
        counts = { x = 1280, top = 120, w = 576, h = 330 },
        timing = { x = 1280, top = 474, w = 576, h = 366 },
    },
}

local DOUBLE = {
    field_end = 548,
    head = { x = 580, top = 40, w = 760, h = 56 },
    bga = { x = 580, top = 120, w = 760, h = 428 },
    gauge = { x = 580, top = 572, w = 760, h = 24 },
    score = { x = 580, top = 620, w = 368, h = 220 },
    counts = { x = 972, top = 620, w = 368, h = 220 },
    timing = { x = 580, top = 864, w = 760, h = 152 },
}

function layout.mode(keys)
    local mode = MODES[keys]
    if mode == nil then
        error("no play document for " .. tostring(keys) .. " keys", 2)
    end
    return mode
end

local function side_kinds(mode, scratch_right)
    local kinds = {}
    if not mode.scratch then
        for slot, color in ipairs(BUTTON_COLORS) do
            kinds[slot] = { kind = "button", slot = slot, color = color, tint = BUTTON_TINT[color], order = slot }
        end
        return kinds
    end
    for slot = 1, mode.keys do
        local kind = BEAT_KEYS[slot]
        kinds[slot] = { kind = kind, slot = slot, color = KIND_COLOR[kind], tint = KIND_TINT[kind], order = scratch_right and slot or slot + 1 }
    end
    local scratch = { kind = "scratch", slot = SCRATCH_SLOT, color = KIND_COLOR.scratch, tint = KIND_TINT.scratch, order = scratch_right and mode.keys + 1 or 1 }
    kinds[mode.keys + 1] = scratch
    return kinds
end

local function field_width(mode)
    local width = -LANE.gap
    for _, lane in ipairs(side_kinds(mode, false)) do
        width = width + LANE[lane.kind] + LANE.gap
    end
    return width
end

local function field_of(mode, side, x, scratch_right)
    local field = layout.FIELD
    local kinds = side_kinds(mode, scratch_right)
    local by_order = {}
    for _, lane in ipairs(kinds) do
        by_order[lane.order] = lane
    end
    local cursor = x
    for _, lane in ipairs(by_order) do
        lane.x = cursor
        lane.w = LANE[lane.kind]
        lane.side = side
        lane.note_height = mode.scratch and NOTE_HEIGHT.beat or NOTE_HEIGHT.button
        lane.shaded = lane.kind == "black" or (lane.kind == "button" and lane.order % 2 == 0)
        cursor = cursor + lane.w + LANE.gap
    end
    return { side = side, x = x, top = field.top, w = cursor - LANE.gap - x, h = field.height, lanes = kinds, shown = by_order }
end

local function mirrored(box)
    return { x = theme.width - box.x - box.w, top = box.top, w = box.w, h = box.h }
end

local function under(field, top, height)
    return { x = field.x, top = top, w = field.w, h = height }
end

local function beside(field, on_right)
    local x = on_right and field.x + field.w + SIDE_GAP or field.x - SIDE_GAP - PROGRESS_WIDTH
    return { x = x, top = field.top, w = PROGRESS_WIDTH, h = field.h }
end

local function single(mode, option)
    local base = mode.scratch and SINGLE or NINE
    local flipped = mode.scratch and option[layout.OPTION_NAME.FIELD] == layout.OPTION.FIELD_RIGHT
    local scratch_right = option[layout.OPTION_NAME.SCRATCH] == layout.OPTION.SCRATCH_RIGHT
    local graphed = option[layout.OPTION_NAME.GRAPH] ~= layout.OPTION.GRAPH_OFF
    local width = field_width(mode)
    local place = function(box)
        return flipped and mirrored(box) or box
    end
    local info = function(name)
        return place(not graphed and base.without_graph[name] or base[name])
    end
    local field_box = place({ x = base.field_end - width, top = 0, w = width, h = 0 })
    local field = field_of(mode, 0, field_box.x, scratch_right)
    return {
        fields = { field },
        progress = { beside(field, not flipped) },
        gauge = under(field, layout.FIELD.gauge_top, layout.FIELD.gauge_height),
        foot = { under(field, layout.FIELD.foot_top, theme.height - layout.FIELD.foot_top) },
        graph = graphed and place(base.graph) or nil,
        head = info("head"),
        bga = info("bga"),
        score = info("score"),
        counts = info("counts"),
        timing = info("timing"),
    }
end

local function double(mode)
    local width = field_width(mode)
    local first = field_of(mode, 0, DOUBLE.field_end - width, false)
    local second = field_of(mode, 1, theme.width - DOUBLE.field_end, true)
    return {
        fields = { first, second },
        progress = { beside(first, true), beside(second, false) },
        gauge = DOUBLE.gauge,
        foot = { under(first, DOUBLE_FOOT_TOP, theme.height - DOUBLE_FOOT_TOP), under(second, DOUBLE_FOOT_TOP, theme.height - DOUBLE_FOOT_TOP) },
        head = DOUBLE.head,
        bga = DOUBLE.bga,
        score = DOUBLE.score,
        counts = DOUBLE.counts,
        timing = DOUBLE.timing,
    }
end

function layout.of(mode, option)
    local view = mode.sides == 1 and single(mode, option) or double(mode)
    view.mode = mode
    view.detail = option[layout.OPTION_NAME.DETAIL] or layout.OPTION.DETAIL_FAST_SLOW
    view.lanes = {}
    for _, field in ipairs(view.fields) do
        for _, lane in ipairs(field.lanes) do
            table.insert(view.lanes, lane)
        end
    end
    return view
end

return layout
