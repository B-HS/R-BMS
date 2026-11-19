local theme = require("lua.theme")
local props = require("lua.props")
local parts = require("lua.parts")
local ui = require("lua.ui")
local i18n = require("lua.i18n")
local kit = require("lua.select.kit")

local controls = {}

local HEAD = theme.select_layout.head
local CONTROL = HEAD.control
local FIT = props.text_overflow.SHRINK_CUT

local PLACE = {
    trailing = "%s*>%s*$",
    joint = "  ·  ",
    separator = " > ",
    elision = "… > ",
    character = "[\1-\127\194-\244][\128-\191]*",
    wide_bytes = 3,
    narrow_weight = 10,
    wide_weight = 16,
    budget = 420,
}
local SEARCH = { id = "txt.search", opens = "Search : '" }
local FILTERS = {
    { id = "btn.mode", name = i18n.t("bar_mode"), width = 176, name_width = 60, act = props.event.MODE, ref = props.text.MODE_FILTER, unnamed = "" },
    { id = "btn.sort", name = i18n.t("bar_sort"), width = 224, name_width = 56, act = props.event.SORT, ref = props.text.SORT, unnamed = "DEFAULT" },
    { id = "btn.lnmode", name = i18n.t("bar_ln_mode"), width = 184, name_width = 92, act = props.event.LNMODE, index = props.index.LNMODE },
}
local FILTER_WORDS = {
    ALL = i18n.t("bar_all"),
    TITLE = i18n.t("bar_sort_title"),
    ARTIST = i18n.t("bar_sort_artist"),
    LENGTH = i18n.t("bar_sort_length"),
    LEVEL = i18n.t("bar_sort_level"),
    SCORE = i18n.t("bar_sort_score"),
}
local LN_MODE_NAMES = { [0] = "LN", "CN", "HCN" }
local NO_NAME = "-"

local PLAY_STYLES = {
    { name = "SP", ops = { props.option.KEYS_7, props.option.KEYS_5 } },
    { name = "DP", ops = { props.option.KEYS_14, props.option.KEYS_10 } },
    { name = "9KEYS", ops = { props.option.KEYS_9 } },
}
local PLAY_MODES = { free = i18n.t("bar_free_play"), course = i18n.t("common_course") }
local SCREEN_NAME = i18n.t("bar_music_select")
local CLOCK = { hour = 24, minute = 25, format = "%02d:%02d", unknown = "--:--" }

local FOOT = theme.select_layout.foot
local HINT_KEYS = {
    { key = "ENTER", key_width = 76, label = "bar_hint_play" },
    { key = "ESC", key_width = 52, label = "bar_hint_back" },
    { key = "↑↓", key_width = 44, label = "bar_hint_move" },
    { key = "H", key_width = 32, label = "bar_hint_key_guide", last = true },
}
local REPLAY_SLOTS = {
    { id = "btn.replay1", act = props.event.REPLAY, there = props.option.REPLAYDATA, chosen = props.option.SELECT_REPLAYDATA },
    { id = "btn.replay2", act = props.event.REPLAY2, there = props.option.REPLAYDATA2, chosen = props.option.SELECT_REPLAYDATA2 },
    { id = "btn.replay3", act = props.event.REPLAY3, there = props.option.REPLAYDATA3, chosen = props.option.SELECT_REPLAYDATA3 },
    { id = "btn.replay4", act = props.event.REPLAY4, there = props.option.REPLAYDATA4, chosen = props.option.SELECT_REPLAYDATA4 },
}
local ACTIONS = {
    { id = "btn.autoplay", label = "AUTOPLAY", width = 108, act = props.event.AUTOPLAY },
    { id = "btn.practice", label = i18n.t("bar_practice"), width = 100, act = props.event.PRACTICE },
    { id = "btn.ranking", label = i18n.t("bar_ranking"), width = 96, act = props.extension_event.RANKING_MODE, lit = props.extension.RANKING_MODE_ON },
    { id = "btn.favorite", label = i18n.t("bar_favorite"), width = 136, act = props.event.FAVORITE_CHART, star = true },
}
local STAR = { id = "set.star", inset = 10, gap = 8 }
local UNDERLINE = 2

local function head_text_top(size)
    return theme.text_top(HEAD.baseline, size)
end

local function control_box(x, width)
    return { x = x, top = CONTROL.top, w = width, h = CONTROL.height }
end

local function play_style(main_state)
    for _, style in ipairs(PLAY_STYLES) do
        for _, op in ipairs(style.ops) do
            if main_state.option(op) then
                return style.name
            end
        end
    end
    return nil
