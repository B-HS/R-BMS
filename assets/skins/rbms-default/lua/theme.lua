local i18n = require("lua.i18n")

local theme = {}

theme.name = "R-BMS Default"
theme.author = "R-BMS"
theme.width = 1920
theme.height = 1080

theme.mono = {
    bg = { 0, 0, 0 },
    panel = { 22, 22, 22 },
    raised = { 38, 38, 38 },
    line = { 60, 60, 60 },
    text = { 242, 242, 242 },
    sub = { 184, 184, 184 },
    muted = { 128, 128, 128 },
    bar = { 200, 200, 200 },
    lamp_off = { 68, 68, 68 },
}

theme.slate = {
    bg = { 12, 15, 20 },
    panel = { 20, 25, 34 },
    raised = { 29, 36, 48 },
    line = { 43, 53, 69 },
    text = { 241, 244, 248 },
    sub = { 167, 178, 193 },
    muted = { 102, 115, 135 },
    bar = { 92, 115, 160 },
    lamp_off = { 58, 69, 86 },
}

theme.accent = {
    white = { 255, 255, 255 },
    sky = { 76, 201, 240 },
    amber = { 242, 179, 61 },
    green = { 89, 209, 133 },
    pink = { 240, 104, 158 },
}

theme.accent_dim = {
    white = { 102, 102, 102 },
    sky = { 31, 95, 120 },
    amber = { 97, 72, 24 },
    green = { 36, 84, 53 },
    pink = { 96, 42, 63 },
}

theme.note = {
    white = { 241, 244, 248 },
    cyan = { 76, 201, 240 },
    gray = { 140, 140, 140 },
    silver = { 196, 196, 196 },
    shade = { 12, 15, 20 },
}

theme.OPTION = {
    THEME_MONO = 920,
    THEME_SLATE = 921,
    ACCENT_WHITE = 922,
    ACCENT_SKY = 923,
    ACCENT_AMBER = 924,
    ACCENT_GREEN = 925,
    ACCENT_PINK = 926,
}

theme.OPTION_NAME = { THEME = "Theme", ACCENT = "Accent" }

local SURFACES = {
    { name = "Mono", op = theme.OPTION.THEME_MONO, palette = theme.mono, suffix = "" },
    { name = "Slate", op = theme.OPTION.THEME_SLATE, palette = theme.slate, suffix = "_slate" },
}

local ACCENTS = {
    { name = "White", op = theme.OPTION.ACCENT_WHITE, key = "white" },
    { name = "Sky", op = theme.OPTION.ACCENT_SKY, key = "sky" },
    { name = "Amber", op = theme.OPTION.ACCENT_AMBER, key = "amber" },
    { name = "Green", op = theme.OPTION.ACCENT_GREEN, key = "green" },
    { name = "Pink", op = theme.OPTION.ACCENT_PINK, key = "pink" },
}

local function copied(color)
    return { color[1], color[2], color[3] }
end

theme.color = { black = { 0, 0, 0 }, white = { 255, 255, 255 } }
for name, color in pairs(SURFACES[1].palette) do
    theme.color[name] = copied(color)
end
theme.color.accent = copied(theme.accent[ACCENTS[1].key])
theme.color.accent_dim = copied(theme.accent_dim[ACCENTS[1].key])
theme.sprite_suffix = SURFACES[1].suffix

theme.tone = {
    red = { 240, 86, 76 },
    orange = { 240, 136, 90 },
    yellow = { 242, 179, 61 },
    green = { 89, 209, 133 },
    blue = { 86, 140, 245 },
    violet = { 178, 107, 240 },
    rose = { 194, 69, 94 },
}

theme.difficulty = {
    [0] = { 195, 195, 195 },
    [1] = { 6, 255, 0 },
    [2] = { 18, 210, 215 },
    [3] = { 255, 192, 0 },
    [4] = { 255, 0, 0 },
    [5] = { 168, 64, 170 },
}

theme.difficulty_name = {
    [0] = i18n.t("common_unknown"),
    [1] = "BEGINNER",
    [2] = "NORMAL",
    [3] = "HYPER",
    [4] = "ANOTHER",
    [5] = "INSANE",
}

theme.judge = {
    pgreat = { 107, 166, 245 },
    great = { 246, 171, 34 },
    good = { 249, 178, 31 },
    bad = { 251, 129, 22 },
    poor = { 242, 82, 0 },
}

theme.judge_word = {
    pgreat = { 132, 206, 255 },
    great = { 255, 214, 51 },
}

theme.timing = {
    fast = { 3, 169, 255 },
    slow = { 255, 69, 3 },
}

theme.lamp = {
    failed = { 255, 0, 0 },
    assist = { 255, 99, 253 },
    light_assist = { 101, 99, 255 },
    easy = { 0, 255, 36 },
    normal = { 255, 187, 0 },
    hard = { 223, 223, 223 },
    exhard = { 255, 110, 110 },
    fullcombo = { 0, 223, 255 },
    perfect = { 244, 153, 242 },
    max = { 234, 243, 0 },
}

