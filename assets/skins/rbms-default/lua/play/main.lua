local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")
local layout = require("lua.play.layout")
local field = require("lua.play.field")
local info = require("lua.play.info")

local play = {}

local TIME = { input = 500, fadeout = 600, loadend = 1500, playstart = 1000, close = 1500, finishmargin = 1000 }
local BOMB_UP_TO_GREAT = 1
local CLOSING = 500
local FAILED = { cover_alpha = 200, cover_fade = 400, size = theme.type.display, slide = 300, at = 150 }

local function chosen(name, first, second)
    return { name = name, item = { first, second }, def = first.name }
end

local function properties(mode)
    local option, name = layout.OPTION, layout.OPTION_NAME
    local list = { chosen(name.LANES, { name = "Normal", op = option.LANES_NORMAL }, { name = "Wide", op = option.LANES_WIDE }) }
    if mode.sides == 1 and mode.scratch then
        table.insert(list, chosen(name.FIELD, { name = "Left", op = option.FIELD_LEFT }, { name = "Right", op = option.FIELD_RIGHT }))
        table.insert(list, chosen(name.SCRATCH, { name = "Left", op = option.SCRATCH_LEFT }, { name = "Right", op = option.SCRATCH_RIGHT }))
    end
    table.insert(list, {
        name = name.DETAIL,
        item = {
            { name = "Fast-Slow", op = option.DETAIL_FAST_SLOW },
            { name = "Milliseconds", op = option.DETAIL_MILLISECONDS },
            { name = "Off", op = option.DETAIL_OFF },
        },
        def = "Fast-Slow",
    })
    table.insert(list, {
        name = name.DIFFERENCE,
        item = {
            { name = "Target", op = option.DIFFERENCE_TARGET },
            { name = "Best", op = option.DIFFERENCE_BEST },
            { name = "Off", op = option.DIFFERENCE_OFF },
        },
        def = "Target",
    })
    table.insert(list, parts.shown_or_hidden(name.JUDGE, option.JUDGE_SHOW, option.JUDGE_HIDE))
    if mode.sides == 1 then
        table.insert(list, chosen(name.GRAPH, { name = "On", op = option.GRAPH_ON }, { name = "Off", op = option.GRAPH_OFF }))
    end
    if mode.scratch then
        local sets = {}
        for index, set in ipairs(layout.NOTE_SETS) do
            sets[index] = { name = set.name, op = set.op }
        end
        table.insert(list, { name = name.NOTES, item = sets, def = sets[1].name })
    end
    table.insert(list, chosen(name.BOMB, { name = "Note", op = option.BOMB_NOTE }, { name = "White", op = option.BOMB_WHITE }))
    return theme.properties(list)
end

local function offsets(mode)
    local list = { { name = layout.JUDGE_POSITION.name, id = layout.JUDGE_POSITION.id, y = true } }
    for _, row in ipairs(layout.PART_ROWS) do
        if mode.sides == 1 or not row.single_only then
            table.insert(list, { name = row.name, id = row.id, a = true })
        end
    end
    return list
end

function play.header(mode)
    return {
        type = mode.type,
        name = theme.name .. " (" .. mode.name .. ")",
        author = theme.author,
        w = theme.width,
        h = theme.height,
        input = TIME.input,
        scene = 0,
        fadeout = TIME.fadeout,
        loadend = TIME.loadend,
        playstart = TIME.playstart,
        close = TIME.close,
        finishmargin = TIME.finishmargin,
        judgetimer = BOMB_UP_TO_GREAT,
        property = properties(mode),
        offset = offsets(mode),
    }
end

local function failed(s)
    local whole = { x = 0, top = 0, w = theme.width, h = theme.height }
    local timer = props.timer.FAILED
    s:rect(whole, theme.color.bg, { timer = timer, keys = { { time = 0, a = 0 }, { time = FAILED.cover_fade, a = FAILED.cover_alpha } } })
    local word = { x = 0, top = (theme.height - theme.cap_height(FAILED.size)) / 2, w = theme.width }
    local falling = motion.slide_in(FAILED.at, FAILED.slide, 0, -theme.time.slide_distance, { timer = timer, color = theme.tone.red })
    s:text({ text = "FAILED", size = FAILED.size, align = "center" }, word, falling)
end

function play.build(spec)
    local mode = layout.mode(spec.keys)
    local header = play.header(mode)
    if skin_config == nil then
        return header
    end
    local main_state = require("main_state")
    local option = skin_config.option or {}
    theme.apply(option)
    local s = ui.new(header)
    s:atlas("common")
    s:atlas("play")
    local view = layout.of(mode, option)
    parts.backdrop(s)
    info.build(s, view, main_state)
    field.under(s, view)
    field.over(s, view)
    failed(s)
    parts.fade_from_black(s, theme.time.fade)
    parts.fade_to_black(s, 0, CLOSING)
    return s:finish()
end

return play
