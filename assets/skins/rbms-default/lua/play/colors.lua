local theme = require("lua.theme")

local colors = {}

colors.NO_TINT = -1
colors.PURE_NOTES = "pure"

local CATEGORY = { LANE = "Lane", NOTE = "Note", EFFECT = "Effect" }

local function row(id, name, category, label, color)
    return { id = id, name = name, category = category, label = label, color = color }
end

colors.ROW = {
    WHITE_KEY_LANE = row(1, "White Key Lane", CATEGORY.LANE, "White Key", theme.mono.panel),
    BLACK_KEY_LANE = row(2, "Black Key Lane", CATEGORY.LANE, "Black Key", theme.mono.bg),
    SCRATCH_LANE = row(3, "Scratch Lane", CATEGORY.LANE, "Scratch", theme.mono.panel),
    LANE_DIVIDER = row(4, "Lane Divider", CATEGORY.LANE, "Divider", theme.mono.line),
    BAR_LINE = row(5, "Bar Line", CATEGORY.LANE, "Bar Line", theme.mono.line),
    JUDGE_LINE = row(6, "Judge Line", CATEGORY.LANE, "Judge Line", theme.accent.white),
    LANE_COVER = row(7, "Lane Cover", CATEGORY.LANE, "Cover", theme.mono.raised),
    WHITE_KEY_NOTE = row(10, "White Key Note", CATEGORY.NOTE, "White Key", theme.note.white),
    BLACK_KEY_NOTE = row(11, "Black Key Note", CATEGORY.NOTE, "Black Key", theme.note.cyan),
    SCRATCH_NOTE = row(12, "Scratch Note", CATEGORY.NOTE, "Scratch", theme.tone.red),
    WHITE_BUTTON_NOTE = row(13, "White Button Note", CATEGORY.NOTE, "White Buttons", theme.note.white),
    YELLOW_BUTTON_NOTE = row(14, "Yellow Button Note", CATEGORY.NOTE, "Yellow Buttons", theme.tone.yellow),
    GREEN_BUTTON_NOTE = row(15, "Green Button Note", CATEGORY.NOTE, "Green Buttons", theme.tone.green),
    BLUE_BUTTON_NOTE = row(16, "Blue Button Note", CATEGORY.NOTE, "Blue Buttons", theme.tone.blue),
    RED_BUTTON_NOTE = row(17, "Red Button Note", CATEGORY.NOTE, "Red Button", theme.tone.red),
    WHITE_KEY_LN = row(40, "White Key LN", CATEGORY.NOTE, "White Key LN", theme.note.white),
    BLACK_KEY_LN = row(41, "Black Key LN", CATEGORY.NOTE, "Black Key LN", theme.note.cyan),
    SCRATCH_LN = row(42, "Scratch LN", CATEGORY.NOTE, "Scratch LN", theme.tone.red),
    WHITE_BUTTON_LN = row(43, "White Button LN", CATEGORY.NOTE, "White Buttons LN", theme.note.white),
    YELLOW_BUTTON_LN = row(44, "Yellow Button LN", CATEGORY.NOTE, "Yellow Buttons LN", theme.tone.yellow),
    GREEN_BUTTON_LN = row(45, "Green Button LN", CATEGORY.NOTE, "Green Buttons LN", theme.tone.green),
    BLUE_BUTTON_LN = row(46, "Blue Button LN", CATEGORY.NOTE, "Blue Buttons LN", theme.tone.blue),
    RED_BUTTON_LN = row(47, "Red Button LN", CATEGORY.NOTE, "Red Button LN", theme.tone.red),
    WHITE_KEY_BEAM = row(20, "White Key Beam", CATEGORY.EFFECT, "White Key Beam", theme.note.white),
    BLACK_KEY_BEAM = row(21, "Black Key Beam", CATEGORY.EFFECT, "Black Key Beam", theme.note.cyan),
    SCRATCH_BEAM = row(22, "Scratch Beam", CATEGORY.EFFECT, "Scratch Beam", theme.tone.red),
    WHITE_BUTTON_BEAM = row(23, "White Button Beam", CATEGORY.EFFECT, "White Button Beam", theme.note.white),
    YELLOW_BUTTON_BEAM = row(24, "Yellow Button Beam", CATEGORY.EFFECT, "Yellow Button Beam", theme.tone.yellow),
    GREEN_BUTTON_BEAM = row(25, "Green Button Beam", CATEGORY.EFFECT, "Green Button Beam", theme.tone.green),
    BLUE_BUTTON_BEAM = row(26, "Blue Button Beam", CATEGORY.EFFECT, "Blue Button Beam", theme.tone.blue),
    RED_BUTTON_BEAM = row(27, "Red Button Beam", CATEGORY.EFFECT, "Red Button Beam", theme.tone.red),
    BOMB = row(30, "Bomb", CATEGORY.EFFECT, "Bomb", theme.note.white),
}

local BUTTON_LABEL = { WHITE_KEY_LANE = "Odd Buttons", BLACK_KEY_LANE = "Even Buttons" }

local SHARED_LANE_ROWS = { "LANE_DIVIDER", "BAR_LINE", "JUDGE_LINE", "LANE_COVER" }

local KEYED = {
    lanes = { "WHITE_KEY_LANE", "BLACK_KEY_LANE", "SCRATCH_LANE" },
    notes = { "WHITE_KEY_NOTE", "BLACK_KEY_NOTE", "SCRATCH_NOTE" },
    longs = { "WHITE_KEY_LN", "BLACK_KEY_LN", "SCRATCH_LN" },
    beams = { "WHITE_KEY_BEAM", "BLACK_KEY_BEAM", "SCRATCH_BEAM" },
}

