local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local parts = require("lua.parts")
local kit = require("lua.select.kit")
local i18n = require("lua.i18n")

local side = {}

local CLOSED = 0
local SOUND = { open = "sound/o-open.wav", close = "sound/o-close.wav" }

local TAB = { x = 0, top = 444, width = 48, height = 96 }
local TABS = {
    { id = "btn.side.settings", icon = "icon_settings" },
    { id = "btn.side.volume", icon = "icon_volume" },
}

local PANEL = { x = 64, top = 128, width = 1032, height = 840, slide = 200, inset = 32, title_top = 168, rule_top = 216 }
local ROW = { first = 240, pitch = 72, height = 56, inset = 24 }
local SETTINGS = {
    { id = "btn.side.keyconfig", label = i18n.t("side_key_config"), act = props.event.KEYCONFIG, fixed = i18n.t("side_open") },
    { id = "btn.side.skin", label = i18n.t("side_skin"), act = props.event.SKINSELECT, fixed = i18n.t("side_open") },
    {
        id = "btn.side.lnmode",
        label = i18n.t("bar_ln_mode"),
        act = props.event.LNMODE,
        index = props.index.LNMODE,
        names = { [0] = i18n.t("common_long_note"), i18n.t("common_charge_note"), i18n.t("common_hell_charge_note") },
    },
    { id = "btn.side.bga", label = "BGA", act = props.event.BGA, index = props.index.BGA, names = { [0] = "ON", [2] = "OFF" } },
    {
        id = "btn.side.gaugeshift",
        label = i18n.t("sheet_gauge_auto_shift"),
        act = props.event.GAUGEAUTOSHIFT,
        index = props.index.GAUGEAUTOSHIFT,
        names = { [0] = "NONE", "CONTINUE", "SURVIVAL TO GROOVE", "BEST CLEAR", "SELECT TO UNDER" },
    },
    {
        id = "btn.side.algorithm",
        label = i18n.t("sheet_judge_algorithm"),
        act = props.event.JUDGEALGORITHM,
        index = props.index.JUDGEALGORITHM,
        names = { [0] = "COMBO", "DURATION", "LOWEST" },
        unlisted = i18n.t("side_score_elsewhere"),
    },
}
local NO_NAME = "-"
local SETTINGS_HINT = i18n.t("side_hint_settings")

local VOLUME = { first = 256, pitch = 128, track_drop = 52, track_height = 4, thumb_width = 12, thumb_height = 32, sprite = "num32", digits = 3, percent_gap = 6 }
local VOLUMES = {
    { label = i18n.t("side_master"), rate = props.rate.MASTER_VOLUME, number = props.number.MASTER_VOLUME },
    { label = i18n.t("side_key_sound"), rate = props.rate.KEY_VOLUME, number = props.number.KEY_VOLUME },
    { label = "BGM", rate = props.rate.BGM_VOLUME, number = props.number.BGM_VOLUME },
}
local VOLUME_HINT = i18n.t("side_hint_volume")
local PERCENT_WIDTH = 32

local function tab_box(index)
    return { x = TAB.x, top = TAB.top + (index - 1) * TAB.height, w = TAB.width, h = TAB.height }
end

local function switch(main_state, state, index)
    return function()
        if state.open == index then
            state.open = CLOSED
            main_state.audio_play(SOUND.close)
        else
            state.open = index
            main_state.audio_play(SOUND.open)
        end
    end
end

local function tabs(s, main_state, state, shown)
    for index, tab in ipairs(TABS) do
        local box = tab_box(index)
        kit.button(s, { id = tab.id, act = switch(main_state, state, index) }, box)
        local sprite = s:sprite(tab.icon)
        local icon = { x = box.x + (box.w - sprite.w) / 2, top = box.top + (box.h - sprite.h) / 2 }
        s:image(tab.icon, icon, { color = theme.color.muted })
        s:image(tab.icon, icon, { color = theme.color.accent, timer = shown[index] })
        s:rect({ x = box.x + box.w - theme.grid.rule, top = box.top, w = theme.grid.rule, h = box.h }, theme.color.accent, { timer = shown[index] })
    end
end