theme.grade_lamp = {
    easy = { 58, 209, 159 },
    clear = { 196, 167, 150 },
    hard = { 175, 184, 239 },
    exhard = { 202, 118, 169 },
}

theme.lamp.noplay = theme.color.lamp_off

theme.lamp_cycle = 50

theme.clear = {
    [0] = { key = "noplay", name = "NO PLAY", color = theme.lamp.noplay, text = theme.color.muted },
    [1] = { key = "failed", name = "FAILED", color = theme.lamp.failed },
    [2] = { key = "assist", name = "ASSIST", color = theme.lamp.assist },
    [3] = { key = "light_assist", name = "LIGHT ASSIST", color = theme.lamp.light_assist },
    [4] = { key = "easy", name = "EASY CLEAR", color = theme.lamp.easy },
    [5] = { key = "normal", name = "CLEAR", color = theme.lamp.normal },
    [6] = { key = "hard", name = "HARD CLEAR", color = theme.lamp.hard },
    [7] = { key = "exhard", name = "EX HARD CLEAR", color = theme.lamp.exhard },
    [8] = { key = "fullcombo", name = "FULL COMBO", color = theme.lamp.fullcombo },
    [9] = { key = "perfect", name = "PERFECT", color = theme.lamp.perfect },
    [10] = { key = "max", name = "MAX", color = theme.lamp.max },
}
theme.clear_last = 10
theme.graded_clear = 3

theme.assist_grade = {
    [0] = { name = theme.clear[theme.graded_clear].name, color = theme.lamp.light_assist },
    [1] = { name = "ASSIST EASY", color = theme.grade_lamp.easy },
    [2] = { name = "ASSIST CLEAR", color = theme.grade_lamp.clear },
    [3] = { name = "ASSIST HARD", color = theme.grade_lamp.hard },
    [4] = { name = "ASSIST EXHARD", color = theme.grade_lamp.exhard },
}
theme.assist_grade_last = 4

theme.rank_name = { "AAA", "AA", "A", "B", "C", "D", "E", "F" }

theme.rank = {
    f = { 56, 56, 56 },
    e = { 68, 68, 68 },
    d = { 80, 80, 80 },
    c = { 94, 94, 94 },
    b = { 110, 110, 110 },
    a = { 176, 112, 72 },
    aa = { 192, 198, 208 },
    aaa = { 242, 201, 76 },
}

theme.rank_ink = {
    f = { 242, 242, 242 },
    e = { 242, 242, 242 },
    d = { 242, 242, 242 },
    c = { 242, 242, 242 },
    b = { 242, 242, 242 },
    a = { 26, 16, 6 },
    aa = { 26, 28, 32 },
    aaa = { 38, 28, 8 },
}

theme.rank_key = { [0] = "f", "e", "d", "c", "b", "a", "aa", "aaa" }
theme.rank_last = 7

theme.medal = {
    gold = theme.rank.aaa,
    silver = theme.rank.aa,
    bronze = theme.rank.a,
}

theme.medal_ink = {
    gold = theme.rank_ink.aaa,
    silver = theme.rank_ink.aa,
    bronze = theme.rank_ink.a,
}

theme.medal_order = { "gold", "silver", "bronze" }

theme.gauge_name = {
    [0] = "ASSIST EASY",
    [1] = "EASY",
    [2] = "NORMAL",
    [3] = "HARD",
    [4] = "EX HARD",
    [5] = "HAZARD",
    [6] = "CLASS",
    [7] = "EX CLASS",
    [8] = "EX HARD CLASS",
}

theme.random_name = {
    [0] = "OFF",
    [1] = "MIRROR",
    [2] = "RANDOM",
    [3] = "R-RANDOM",
    [4] = "S-RANDOM",
    [5] = "ROTATE",
    [6] = "H-RANDOM",
    [7] = "ALL-SCRATCH",
}

theme.bpm_line = {
    main = theme.color.accent,
    min = theme.tone.blue,
    max = theme.tone.red,
    other = theme.tone.yellow,
    stop = theme.tone.violet,
    transition = theme.color.muted,
    width = 2,
}

theme.type = {
    display = 96,
    title = 64,
    heading = 44,
    body = 32,
    label = 24,
    caption = 18,
}

theme.cap_height_ratio = 0.727

theme.grid = {
    unit = 8,
    margin = 64,
    column = 120,
    gutter = 32,
    columns = 12,
    header_rule = 104,
    footer_rule = 1000,
    rule = 2,
}

theme.time = {
    fade = 300,
    slide = 300,
    stagger = 50,
    slide_distance = 32,
}

theme.font = {
    id = "inter",
    path = "font/Inter-Regular.ttf",
    bold = { id = "inter-bold", path = "font/Inter-Bold.ttf" },
}

