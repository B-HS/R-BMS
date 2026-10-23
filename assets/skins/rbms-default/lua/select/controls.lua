local theme = require("lua.theme")
local props = require("lua.props")
local parts = require("lua.parts")
local ui = require("lua.ui")
local i18n = require("lua.i18n")
local kit = require("lua.select.kit")

local controls = {}

local HEAD = theme.select_layout.head
local HEADER = { caption_top = 70, value_top = 66, name_top = 46 }
local SEARCH = { label_x = HEAD.search.label_x, x = HEAD.search.x, top = HEAD.chip.top, width = HEAD.search.width, height = HEAD.chip.height, inset = 12, id = "txt.search" }
local FILTER = { top = 40, height = 56, inset = 12 }
local FILTERS = {
    { id = "btn.mode", name = i18n.t("bar_mode"), x = 1128, width = 224, act = props.event.MODE, ref = props.text.MODE_FILTER, unnamed = "" },
    { id = "btn.sort", name = i18n.t("bar_sort"), x = 1368, width = 224, act = props.event.SORT, ref = props.text.SORT, unnamed = "DEFAULT" },
    { id = "btn.lnmode", name = i18n.t("bar_ln_mode"), x = 1608, width = 248, act = props.event.LNMODE, index = props.index.LNMODE },
}
local FILTER_WORDS = {
    ALL = i18n.t("bar_all"),
    TITLE = i18n.t("bar_sort_title"),
    ARTIST = i18n.t("bar_sort_artist"),
    LENGTH = i18n.t("bar_sort_length"),
    LEVEL = i18n.t("bar_sort_level"),
    SCORE = i18n.t("bar_sort_score"),
}
local LN_MODE_NAMES = { [0] = i18n.t("common_long_note"), i18n.t("common_charge_note"), i18n.t("common_hell_charge_note") }
local NO_NAME = "-"

local KEY_MODE_OPTIONS = { props.option.KEYS_7, props.option.KEYS_5, props.option.KEYS_14, props.option.KEYS_10, props.option.KEYS_9 }
local PLAY_STYLES = {
    { name = "SP", ops = { props.option.KEYS_7, props.option.KEYS_5 } },
    { name = "DP", ops = { props.option.KEYS_14, props.option.KEYS_10 } },
    { name = "9KEYS", ops = { props.option.KEYS_9 } },
}
local PLAY_MODES = {
    { text = i18n.t("bar_free_play"), op = -props.option.GRADEBAR },
    { text = i18n.t("common_course"), op = props.option.GRADEBAR },
}
local CLOCK = { hour = 24, minute = 25, format = "%02d:%02d", unknown = "--:--" }

local HINT_KEYS = {
    { key = "ENTER", key_width = 76, label = "bar_hint_play", width = 64 },
    { key = "ESC", key_width = 52, label = "bar_hint_back", width = 64 },
    { key = "UP DOWN", key_width = 108, label = "bar_hint_move", width = 64 },
    { key = "H", key_width = 32, label = "bar_hint_all_keys" },
}
local HINT = { x = 64, right = 1048, keys_top = 1010, key_height = 26, key_gap = 8, item_gap = 24, line_top = 1046 }
local HINT_LINE = i18n.t("bar_hint_line")

local DOCK = { top = 1020, height = 40 }
local REPLAY = { label_x = 1080, x = 1150, cell = 40, gap = 4, underline = 2 }
local REPLAY_SLOTS = {
    { id = "btn.replay1", act = props.event.REPLAY, there = props.option.REPLAYDATA, chosen = props.option.SELECT_REPLAYDATA },
    { id = "btn.replay2", act = props.event.REPLAY2, there = props.option.REPLAYDATA2, chosen = props.option.SELECT_REPLAYDATA2 },
    { id = "btn.replay3", act = props.event.REPLAY3, there = props.option.REPLAYDATA3, chosen = props.option.SELECT_REPLAYDATA3 },
    { id = "btn.replay4", act = props.event.REPLAY4, there = props.option.REPLAYDATA4, chosen = props.option.SELECT_REPLAYDATA4 },
}
local ACTIONS = {
    { id = "btn.autoplay", label = "AUTOPLAY", x = 1340, width = 118, act = props.event.AUTOPLAY },
    { id = "btn.practice", label = i18n.t("bar_practice"), x = 1466, width = 118, act = props.event.PRACTICE },
    { id = "btn.ranking", label = i18n.t("bar_ranking"), x = 1592, width = 118, act = props.extension_event.RANKING_MODE, lit = props.extension.RANKING_MODE_ON },
    { id = "btn.favorite", label = i18n.t("bar_favorite"), x = 1718, width = 138, act = props.event.FAVORITE_CHART, star = true },
}
local STAR = { id = "set.star", inset = 12, gap = 8 }
local UNDERLINE = 2

local function chip(s, x, width)
    local box = { x = x, top = HEAD.chip.top, w = width, h = HEAD.chip.height }
    s:rect(box, theme.color.panel)
    return box
end

local function chip_text(s, spec, box, opts)
    local size = theme.type.label
    local placed = ui.merge(spec, { size = size, align = "center", overflow = props.text_overflow.SHRINK })
    s:text(placed, { x = box.x, top = theme.centered_top(box, size), w = box.w }, ui.merge(opts, { color = theme.color.text }))
end