local function entering(timer, opts)
    local keys = { { time = 0, dx = -(PANEL.x + PANEL.width), a = 0 }, { time = PANEL.slide } }
    return ui.merge(opts, { timer = timer, keys = keys, acc = props.acc.DECELERATE })
end

local function shell(s, timer, title, hint)
    local box = { x = PANEL.x, top = PANEL.top, w = PANEL.width, h = PANEL.height }
    s:rect(box, theme.color.raised, entering(timer))
    s:rect({ x = box.x, top = box.top, w = box.w, h = theme.grid.rule }, theme.color.accent, entering(timer))
    local inner = { x = box.x + PANEL.inset, w = box.w - PANEL.inset * 2 }
    s:text({ text = title, size = theme.type.body }, { x = inner.x, top = PANEL.title_top, w = inner.w }, entering(timer, { color = theme.color.text }))
    s:rect({ x = inner.x, top = PANEL.rule_top, w = inner.w, h = theme.grid.rule }, theme.color.line, entering(timer))
    local size = theme.type.caption
    local foot = { x = inner.x, top = box.top + box.h - PANEL.inset - size, w = inner.w }
    s:text({ text = hint, size = size }, foot, entering(timer, { color = theme.color.muted }))
    return inner
end

local function settings(s, main_state, timer)
    local inner = shell(s, timer, i18n.t("side_quick_settings"), SETTINGS_HINT)
    local size = theme.type.label
    for index, row in ipairs(SETTINGS) do
        local box = { x = inner.x, top = ROW.first + (index - 1) * ROW.pitch, w = inner.w, h = ROW.height }
        kit.button(s, { id = row.id, act = row.act, lit = theme.color.line }, box, entering(timer))
        local text = { x = box.x + ROW.inset, top = theme.centered_top(box, size), w = box.w - ROW.inset * 2 }
        s:text({ text = row.label, size = size }, text, entering(timer, { color = theme.color.text }))
        local value = { size = size, align = "right" }
        if row.fixed then
            value.text = row.fixed
        else
            value.value = parts.choice(main_state, row.index, row.names, row.unlisted or NO_NAME)
        end
        s:text(value, text, entering(timer, { color = theme.color.accent }))
    end
end

local function volume(s, timer)
    local inner = shell(s, timer, i18n.t("side_volume"), VOLUME_HINT)
    local travel = inner.w - VOLUME.thumb_width
    for index, bus in ipairs(VOLUMES) do
        local top = VOLUME.first + (index - 1) * VOLUME.pitch
        s:text({ text = bus.label, size = theme.type.label }, { x = inner.x, top = top, w = inner.w }, entering(timer, { color = theme.color.text }))
        local size = theme.type.label
        local percent = { x = inner.x + inner.w - PERCENT_WIDTH, top = top, w = PERCENT_WIDTH }
        s:text({ text = "%", size = size, align = "right" }, percent, entering(timer, { color = theme.color.muted }))
        local number = { sprite = VOLUME.sprite, ref = bus.number, digit = VOLUME.digits, align = "right", size = size }
        local slot = { x = inner.x, top = top, w = inner.w - PERCENT_WIDTH - VOLUME.percent_gap }
        s:number(number, slot, entering(timer, { color = theme.color.accent }))

        local track = { x = inner.x, top = top + VOLUME.track_drop, w = inner.w, h = VOLUME.track_height }
        s:rect(track, theme.color.line, entering(timer))
        s:object("graph", kit.meter(s, bus.rate), { x = track.x, top = track.top, w = travel, h = track.h }, entering(timer, { color = theme.color.accent }))
        local handle = kit.slider(s, bus.rate, props.slider_angle.RIGHT, travel)
        local thumb = { x = track.x, top = track.top + (track.h - VOLUME.thumb_height) / 2, w = VOLUME.thumb_width, h = VOLUME.thumb_height }
        s:object("slider", handle, thumb, entering(timer, { color = theme.color.text }))
    end
end

function side.build(s, main_state)
    local timer_util = require("timer_util")
    local state = { open = CLOSED }
    local shown = {}
    for index = 1, #TABS do
        shown[index] = timer_util.timer_observe_boolean(function()
            return state.open == index
        end)
    end
    tabs(s, main_state, state, shown)
    settings(s, main_state, shown[1])
    volume(s, shown[2])
end

return side