local BUTTONS = {
    lanes = { "WHITE_KEY_LANE", "BLACK_KEY_LANE" },
    notes = { "WHITE_BUTTON_NOTE", "YELLOW_BUTTON_NOTE", "GREEN_BUTTON_NOTE", "BLUE_BUTTON_NOTE", "RED_BUTTON_NOTE" },
    longs = { "WHITE_BUTTON_LN", "YELLOW_BUTTON_LN", "GREEN_BUTTON_LN", "BLUE_BUTTON_LN", "RED_BUTTON_LN" },
    beams = { "WHITE_BUTTON_BEAM", "YELLOW_BUTTON_BEAM", "GREEN_BUTTON_BEAM", "BLUE_BUTTON_BEAM", "RED_BUTTON_BEAM" },
}

local KEY_ROWS = {
    white = { note = "WHITE_KEY_NOTE", long = "WHITE_KEY_LN", beam = "WHITE_KEY_BEAM" },
    black = { note = "BLACK_KEY_NOTE", long = "BLACK_KEY_LN", beam = "BLACK_KEY_BEAM" },
    scratch = { note = "SCRATCH_NOTE", long = "SCRATCH_LN", beam = "SCRATCH_BEAM" },
}

local BUTTON_ROWS = {
    white = { note = "WHITE_BUTTON_NOTE", long = "WHITE_BUTTON_LN", beam = "WHITE_BUTTON_BEAM" },
    yellow = { note = "YELLOW_BUTTON_NOTE", long = "YELLOW_BUTTON_LN", beam = "YELLOW_BUTTON_BEAM" },
    green = { note = "GREEN_BUTTON_NOTE", long = "GREEN_BUTTON_LN", beam = "GREEN_BUTTON_BEAM" },
    blue = { note = "BLUE_BUTTON_NOTE", long = "BLUE_BUTTON_LN", beam = "BLUE_BUTTON_BEAM" },
    red = { note = "RED_BUTTON_NOTE", long = "RED_BUTTON_LN", beam = "RED_BUTTON_BEAM" },
}

local SHARED_DRAWN = {
    LANE_DIVIDER = "line",
    BAR_LINE = "line",
    JUDGE_LINE = "accent",
    LANE_COVER = "raised",
}

local FACE_ROWS = { white = "WHITE_KEY_LANE", black = "BLACK_KEY_LANE", scratch = "SCRATCH_LANE" }

local function keys_of(mode)
    local groups = mode.scratch and KEYED or BUTTONS
    local keys = {}
    for _, list in ipairs({ groups.lanes, SHARED_LANE_ROWS, groups.notes, groups.longs, groups.beams, { "BOMB" } }) do
        for _, key in ipairs(list) do
            table.insert(keys, key)
        end
    end
    return keys
end

function colors.header(mode)
    local list = {}
    for _, key in ipairs(keys_of(mode)) do
        local entry = colors.ROW[key]
        local label = not mode.scratch and BUTTON_LABEL[key] or entry.label
        table.insert(list, { category = entry.category, name = entry.name, label = label, id = entry.id, def = theme.hex(entry.color) })
    end
    return list
end

local function lane_rows(lane)
    if lane.kind == "button" then
        local rows = BUTTON_ROWS[lane.color]
        return lane.shaded and "BLACK_KEY_LANE" or "WHITE_KEY_LANE", rows.note, rows.long, rows.beam
    end
    local rows = KEY_ROWS[lane.kind]
    return FACE_ROWS[lane.kind], rows.note, rows.long, rows.beam
end

local function stored_hex(chosen)
    return string.format("%02x%02x%02x%02x", chosen.r, chosen.g, chosen.b, chosen.a)
end

local function drawn_of(view, stored)
    local drawn = { BOMB = theme.hex(view.bomb_tint or theme.note.white) }
    for key, color in pairs(SHARED_DRAWN) do
        drawn[key] = theme.hex(theme.color[color])
    end
    for _, lane in ipairs(view.lanes) do
        local face, note, long, beam = lane_rows(lane)
        local note_color = theme.hex(lane.tint)
        local chosen = stored[colors.ROW[note].name]
        drawn[face] = theme.hex(lane.shaded and theme.color.bg or theme.color.panel)
        drawn[note], drawn[beam] = note_color, note_color
        drawn[long] = chosen ~= nil and chosen.custom and stored_hex(chosen) or note_color
    end
    return drawn
end

function colors.paint(view, stored, rows)
    local tint = {}
    for _, key in ipairs(keys_of(view.mode)) do
        local entry = colors.ROW[key]
        local chosen = stored[entry.name]
        if chosen ~= nil and chosen.custom then
            tint[key] = entry.id
        end
    end
    view.tint = tint
    for _, lane in ipairs(view.lanes) do
        local face, note, long, beam = lane_rows(lane)
        lane.face_tint, lane.note_tint, lane.beam_tint = tint[face], tint[note], tint[beam]
        lane.long_tint = tint[long] or lane.note_tint
    end
    local drawn = drawn_of(view, stored)
    local key_of = {}
    for key, entry in pairs(colors.ROW) do
        key_of[entry.id] = key
    end
    for _, entry in ipairs(rows) do
        entry.drawn = drawn[key_of[entry.id]]
    end
end

function colors.base(color, tint)
    return tint ~= nil and theme.color.white or color
end

function colors.tinted(destination, tint)
    destination.tint = tint
    return destination
end

return colors