local function play_style(s)
    local box = chip(s, HEAD.style_chip.x, HEAD.style_chip.width)
    for _, style in ipairs(PLAY_STYLES) do
        for _, op in ipairs(style.ops) do
            chip_text(s, { text = style.name }, box, { op = { op } })
        end
    end
    local none = {}
    for index, op in ipairs(KEY_MODE_OPTIONS) do
        none[index] = -op
    end
    chip_text(s, { text = NO_NAME }, box, { op = none })
end

local function play_mode(s)
    local box = chip(s, HEAD.mode_chip.x, HEAD.mode_chip.width)
    for _, mode in ipairs(PLAY_MODES) do
        chip_text(s, { value = mode.text }, box, { op = { mode.op } })
    end
end

local function clock(s, main_state)
    local box = chip(s, HEAD.clock.x, HEAD.clock.width)
    local value = function()
        local hour = main_state.number(CLOCK.hour)
        local minute = main_state.number(CLOCK.minute)
        if not ui.has_value(hour) or not ui.has_value(minute) then
            return CLOCK.unknown
        end
        return string.format(CLOCK.format, hour, minute)
    end
    chip_text(s, { value = value }, box)
end

local function search(s)
    local size = theme.type.caption
    s:text({ text = i18n.t("bar_search"), size = size }, { x = SEARCH.label_x, top = HEADER.caption_top, w = SEARCH.x - SEARCH.label_x }, { color = theme.color.muted })
    local field = { x = SEARCH.x, top = SEARCH.top, w = SEARCH.width, h = SEARCH.height }
    s:rect(field, theme.color.panel)
    s:rect({ x = field.x, top = field.top + field.h - theme.grid.rule, w = field.w, h = theme.grid.rule }, theme.color.line)
    local typed = { x = field.x + SEARCH.inset, top = HEADER.value_top, w = field.w - SEARCH.inset * 2 }
    s:text({ id = SEARCH.id, ref = props.text.SEARCHWORD, size = theme.type.label, overflow = props.text_overflow.SHRINK }, typed, { color = theme.color.text })
end

local function named(main_state, ref, unnamed)
    return function()
        local name = main_state.text(ref)
        if name == "" then
            return unnamed
        end
        local word = FILTER_WORDS[name]
        if word ~= nil then
            return word()
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
        if filter.ref then
            spec.value = named(main_state, filter.ref, filter.unnamed)
        else
            spec.value = parts.choice(main_state, filter.index, LN_MODE_NAMES, NO_NAME)
        end
        s:text(spec, { x = inner.x, top = HEADER.value_top, w = inner.w }, { color = theme.color.text })
    end
end

local function hints(s)
    local size = theme.type.caption
    local x = HINT.x
    for _, hint in ipairs(HINT_KEYS) do
        local key = { x = x, top = HINT.keys_top, w = hint.key_width, h = HINT.key_height }
        s:rect(key, theme.color.raised)
        local top = theme.centered_top(key, size)
        s:text({ text = hint.key, size = size, align = "center" }, { x = key.x, top = top, w = key.w }, { color = theme.color.text })
        local label_x = key.x + key.w + HINT.key_gap
        local width = hint.width or HINT.right - label_x
        s:text({ value = i18n.t(hint.label), size = size, overflow = props.text_overflow.SHRINK }, { x = label_x, top = top, w = width }, { color = theme.color.muted })
        x = label_x + width + HINT.item_gap
    end
    local line = { text = HINT_LINE, size = size, overflow = props.text_overflow.SHRINK }
    s:text(line, { x = HINT.x, top = HINT.line_top, w = HINT.right - HINT.x }, { color = theme.color.muted })
end

local function replays(s)
    local size = theme.type.caption
    local dock = { top = DOCK.top, h = DOCK.height }
    s:text({ text = i18n.t("common_replay"), size = size }, { x = REPLAY.label_x, top = theme.centered_top(dock, size), w = REPLAY.x - REPLAY.label_x }, { color = theme.color.muted })
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

local function lit_action(s, action, box, top)
    local spec = { value = action.label, size = theme.type.caption, align = "center", overflow = props.text_overflow.SHRINK }
    local at = { x = box.x, top = top, w = box.w }
    s:text(spec, at, { color = theme.color.sub, op = { -action.lit } })
    s:text(spec, at, { color = theme.color.text, op = { action.lit } })
    s:rect({ x = box.x, top = box.top + box.h - UNDERLINE, w = box.w, h = UNDERLINE }, theme.color.accent, { op = { action.lit } })
end

local function actions(s)
    local size = theme.type.caption
    for _, action in ipairs(ACTIONS) do
        local box = { x = action.x, top = DOCK.top, w = action.width, h = DOCK.height }
        kit.button(s, { id = action.id, act = action.act }, box)
        local top = theme.centered_top(box, size)
        if action.lit then
            lit_action(s, action, box, top)
        elseif action.star then
            local x = star(s, box)
            s:text({ text = action.label, size = size }, { x = x, top = top, w = box.x + box.w - x }, { color = theme.color.sub })
        else
            s:text({ text = action.label, size = size, align = "center" }, { x = box.x, top = top, w = box.w }, { color = theme.color.sub })
        end
    end
end

function controls.header(s, main_state)
    play_style(s)
    play_mode(s)
    search(s)
    clock(s, main_state)
    filters(s, main_state)
end

function controls.footer(s)
    hints(s)
    replays(s)
    actions(s)
end

return controls
