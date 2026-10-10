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

theme.difficulty = {
    [0] = { 138, 151, 168 },
    [1] = { 89, 209, 133 },
    [2] = { 86, 140, 245 },
    [3] = { 242, 179, 61 },
    [4] = { 240, 86, 76 },
    [5] = { 178, 107, 240 },
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
    pgreat = { 127, 227, 255 },
    great = { 139, 212, 95 },
    good = { 242, 201, 76 },
    bad = { 240, 136, 90 },
    poor = { 240, 86, 76 },
    miss = { 194, 69, 94 },
}

theme.lamp = {
    noplay = { 58, 69, 86 },
    failed = { 176, 65, 62 },
    assist = { 178, 107, 240 },
    light_assist = { 214, 140, 240 },
    easy = { 89, 209, 133 },
    normal = { 86, 140, 245 },
    hard = { 240, 242, 245 },
    exhard = { 242, 179, 61 },
    fullcombo = { 76, 224, 208 },
    perfect = { 255, 224, 138 },
    max = { 255, 92, 168 },
}

theme.clear = {
    [0] = { name = "NO PLAY", color = theme.lamp.noplay, text = theme.color.muted },
    [1] = { name = "FAILED", color = theme.lamp.failed },
    [2] = { name = "ASSIST EASY", color = theme.lamp.assist },
    [3] = { name = "LIGHT ASSIST EASY", color = theme.lamp.light_assist },
    [4] = { name = "EASY", color = theme.lamp.easy },
    [5] = { name = "NORMAL", color = theme.lamp.normal },
    [6] = { name = "HARD", color = theme.lamp.hard },
    [7] = { name = "EX HARD", color = theme.lamp.exhard },
    [8] = { name = "FULL COMBO", color = theme.lamp.fullcombo },
    [9] = { name = "PERFECT", color = theme.lamp.perfect },
    [10] = { name = "MAX", color = theme.lamp.max },
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
    min = theme.difficulty[2],
    max = theme.judge.poor,
    other = theme.difficulty[3],
    stop = theme.difficulty[5],
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

function theme.hex(color, alpha)
    local text = string.format("%02x%02x%02x", color[1], color[2], color[3])
    if alpha == nil then
        return text
    end
    return text .. string.format("%02x", alpha)
end

return theme
