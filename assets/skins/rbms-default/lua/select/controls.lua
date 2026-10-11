local theme = require("lua.theme")
local props = require("lua.props")
local parts = require("lua.parts")
local kit = require("lua.select.kit")

local controls = {}

local HEADER = { caption_top = 70, value_top = 66, name_top = 46 }
local SEARCH = { label_x = 368, x = 456, top = 56, width = 640, height = 36, inset = 12, id = "txt.search" }
local FILTER = { top = 40, height = 56, inset = 12 }
local FILTERS = {
    { id = "btn.mode", name = "MODE", x = 1128, width = 224, act = props.event.MODE, ref = props.text.MODE_FILTER },
    { id = "btn.sort", name = "SORT", x = 1368, width = 224, act = props.event.SORT, ref = props.text.SORT, unnamed = "DEFAULT" },
    { id = "btn.lnmode", name = "LN MODE", x = 1608, width = 248, act = props.event.LNMODE, index = props.index.LNMODE },
}
local LN_MODE_NAMES = { [0] = "LONG NOTE", "CHARGE NOTE", "HELL CHARGE NOTE" }
local NO_NAME = "-"

local HINT_ROWS = {
    { top = 1016, text = "ENTER  PLAY        ESC  BACK        UP DOWN  MOVE        H  ALL KEYS AS THEY ARE SET" },
    { top = 1046, text = "SEARCH, FILTER, SORT, FOLDERS, TABLES, SETTINGS AND KEY CONFIG: SEE H FOR THE KEY OF EACH" },
}
local HINT_WIDTH = 1064

local DOCK = { top = 1020, height = 40 }
local REPLAY = { label_x = 1160, x = 1244, cell = 40, gap = 4, underline = 2 }
local REPLAY_SLOTS = {
    { id = "btn.replay1", act = props.event.REPLAY, there = props.option.REPLAYDATA, chosen = props.option.SELECT_REPLAYDATA },
    { id = "btn.replay2", act = props.event.REPLAY2, there = props.option.REPLAYDATA2, chosen = props.option.SELECT_REPLAYDATA2 },
    { id = "btn.replay3", act = props.event.REPLAY3, there = props.option.REPLAYDATA3, chosen = props.option.SELECT_REPLAYDATA3 },
    { id = "btn.replay4", act = props.event.REPLAY4, there = props.option.REPLAYDATA4, chosen = props.option.SELECT_REPLAYDATA4 },
}
local ACTIONS = {
    { id = "btn.autoplay", label = "AUTOPLAY", x = 1440, width = 130, act = props.event.AUTOPLAY },
    { id = "btn.practice", label = "PRACTICE", x = 1582, width = 120, act = props.event.PRACTICE },
    { id = "btn.favorite", label = "FAVORITE", x = 1714, width = 142, act = props.event.FAVORITE_CHART, star = true },
}
local STAR = { id = "set.star", inset = 12, gap = 8 }

local function search(s)
    local size = theme.type.caption
    s:text({ text = "SEARCH", size = size }, { x = SEARCH.label_x, top = HEADER.caption_top, w = SEARCH.x - SEARCH.label_x }, { color = theme.color.muted })
    local field = { x = SEARCH.x, top = SEARCH.top, w = SEARCH.width, h = SEARCH.height }
    s:rect(field, theme.color.panel)
    s:rect({ x = field.x, top = field.top + field.h - theme.grid.rule, w = field.w, h = theme.grid.rule }, theme.color.line)
    local typed = { x = field.x + SEARCH.inset, top = HEADER.value_top, w = field.w - SEARCH.inset * 2 }
    s:text({ id = SEARCH.id, ref = props.text.SEARCHWORD, size = theme.type.label }, typed, { color = theme.color.text })
end

local function named(main_state, ref, unnamed)
    return function()
        local name = main_state.text(ref)
        if name == "" then
            return unnamed
        end
        return name
    end
end

local function filters(s, main_state)
    for _, filter in ipairs(FILTERS) do
        local box = { x = filter.x, top = FILTER.top, w = filter.width, h = FILTER.height }
        kit.button(s, { id = filter.id, act = filter.act }, box)
        local inner = { x = box.x + FILTER.inset, w = box.w - FILTER.inset * 2 }
        s:text({ text = filter.name, size = theme.type.caption }, { x = inner.x, top = HEADER.name_top, w = inner.w }, { color = theme.color.muted })
        local spec = { size = theme.type.label, overflow = props.text_overflow.SHRINK }
        if filter.unnamed then
            spec.value = named(main_state, filter.ref, filter.unnamed)
        elseif filter.ref then
            spec.ref = filter.ref
        else
            spec.value = parts.choice(main_state, filter.index, LN_MODE_NAMES, NO_NAME)
        end
        s:text(spec, { x = inner.x, top = HEADER.value_top, w = inner.w }, { color = theme.color.text })
    end
end

local function hints(s)
    for _, row in ipairs(HINT_ROWS) do
        local spec = { text = row.text, size = theme.type.caption, overflow = props.text_overflow.SHRINK }
        s:text(spec, { x = theme.grid.margin, top = row.top, w = HINT_WIDTH }, { color = theme.color.muted })
    end
end

local function replays(s)
    local size = theme.type.caption
    local dock = { top = DOCK.top, h = DOCK.height }
    s:text({ text = "REPLAY", size = size }, { x = REPLAY.label_x, top = theme.centered_top(dock, size), w = REPLAY.x - REPLAY.label_x }, { color = theme.color.muted })
    for index, slot in ipairs(REPLAY_SLOTS) do
        local box = { x = REPLAY.x + (index - 1) * (REPLAY.cell + REPLAY.gap), top = DOCK.top, w = REPLAY.cell, h = DOCK.height }
        kit.button(s, { id = slot.id, act = slot.act }, box)
        local digit = { text = tostring(index), size = theme.type.label, align = "center" }
        local at = { x = box.x, top = theme.centered_top(box, theme.type.label), w = box.w }
        s:text(digit, at, { color = theme.color.muted, op = { -slot.there } })
        s:text(digit, at, { color = theme.color.text, op = { slot.there } })
        local underline = { x = box.x, top = box.top + box.h - REPLAY.underline, w = box.w, h = REPLAY.underline }
        s:rect(underline, theme.color.accent, { op = { slot.chosen } })
    end
end

local function star(s, box)
    local images = { s:define_image("star_off"), s:define_image("star_on") }
    s:object("imageset", { id = STAR.id, images = images, ref = props.index.FAVORITE_CHART })
    local sprite = s:sprite("star_on")
    local at = { x = box.x + STAR.inset, top = box.top + (box.h - sprite.h) / 2, w = sprite.w, h = sprite.h }
    s:place(STAR.id, at, { color = theme.color.accent, filter = props.filter.LINEAR })
    return at.x + at.w + STAR.gap
end

local function actions(s)
    local size = theme.type.caption
    for _, action in ipairs(ACTIONS) do
        local box = { x = action.x, top = DOCK.top, w = action.width, h = DOCK.height }
        kit.button(s, { id = action.id, act = action.act }, box)
        local top = theme.centered_top(box, size)
        if action.star then
            local x = star(s, box)
            s:text({ text = action.label, size = size }, { x = x, top = top, w = box.x + box.w - x }, { color = theme.color.sub })
        else
            s:text({ text = action.label, size = size, align = "center" }, { x = box.x, top = top, w = box.w }, { color = theme.color.sub })
        end
    end
end

function controls.header(s, main_state)
    search(s)
    filters(s, main_state)
end

function controls.footer(s)
    hints(s)
    replays(s)
    actions(s)
end

return controls
