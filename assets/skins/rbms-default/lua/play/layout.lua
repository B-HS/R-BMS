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
    LANES_NORMAL = 909,
    LANES_WIDE = 910,
    DIFFERENCE_TARGET = 911,
    DIFFERENCE_BEST = 912,
    DIFFERENCE_OFF = 913,
    JUDGE_SHOW = 914,
    JUDGE_HIDE = 915,
}

layout.OPTION_NAME = {
    FIELD = "Field Side",
    SCRATCH = "Scratch Side",
    DETAIL = "Judge Detail",
    GRAPH = "Score Graph",
    LANES = "Lane Width",
    DIFFERENCE = "Score Difference",
    JUDGE = "Judge Display",
}

layout.JUDGE_POSITION = { name = "Judge Position", id = 100 }

layout.PART = {
    LANE_BACKGROUND = 101,
    LANE_DIVIDER = 102,
    JUDGE_LINE = 103,
    KEY_BEAM = 104,
    KEY_DISPLAY = 105,
    BOMB = 106,
    NOTE = 107,
    BAR_LINE = 108,
    LANE_COVER = 109,
    GAUGE = 110,
    PROGRESS_BAR = 111,
    SONG_HEADER = 112,
    BGA = 113,
    SCORE_GRAPH = 114,
    SCORE = 115,
    JUDGE_COUNT = 116,
    TIMING = 117,
    DISTRIBUTION_GRAPH = 118,
    SPEED_DISPLAY = 119,
}

layout.PART_ROWS = {
    { name = "Lane Background Opacity", id = layout.PART.LANE_BACKGROUND },
    { name = "Lane Divider Opacity", id = layout.PART.LANE_DIVIDER },
    { name = "Judge Line Opacity", id = layout.PART.JUDGE_LINE },
    { name = "Key Beam Opacity", id = layout.PART.KEY_BEAM },
    { name = "Key Display Opacity", id = layout.PART.KEY_DISPLAY },
    { name = "Bomb Opacity", id = layout.PART.BOMB },
    { name = "Note Opacity", id = layout.PART.NOTE },
    { name = "Bar Line Opacity", id = layout.PART.BAR_LINE },
    { name = "Lane Cover Opacity", id = layout.PART.LANE_COVER },
    { name = "Gauge Opacity", id = layout.PART.GAUGE },
    { name = "Progress Bar Opacity", id = layout.PART.PROGRESS_BAR },
    { name = "Song Header Opacity", id = layout.PART.SONG_HEADER },
    { name = "BGA Opacity", id = layout.PART.BGA },
    { name = "Score Graph Opacity", id = layout.PART.SCORE_GRAPH, single_only = true },
    { name = "Score Opacity", id = layout.PART.SCORE },
    { name = "Judge Count Opacity", id = layout.PART.JUDGE_COUNT },
    { name = "Timing Opacity", id = layout.PART.TIMING },
    { name = "Distribution Graph Opacity", id = layout.PART.DISTRIBUTION_GRAPH },
    { name = "Speed Display Opacity", id = layout.PART.SPEED_DISPLAY },
}

layout.FIELD = { top = 0, height = 840, line = 8, keys_top = 848, keys_height = 64, gauge_top = 928, gauge_height = 24, foot_top = 968 }

local LANE = { white = 58, black = 46, scratch = 100, button = 62 }
local LANE_GAP = 2
local LANE_SCALE = { normal = 1, wide = 1.5 }
local LANE_WIDTH = { [layout.OPTION.LANES_NORMAL] = "normal", [layout.OPTION.LANES_WIDE] = "wide" }
local NOTE_HEIGHT = { beat = 16, button = 24 }
local SCRATCH_SLOT = 0
local SIDE_GAP = 8
local PROGRESS_WIDTH = 4

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
    normal = {
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
    },
    wide = {
        field_end = 783,
        graph = { x = 820, top = 120, w = 136, h = 720 },
        head = { x = 980, top = 40, w = 876, h = 56 },
        bga = { x = 980, top = 120, w = 876, h = 450 },
        score = { x = 980, top = 594, w = 426, h = 198 },
        counts = { x = 1430, top = 594, w = 426, h = 198 },
        timing = { x = 980, top = 816, w = 876, h = 152 },
        without_graph = {
            head = { x = 820, top = 40, w = 1036, h = 56 },
            bga = { x = 820, top = 120, w = 1036, h = 626 },
            score = { x = 820, top = 770, w = 328, h = 198 },
            counts = { x = 1172, top = 770, w = 328, h = 198 },
            timing = { x = 1524, top = 770, w = 332, h = 198 },
        },
    },
}

local NINE = {
    normal = {
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
    },
    wide = {
        field_end = 1387,
        graph = { x = 1420, top = 120, w = 136, h = 720 },
        head = { x = 64, top = 40, w = 440, h = 56 },
        bga = { x = 64, top = 120, w = 440, h = 440 },
        timing = { x = 64, top = 584, w = 440, h = 256 },
        counts = { x = 1588, top = 120, w = 268, h = 330 },
        score = { x = 1588, top = 474, w = 268, h = 366 },
        without_graph = {
            counts = { x = 1420, top = 120, w = 436, h = 330 },
            score = { x = 1420, top = 474, w = 436, h = 366 },
        },
    },
}

