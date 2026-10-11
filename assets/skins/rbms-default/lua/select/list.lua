local theme = require("lua.theme")
local props = require("lua.props")
local motion = require("lua.motion")
local kit = require("lua.select.kit")

local list = {}

local WHEEL = { x = 1128, width = 728, bar_height = 64, pitch = 72, slots = 15, center = 7, center_top = 520, first_clickable = 3, last_clickable = 11 }
local WHOLE = { first = 2, last = 12 }
local FRAME = { x = 1096, width = 760, band = 8, pulse = 2000, pulse_low = 160 }
local FRAME_KINDS = { props.option.FOLDERBAR, props.option.SONGBAR, props.option.GRADEBAR }
local BAR_SET = "set.bar"
local BAR_SPRITES = { "bar_song", "bar_folder", "bar_table", "bar_course", "bar_missing", "bar_command", "bar_search" }

local LAMP = { x = 0, width = 8, sprite = "lamp_" }

local LEVEL = { x = 16, digits = 3, sprite = "num24", id = "num.bar.level", kinds = 7 }
local TROPHY = { x = 104, side = 16 }
local TROPHY_COLORS = { { 176, 112, 72 }, { 192, 198, 208 }, { 242, 201, 76 } }

local TITLE = { size = 28, chart_x = 80, plain_x = 136, plain_right = 712, label_gap = 18 }
local TITLE_KINDS = {
    { id = "txt.bar.normal", color = theme.color.text },
    { id = "txt.bar.new", color = theme.color.text },
    { id = "txt.bar.chart", color = theme.color.text, chart = true },
    { id = "txt.bar.chart.new", color = theme.color.text, chart = true },
    { id = "txt.bar.folder", color = theme.color.text },
    { id = "txt.bar.folder.new", color = theme.color.text },
    { id = "txt.bar.table", color = theme.color.text },
    { id = "txt.bar.course", color = theme.color.text, labelled = true },
    { id = "txt.bar.missing", color = theme.color.muted },
    { id = "txt.bar.command", color = theme.color.sub },
    { id = "txt.bar.search", color = theme.color.sub },
}

local LABEL = { right = 714, pitch = 20 }
local LABELS = {
    { sprite = "label_ln", color = theme.color.accent, row = 0 },
    { sprite = "label_random", color = theme.color.sub, row = 1 },
    { sprite = "label_mine", color = theme.tone.red, row = -1 },
    { sprite = "label_cn", color = theme.color.accent, row = 0 },
    { sprite = "label_hcn", color = theme.color.accent, row = 0 },
}

local GRAPH = { id = "graph.bar.lamps", x = 136, top = 54, width = 576, height = 4, by_lamp = -1 }

local SCROLL = { x = 1872, top = 136, width = 12, height = 832, thumb = 32 }

local function slot_top(slot)
    return WHEEL.center_top + (slot - WHEEL.center) * WHEEL.pitch
end

local function bar_box()
    return { x = 0, top = 0, w = WHEEL.width, h = WHEEL.bar_height }
end

local function centered(height)
    return (WHEEL.bar_height - height) / 2
end

local function frame(s)
    local top = slot_top(WHEEL.center)
    local box = { x = FRAME.x, top = top, w = FRAME.width, h = WHEEL.bar_height }
    local band = { x = FRAME.x, top = top, w = FRAME.band, h = WHEEL.bar_height }
    for _, kind in ipairs(FRAME_KINDS) do
        s:rect(box, theme.color.raised, { op = { kind } })
        s:rect(band, theme.color.accent, motion.pulse(FRAME.pulse, FRAME.pulse_low, { op = { kind } }))
    end
end

local function bar_set(s)
    local images = {}
    for index, sprite in ipairs(BAR_SPRITES) do
        images[index] = s:define_image(sprite)
    end
    s:object("imageset", { id = BAR_SET, images = images })
end

local function bars(s)
    local slots = {}
    for slot = 0, WHEEL.slots - 1 do
        local box = { x = WHEEL.x, top = slot_top(slot), w = WHEEL.width, h = WHEEL.bar_height }
        slots[slot + 1] = s:dst(BAR_SET, box, { filter = props.filter.LINEAR })
    end
    return slots
end

local function label_column_left(s)
    local widest = 0
    for _, label in ipairs(LABELS) do
        widest = math.max(widest, s:sprite(label.sprite).w)
    end
    return LABEL.right - widest
end