theme.panel = {
    column = {
        orange = theme.tone.orange,
        green = theme.tone.green,
        violet = theme.tone.violet,
        sky = theme.accent.sky,
        locked = theme.mono.muted,
    },
    pick = { 255, 152, 52 },
    pick_ink = { 26, 16, 6 },
    glow = 112,
    key_white = { 226, 230, 236 },
    key_black = { 30, 33, 40 },
    key_white_ink = { 28, 30, 36 },
    key_black_ink = { 236, 238, 242 },
    arrow = theme.tone.red,
    green_number = theme.tone.green,
    caution = theme.tone.orange,
}

local RADAR_AREA = { x = 64, top = 640, width = 256, height = 256 }
local RADAR_PICTURE = { side = 256, radius = 124 }
local RADAR_SIDE = 144
local RADAR_CENTER_DROP = 136

theme.radar = {
    area = RADAR_AREA,
    picture = RADAR_PICTURE,
    side = RADAR_SIDE,
    radius = RADAR_SIDE * RADAR_PICTURE.radius / RADAR_PICTURE.side,
    center = { x = RADAR_AREA.x + RADAR_AREA.width / 2, top = RADAR_AREA.top + RADAR_CENTER_DROP },
    label_gap = 8,
    outline = theme.grid.rule,
    outline_color = theme.color.text,
    axis_line = 1,
    axis_color = theme.color.muted,
    fill = { 244, 236, 72 },
    fill_alpha = 176,
    star = { 246, 236, 150 },
    axis_order = { "notes", "peak", "scratch", "soflan", "charge", "chord" },
    axis = {
        notes = { 238, 130, 236 },
        peak = { 250, 164, 82 },
        scratch = { 238, 78, 78 },
        soflan = { 104, 188, 232 },
        charge = { 132, 104, 250 },
        chord = { 188, 244, 104 },
    },
}

theme.ranking = {
    you = theme.color.accent,
    you_face = theme.color.raised,
    rival = theme.color.sub,
}

theme.select_layout = {
    head = {
        chip = { top = 56, height = 36 },
        style_chip = { x = 232, width = 88 },
        mode_chip = { x = 336, width = 200 },
        search = { label_x = 552, x = 640, width = 336 },
        clock = { x = 992, width = 104 },
    },
    cover = { x = 64, top = 352, side = 256 },
    chart_data = { x = 352, top = 352, width = 744, height = 256 },
    score = { x = 352, top = 640, width = 400, height = 256 },
    records = { x = 784, top = 640, width = 312, height = 256, rows = 6, header_height = 40, row_height = 36 },
    targets = { x = 352, top = 912, width = 744, height = 64 },
    bar = {
        badge = { x = 0, width = 56 },
        lamp = { x = 56, width = 8 },
        level = { x = 64, width = 64 },
        title_x = 144,
        thumb = { x = 664, side = 64 },
        label_right = 656,
        selected = { left = 1104, grow = 4, band = 8 },
    },
}

function theme.column(index, span)
    local grid = theme.grid
    local pitch = grid.column + grid.gutter
    return grid.margin + index * pitch, (span or 1) * pitch - grid.gutter
end

function theme.content_width()
    return theme.width - theme.grid.margin * 2
end

function theme.cap_height(size)
    return math.floor(size * theme.cap_height_ratio + 0.5)
end

function theme.text_top(baseline, size)
    return baseline - theme.cap_height(size)
end

function theme.centered_top(box, size)
    return box.top + (box.h - theme.cap_height(size)) / 2
end

function theme.clear_text(entry)
    return entry.text or entry.color
end

function theme.mix(color, toward, share)
    local mixed = {}
    for channel = 1, 3 do
        mixed[channel] = math.floor(toward[channel] + (color[channel] - toward[channel]) * share + 0.5)
    end
    return mixed
end

function theme.hex(color, alpha)
    local text = string.format("%02x%02x%02x", color[1], color[2], color[3])
    if alpha == nil then
        return text
    end
    return text .. string.format("%02x", alpha)
end

local function chosen(rows, op)
    for _, row in ipairs(rows) do
        if row.op == op then
            return row
        end
    end
    return rows[1]
end

local function painted(target, color)
    for channel = 1, 3 do
        target[channel] = color[channel]
    end
end

local function property(name, rows)
    local items = {}
    for index, row in ipairs(rows) do
        items[index] = { name = row.name, op = row.op }
    end
    return { name = name, item = items, def = rows[1].name }
end

function theme.properties(list)
    local rows = list or {}
    table.insert(rows, property(theme.OPTION_NAME.THEME, SURFACES))
    table.insert(rows, property(theme.OPTION_NAME.ACCENT, ACCENTS))
    return rows
end

function theme.apply(option)
    local picked = option or {}
    local surface = chosen(SURFACES, picked[theme.OPTION_NAME.THEME])
    for name, color in pairs(surface.palette) do
        painted(theme.color[name], color)
    end
    local accent = chosen(ACCENTS, picked[theme.OPTION_NAME.ACCENT])
    painted(theme.color.accent, theme.accent[accent.key])
    painted(theme.color.accent_dim, theme.accent_dim[accent.key])
    theme.sprite_suffix = surface.suffix
end

function theme.sprite(name)
    return name .. theme.sprite_suffix
end

return theme
