local theme = {}

theme.name = "R-BMS Default"
theme.author = "R-BMS"
theme.width = 1920
theme.height = 1080

theme.color = {
    bg = { 12, 15, 20 },
    panel = { 20, 25, 34 },
    raised = { 29, 36, 48 },
    line = { 43, 53, 69 },
    text = { 241, 244, 248 },
    sub = { 167, 178, 193 },
    muted = { 102, 115, 135 },
    accent = { 76, 201, 240 },
    accent_dim = { 31, 95, 120 },
    black = { 0, 0, 0 },
    white = { 255, 255, 255 },
}

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
    [0] = "UNKNOWN",
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

theme.judge_flash = {
    blue = { 107, 166, 245 },
    pink = { 244, 125, 167 },
    mint = { 187, 245, 208 },
}

theme.judge_cycle = { flash = 120, blink = 80 }

theme.timing = {
    fast = { 3, 169, 255 },
    slow = { 255, 69, 3 },
}

theme.lamp = {
    noplay = { 58, 69, 86 },
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

theme.lamp_cycle = 50

theme.clear = {
    [0] = { key = "noplay", name = "NO PLAY", color = theme.lamp.noplay, text = theme.color.muted },
    [1] = { key = "failed", name = "FAILED", color = theme.lamp.failed },
    [2] = { key = "assist", name = "ASSIST CLEAR", color = theme.lamp.assist },
    [3] = { key = "light_assist", name = "LIGHT ASSIST CLEAR", color = theme.lamp.light_assist },
    [4] = { key = "easy", name = "EASY CLEAR", color = theme.lamp.easy },
    [5] = { key = "normal", name = "CLEAR", color = theme.lamp.normal },
    [6] = { key = "hard", name = "HARD CLEAR", color = theme.lamp.hard },
    [7] = { key = "exhard", name = "EX HARD CLEAR", color = theme.lamp.exhard },
    [8] = { key = "fullcombo", name = "FULL COMBO", color = theme.lamp.fullcombo },
    [9] = { key = "perfect", name = "PERFECT", color = theme.lamp.perfect },
    [10] = { key = "max", name = "MAX", color = theme.lamp.max },
}
theme.clear_last = 10

theme.rank_name = { "AAA", "AA", "A", "B", "C", "D", "E", "F" }

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

return theme