local DOUBLE = {
    normal = {
        field_end = 548,
        foot_top = 928,
        head = { x = 580, top = 40, w = 760, h = 56 },
        bga = { x = 580, top = 120, w = 760, h = 428 },
        gauge = { x = 580, top = 572, w = 760, h = 24 },
        score = { x = 580, top = 620, w = 368, h = 220 },
        counts = { x = 972, top = 620, w = 368, h = 220 },
        timing = { x = 580, top = 864, w = 760, h = 152 },
    },
    wide = {
        field_end = 751,
        foot_top = layout.FIELD.foot_top,
        head = { x = 784, top = 40, w = 352, h = 56 },
        bga = { x = 784, top = 120, w = 352, h = 264 },
        score = { x = 784, top = 408, w = 352, h = 198 },
        counts = { x = 784, top = 630, w = 352, h = 206 },
        timing = { x = 784, top = 860, w = 352, h = 156 },
    },
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

local function round(value)
    return math.floor(value + 0.5)
end

local function lane_width(kind, scale)
    return round(LANE[kind] * scale)
end

local function field_width(mode, scale)
    local width = -LANE_GAP
    for _, lane in ipairs(side_kinds(mode, false)) do
        width = width + lane_width(lane.kind, scale) + LANE_GAP
    end
    return width
end

local function field_of(mode, side, x, scratch_right, scale)
    local field = layout.FIELD
    local kinds = side_kinds(mode, scratch_right)
    local by_order = {}
    for _, lane in ipairs(kinds) do
        by_order[lane.order] = lane
    end
    local cursor = x
    for _, lane in ipairs(by_order) do
        lane.x = cursor
        lane.w = lane_width(lane.kind, scale)
        lane.side = side
        lane.note_height = mode.scratch and NOTE_HEIGHT.beat or NOTE_HEIGHT.button
        lane.shaded = lane.kind == "black" or (lane.kind == "button" and lane.order % 2 == 0)
        cursor = cursor + lane.w + LANE_GAP
    end
    return { side = side, x = x, top = field.top, w = cursor - LANE_GAP - x, h = field.height, lanes = kinds, shown = by_order }
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

local function single(mode, option, lanes)
    local base = (mode.scratch and SINGLE or NINE)[lanes]
    local scale = LANE_SCALE[lanes]
    local flipped = mode.scratch and option[layout.OPTION_NAME.FIELD] == layout.OPTION.FIELD_RIGHT
    local scratch_right = option[layout.OPTION_NAME.SCRATCH] == layout.OPTION.SCRATCH_RIGHT
    local graphed = option[layout.OPTION_NAME.GRAPH] ~= layout.OPTION.GRAPH_OFF
    local width = field_width(mode, scale)
    local place = function(box)
        return flipped and mirrored(box) or box
    end
    local info = function(name)
        return place(not graphed and base.without_graph[name] or base[name])
    end
    local field_box = place({ x = base.field_end - width, top = 0, w = width, h = 0 })
    local field = field_of(mode, 0, field_box.x, scratch_right, scale)
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

local function double(mode, lanes)
    local base = DOUBLE[lanes]
    local scale = LANE_SCALE[lanes]
    local width = field_width(mode, scale)
    local first = field_of(mode, 0, base.field_end - width, false, scale)
    local second = field_of(mode, 1, theme.width - base.field_end, true, scale)
    local foot_height = theme.height - base.foot_top
    return {
        fields = { first, second },
        progress = { beside(first, true), beside(second, false) },
        gauge = base.gauge or under(first, layout.FIELD.gauge_top, layout.FIELD.gauge_height),
        foot = { under(first, base.foot_top, foot_height), under(second, base.foot_top, foot_height) },
        head = base.head,
        bga = base.bga,
        score = base.score,
        counts = base.counts,
        timing = base.timing,
    }
end

function layout.of(mode, option)
    local lanes = LANE_WIDTH[option[layout.OPTION_NAME.LANES]] or LANE_WIDTH[layout.OPTION.LANES_NORMAL]
    local view = mode.sides == 1 and single(mode, option, lanes) or double(mode, lanes)
    view.mode = mode
    view.detail = option[layout.OPTION_NAME.DETAIL] or layout.OPTION.DETAIL_FAST_SLOW
    view.difference = option[layout.OPTION_NAME.DIFFERENCE] or layout.OPTION.DIFFERENCE_TARGET
    view.judge_shown = option[layout.OPTION_NAME.JUDGE] ~= layout.OPTION.JUDGE_HIDE
    view.lanes = {}
    for _, field in ipairs(view.fields) do
        for _, lane in ipairs(field.lanes) do
            table.insert(view.lanes, lane)
        end
    end
    return view
end

return layout