end

local function screen_line(main_state)
    return function()
        local words = { SCREEN_NAME() }
        local style = play_style(main_state)
        if style ~= nil then
            table.insert(words, style)
        end
        local mode = main_state.option(props.option.GRADEBAR) and PLAY_MODES.course or PLAY_MODES.free
        table.insert(words, mode())
        return table.concat(words, PLACE.joint)
    end
end

local function weight_of(text)
    local weight = 0
    for character in text:gmatch(PLACE.character) do
        weight = weight + (#character >= PLACE.wide_bytes and PLACE.wide_weight or PLACE.narrow_weight)
    end
    return weight
end

local function folders_of(path)
    local folders = {}
    local from = 1
    while true do
        local at, to = path:find(PLACE.separator, from, true)
        if at == nil then
            table.insert(folders, path:sub(from))
            return folders
        end
        table.insert(folders, path:sub(from, at - 1))
        from = to + 1
    end
end

function controls.path_tail(path)
    local folders = folders_of(path)
    local first = 1
    while first < #folders do
        local shown = (first > 1 and PLACE.elision or "") .. table.concat(folders, PLACE.separator, first)
        if weight_of(shown) <= PLACE.budget then
            break
        end
        first = first + 1
    end
    if first == 1 then
        return path
    end
    return PLACE.elision .. table.concat(folders, PLACE.separator, first)
end

local function folder_path(main_state)
    return function()
        local path = main_state.text(props.text.DIRECTORY):gsub(PLACE.trailing, "")
        return controls.path_tail(path)
    end
end

local function place(s, main_state)
    local value_size = theme.type.label
    local value_top = head_text_top(value_size)
    local label_size = theme.type.caption
    local label_top = value_top - kit.SECTION.value_gap - theme.cap_height(label_size)
    local box = { x = HEAD.place.x, w = HEAD.place.width }
    s:text({ value = screen_line(main_state), size = label_size, overflow = FIT }, ui.merge(box, { top = label_top }), { color = theme.color.muted })
    s:text({ value = folder_path(main_state), size = value_size, overflow = FIT }, ui.merge(box, { top = value_top }), { color = theme.color.text })
end

local function searching(main_state, wanted)
    return function()
        local path = main_state.text(props.text.DIRECTORY)
        local found = string.sub(path, 1, #SEARCH.opens) == SEARCH.opens
        return found == wanted
    end
end

local function search(s, main_state)
    local spec = HEAD.search
    local field = control_box(spec.x, spec.width)
    local busy = { draw = searching(main_state, true) }
    local idle = { draw = searching(main_state, false) }
    s:rect(field, theme.color.panel, busy)
    local underline = { x = field.x, top = field.top + field.h - theme.grid.rule, w = field.w, h = theme.grid.rule }
    s:rect(underline, theme.color.line, idle)
    s:rect(underline, theme.color.accent, busy)

    local size = theme.type.caption
    local label = { text = i18n.t("bar_search"), size = size, align = "right", overflow = FIT }
    local label_box = { x = spec.label_x, top = head_text_top(size), w = spec.x - spec.label_gap - spec.label_x }
    s:text(label, label_box, ui.merge(idle, { color = theme.color.muted }))
    s:text(label, label_box, ui.merge(busy, { color = theme.color.sub }))

    local typed_size = theme.type.label
    local typed = { x = field.x + CONTROL.inset, top = head_text_top(typed_size), w = field.w - CONTROL.inset * 2 }
    s:text({ id = SEARCH.id, ref = props.text.SEARCHWORD, size = typed_size, overflow = props.text_overflow.SHRINK }, typed, { color = theme.color.text })
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
    local x = HEAD.chips.x
    for _, filter in ipairs(FILTERS) do
        local box = control_box(x, filter.width)
        kit.button(s, { id = filter.id, act = filter.act }, box)
        local name_size = theme.type.caption
        local name_box = { x = box.x + CONTROL.inset, top = head_text_top(name_size), w = filter.name_width }
        s:text({ text = filter.name, size = name_size, overflow = FIT }, name_box, { color = theme.color.muted })
        local value_x = name_box.x + name_box.w + HEAD.chips.name_gap
        local value_size = theme.type.label
        local spec = { size = value_size, align = "right", overflow = FIT }
        if filter.ref then
            spec.value = named(main_state, filter.ref, filter.unnamed)
        else
            spec.value = parts.choice(main_state, filter.index, LN_MODE_NAMES, NO_NAME)
        end
        local value_box = { x = value_x, top = head_text_top(value_size), w = box.x + box.w - CONTROL.inset - value_x }
        s:text(spec, value_box, { color = theme.color.text })
        x = box.x + box.w + CONTROL.gap
    end
end

local function clock(s, main_state)
    local value = function()
        local hour = main_state.number(CLOCK.hour)
        local minute = main_state.number(CLOCK.minute)
        if not ui.has_value(hour) or not ui.has_value(minute) then
            return CLOCK.unknown
        end
        return string.format(CLOCK.format, hour, minute)
    end
    local size = theme.type.label
    local box = { x = HEAD.clock.x, top = head_text_top(size), w = HEAD.clock.width }
    s:text({ value = value, size = size, align = "right" }, box, { color = theme.color.sub })
end

local function foot_box(x, width, height)
    return { x = x, top = FOOT.center - height / 2, w = width, h = height }
end

local function hints(s)
    local spec = FOOT.hints
    local size = theme.type.caption
    local x = spec.x
    for _, hint in ipairs(HINT_KEYS) do
        local key = foot_box(x, hint.key_width, FOOT.key_height)
        s:rect(key, theme.color.raised)
        local top = theme.centered_top(key, size)
        s:text({ text = hint.key, size = size, align = "center" }, { x = key.x, top = top, w = key.w }, { color = theme.color.text })
        local label_x = key.x + key.w + spec.key_gap
        local width = hint.last and spec.right - label_x or spec.label_width
        s:text({ value = i18n.t(hint.label), size = size, overflow = FIT }, { x = label_x, top = top, w = width }, { color = theme.color.sub })
        x = label_x + width + spec.item_gap
    end
end

local function replays(s)
    local spec = FOOT.replay
    local size = theme.type.caption
    local row = foot_box(spec.x, spec.label_width, FOOT.button_height)
    local label = { text = i18n.t("common_replay"), size = size, overflow = FIT }
    s:text(label, { x = row.x, top = theme.centered_top(row, size), w = row.w }, { color = theme.color.muted })
    local first = spec.x + spec.label_width
    for index, slot in ipairs(REPLAY_SLOTS) do
        local box = foot_box(first + (index - 1) * (spec.cell + spec.gap), spec.cell, FOOT.button_height)
        kit.button(s, { id = slot.id, act = slot.act }, box)
        local digit = { text = tostring(index), size = theme.type.label, align = "center" }
        local at = { x = box.x, top = theme.centered_top(box, theme.type.label), w = box.w }
        s:text(digit, at, { color = theme.color.muted, op = { -slot.there } })
        s:text(digit, at, { color = theme.color.text, op = { slot.there } })
        local underline = { x = box.x, top = box.top + box.h - UNDERLINE, w = box.w, h = UNDERLINE }
        s:rect(underline, theme.color.accent, { op = { slot.chosen } })
    end
    return first + #REPLAY_SLOTS * (spec.cell + spec.gap) - spec.gap
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
    local spec = { value = action.label, size = theme.type.caption, align = "center", overflow = FIT }
    local at = { x = box.x, top = top, w = box.w }
    s:text(spec, at, { color = theme.color.sub, op = { -action.lit } })
    s:text(spec, at, { color = theme.color.text, op = { action.lit } })
    s:rect({ x = box.x, top = box.top + box.h - UNDERLINE, w = box.w, h = UNDERLINE }, theme.color.accent, { op = { action.lit } })
end

local function actions(s, after_replays)
    local size = theme.type.caption
    local x = after_replays + FOOT.actions.group_gap
    for _, action in ipairs(ACTIONS) do
        local box = foot_box(x, action.width, FOOT.button_height)
        kit.button(s, { id = action.id, act = action.act }, box)
        local top = theme.centered_top(box, size)
        if action.lit then
            lit_action(s, action, box, top)
        elseif action.star then
            local text_x = star(s, box)
            s:text({ text = action.label, size = size, overflow = FIT }, { x = text_x, top = top, w = box.x + box.w - STAR.inset - text_x }, { color = theme.color.sub })
        else
            s:text({ text = action.label, size = size, align = "center", overflow = FIT }, { x = box.x, top = top, w = box.w }, { color = theme.color.sub })
        end
        x = box.x + box.w + FOOT.actions.gap
    end
end

function controls.header(s, main_state)
    place(s, main_state)
    search(s, main_state)
    filters(s, main_state)
    clock(s, main_state)
end

function controls.footer(s)
    hints(s)
    actions(s, replays(s))
end

return controls