local function titles(s)
    local parent = bar_box()
    local top = centered(theme.cap_height(TITLE.size))
    local labelled_right = label_column_left(s) - TITLE.label_gap
    local placed = {}
    for index, kind in ipairs(TITLE_KINDS) do
        s:define_text({ id = kind.id, size = TITLE.size, overflow = props.text_overflow.SHRINK })
        local x = kind.chart and TITLE.chart_x or TITLE.plain_x
        local right = (kind.chart or kind.labelled) and labelled_right or TITLE.plain_right
        placed[index] = s:dst(kind.id, { x = x, top = top, w = right - x, h = TITLE.size }, { color = kind.color }, parent)
    end
    return placed
end

local function levels(s)
    local parent = bar_box()
    local sprite = s:sprite(LEVEL.sprite)
    s:define_number({ id = LEVEL.id, sprite = LEVEL.sprite, ref = props.number.PLAYLEVEL, align = "center" }, LEVEL.digits)
    local box = { x = LEVEL.x, top = centered(sprite.cell_h), w = sprite.cell_w, h = sprite.cell_h }
    local placed = {}
    for index = 1, LEVEL.kinds do
        local color = theme.difficulty[index - 1] or theme.difficulty[0]
        placed[index] = s:dst(LEVEL.id, box, { color = color, filter = props.filter.LINEAR }, parent)
    end
    return placed
end

local function lamps(s)
    local parent = bar_box()
    local block = s:define_image("block")
    local box = { x = LAMP.x, top = 0, w = LAMP.width, h = WHEEL.bar_height }
    local placed = {}
    for index = 0, theme.clear_last do
        local lamp = theme.clear[index]
        local frames = LAMP.sprite .. lamp.key
        if s.sprites[frames] then
            placed[index + 1] = s:dst(s:define_image(frames, { cycle = theme.lamp_cycle }), box, nil, parent)
        else
            placed[index + 1] = s:dst(block, box, { color = lamp.color }, parent)
        end
    end
    return placed
end

local function trophies(s)
    local parent = bar_box()
    local block = s:define_image("block")
    local box = { x = TROPHY.x, top = centered(TROPHY.side), w = TROPHY.side, h = TROPHY.side }
    local placed = {}
    for index, color in ipairs(TROPHY_COLORS) do
        placed[index] = s:dst(block, box, { color = color }, parent)
    end
    return placed
end

local function labels(s)
    local parent = bar_box()
    local placed = {}
    for index, label in ipairs(LABELS) do
        local sprite = s:sprite(label.sprite)
        local box = { x = LABEL.right - sprite.w, top = centered(sprite.h) + label.row * LABEL.pitch, w = sprite.w, h = sprite.h }
        placed[index] = s:dst(s:define_image(label.sprite), box, { color = label.color, filter = props.filter.LINEAR }, parent)
    end
    return placed
end

local function graph(s)
    local strip = s:sprite("lamp_strip")
    s:object("graph", { id = GRAPH.id, src = strip.src, x = strip.x, y = strip.y, w = strip.w, h = strip.h, divx = strip.divx, divy = strip.divy, type = GRAPH.by_lamp })
    return s:dst(GRAPH.id, { x = GRAPH.x, top = GRAPH.top, w = GRAPH.width, h = GRAPH.height }, nil, bar_box())
end

local function wheel(s)
    local clickable = {}
    for slot = WHEEL.first_clickable, WHEEL.last_clickable do
        table.insert(clickable, slot)
    end
    bar_set(s)
    s:single("songlist", {
        center = WHEEL.center,
        clickable = clickable,
        listoff = bars(s),
        liston = bars(s),
        text = titles(s),
        level = levels(s),
        lamp = lamps(s),
        trophy = trophies(s),
        label = labels(s),
        graph = graph(s),
    })
end

local function scrollbar(s)
    s:rect({ x = SCROLL.x, top = SCROLL.top, w = SCROLL.width, h = SCROLL.height }, theme.color.panel)
    local thumb = kit.slider(s, props.rate.MUSICSELECT_POSITION, props.slider_angle.DOWN, SCROLL.height - SCROLL.thumb)
    s:object("slider", thumb, { x = SCROLL.x, top = SCROLL.top, w = SCROLL.width, h = SCROLL.thumb }, { color = theme.color.muted })
end

local function trim(s)
    local gap = WHEEL.pitch - WHEEL.bar_height
    local upper = slot_top(WHOLE.first) - gap
    local lower = slot_top(WHOLE.last) + WHEEL.pitch
    local x = FRAME.x
    local width = SCROLL.x - FRAME.x
    local grid = theme.grid
    s:rect({ x = x, top = grid.header_rule, w = width, h = upper - grid.header_rule }, theme.color.bg)
    s:rect({ x = x, top = lower, w = width, h = grid.footer_rule - lower }, theme.color.bg)
end

function list.build(s)
    frame(s)
    wheel(s)
    trim(s)
    scrollbar(s)
end

return list
