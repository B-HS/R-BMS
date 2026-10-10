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
    great = { 242, 201, 76 },
    good = { 139, 212, 95 },
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
    max = { 255, 255, 255 },
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

return theme
