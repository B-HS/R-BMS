local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local parts = require("lua.parts")

local sheet = {}

local FRAME = { x = 64, top = 312, width = 1792, height = 456, veil = 216, rise = 24, move = 150 }
local TITLE = { x = 96, top = 344, width = 1728, hint_top = 350, rule_top = 400 }
local CELL = { first = 96, pitch = 248, width = 224, key_top = 432, name_top = 464, value_top = 508, action_top = 572 }
local EXTRA = { rule_top = 620, key_top = 648, name_top = 680, value_top = 716 }
local UNAVAILABLE = "NOT AVAILABLE"
local NO_NAME = "-"
local HELD = {
    off = "OFF",
    off_action = "KEEP GREEN NUMBER IS OFF",
    note = "NOTE DISPLAY TIME MOVES WHILE KEEP GREEN NUMBER IS ON (PLAY MENU). ITS GREEN NUMBER IS THE ONE EVERY CHART STARTS AT",
}

local DOUBLE_NAMES = { [0] = "OFF", "FLIP", "BATTLE", "BATTLE AUTO-SC" }
local FIX_NAMES = { [0] = "OFF", "START BPM", "MAX BPM", "MAIN BPM", "MIN BPM" }
local SWITCH_NAMES = { [0] = "OFF", "ON" }
local BGA_NAMES = { [0] = "ON", "AUTOPLAY ONLY", "OFF" }
local SHIFT_NAMES = { [0] = "NONE", "CONTINUE", "SURVIVAL TO GROOVE", "BEST CLEAR", "SELECT TO UNDER" }

local function judge_timing(main_state)
    return function()
        local value = main_state.number(props.number.JUDGETIMING)
        if not ui.has_value(value) then
            return NO_NAME
        end
        if value == 0 then
            return "0 MS"
        end
        return string.format("%+d MS", value)
    end
end

local function display_time(main_state, action)
    local held = function()
        local duration = main_state.number(props.number.DURATION)
        local green = main_state.number(props.number.DURATION_GREEN)
        if ui.has_value(duration) and ui.has_value(green) then
            return duration, green
        end
    end
    local value = function()
        local duration, green = held()
        if duration == nil then
            return HELD.off
        end
        return string.format("%d MS", duration)
    end
    local doing = function()
        local duration, green = held()
        if duration == nil then
            return HELD.off_action
        end
        return string.format("GREEN %d   %s", green, action)
    end
    return { name = "NOTE DISPLAY TIME", value = value, action = doing }
end

local function sheets(main_state)
    local function picked(index, names)
        return parts.choice(main_state, index, names, NO_NAME)
    end
    local random = picked(props.index.RANDOM_1P, theme.random_name)
    local timing = judge_timing(main_state)
    return {
        {
            title = "PLAY OPTIONS",
            hold = "HOLD START",
            option = props.option.PANEL1,
            on = props.timer.PANEL1_ON,
            off = props.timer.PANEL1_OFF,
            cells = {
                { name = "RANDOM", value = random, action = "NEXT" },
                { name = "RANDOM", value = random, action = "PREVIOUS" },
                { name = "GAUGE", value = picked(props.index.GAUGE_1P, theme.gauge_name), action = "NEXT" },
                { name = "DP OPTION", value = picked(props.index.DPOPTION, DOUBLE_NAMES), action = "NEXT" },
                { name = "HI-SPEED FIX", value = picked(props.index.HSFIX, FIX_NAMES), action = "NEXT" },
                { name = "RANDOM", value = random, action = "PREVIOUS" },
                { name = "RANDOM", value = random, action = "NEXT" },
            },
            extra = { key = "TURNTABLE / BUTTONS 8-9", name = "TARGET", ref = props.text.TARGET_NAME },
        },
        {
            title = "ASSIST OPTIONS",
            hold = "HOLD SELECT",
            option = props.option.PANEL2,
            on = props.timer.PANEL2_ON,
            off = props.timer.PANEL2_OFF,
            cells = {
                { name = "EX JUDGE", value = picked(props.index.ASSIST_EXJUDGE, SWITCH_NAMES), action = "SET IN SETTINGS" },
                { name = "CONSTANT", value = picked(props.index.ASSIST_CONSTANT, SWITCH_NAMES), action = "SWITCH" },
                { name = "JUDGE AREA" },
                { name = "LEGACY NOTE", value = picked(props.index.ASSIST_LEGACY, SWITCH_NAMES), action = "SWITCH" },
                { name = "MARK NOTE" },
                { name = "BPM GUIDE" },
                { name = "NO MINE" },
            },
            note = "AN OPTION MARKED NOT AVAILABLE HAS NO SETTING IN R-BMS",
        },
        {
            title = "DETAIL OPTIONS",
            hold = "HOLD START AND SELECT, OR THE DETAIL OPTIONS KEY (5 AS SHIPPED)",
            option = props.option.PANEL3,
            on = props.timer.PANEL3_ON,
            off = props.timer.PANEL3_OFF,
            cells = {
                { name = "BGA", value = picked(props.index.BGA, BGA_NAMES), action = "NEXT" },
                { name = "GAUGE AUTO SHIFT", value = picked(props.index.GAUGEAUTOSHIFT, SHIFT_NAMES), action = "NEXT" },
                { name = "TIMING AUTO ADJUST", value = picked(props.index.TIMING_AUTO_ADJUST, SWITCH_NAMES), action = "SWITCH" },
                display_time(main_state, "1 MS SHORTER"),
                { name = "JUDGE TIMING", value = timing, action = "NOTES 1 MS LATER" },
                display_time(main_state, "1 MS LONGER"),
                { name = "JUDGE TIMING", value = timing, action = "NOTES 1 MS EARLIER" },
            },
            note = HELD.note,
        },
    }
end

local function rising(page, opts)
    local keys = { { time = 0, dtop = FRAME.rise, a = 0 }, { time = FRAME.move } }
    return ui.merge(opts, { timer = page.on, op = { page.option }, keys = keys, acc = props.acc.DECELERATE })
end

local function sinking(page, opts)
    local keys = { { time = 0 }, { time = FRAME.move, dtop = FRAME.rise, a = 0 } }
    return ui.merge(opts, { timer = page.off, keys = keys, acc = props.acc.ACCELERATE, loop = props.loop.ONCE })
end

local function both(s, page, id, box, opts)
    s:place(id, box, rising(page, opts))
    s:place(id, box, sinking(page, opts))
end

local function fill(s, page, box, color)
    both(s, page, props.image.WHITE, box, { color = color })
end

local function write(s, page, spec, box, color)
    both(s, page, s:define_text(spec), s:text_box(spec, box), { color = color })
end

local function backdrop(s, page)
    local screen = { x = 0, top = 0, w = theme.width, h = theme.height }
    local box = { x = FRAME.x, top = FRAME.top, w = FRAME.width, h = FRAME.height }
    local veiling = { timer = page.on, op = { page.option }, keys = { { time = 0, a = 0 }, { time = FRAME.move, a = FRAME.veil } } }
    local clearing = { timer = page.off, loop = props.loop.ONCE, keys = { { time = 0, a = FRAME.veil }, { time = FRAME.move, a = 0 } } }
    s:rect(screen, theme.color.bg, veiling)
    s:rect(screen, theme.color.bg, clearing)
    fill(s, page, box, theme.color.raised)
    fill(s, page, { x = box.x, top = box.top, w = box.w, h = theme.grid.rule }, theme.color.accent)
end

local function heading(s, page)
    write(s, page, { text = page.title, size = theme.type.body }, { x = TITLE.x, top = TITLE.top, w = TITLE.width }, theme.color.text)
    local hold = { text = page.hold, size = theme.type.caption, align = "right" }
    write(s, page, hold, { x = TITLE.x, top = TITLE.hint_top, w = TITLE.width }, theme.color.muted)
    fill(s, page, { x = TITLE.x, top = TITLE.rule_top, w = TITLE.width, h = theme.grid.rule }, theme.color.line)
end

local function cell(s, page, index, entry)
    local x = CELL.first + (index - 1) * CELL.pitch
    local caption = theme.type.caption
    write(s, page, { text = "KEY " .. index, size = caption }, { x = x, top = CELL.key_top, w = CELL.width }, theme.color.accent)
    local name = { text = entry.name, size = theme.type.label, overflow = props.text_overflow.SHRINK }
    local value = { x = x, top = CELL.value_top, w = CELL.width }
    if entry.value == nil then
        write(s, page, name, { x = x, top = CELL.name_top, w = CELL.width }, theme.color.muted)
        write(s, page, { text = UNAVAILABLE, size = caption }, value, theme.color.muted)
        return
    end
    write(s, page, name, { x = x, top = CELL.name_top, w = CELL.width }, theme.color.sub)
    local shown = { value = entry.value, size = theme.type.body, overflow = props.text_overflow.SHRINK }
    write(s, page, shown, value, theme.color.text)
    local action = { size = caption, overflow = props.text_overflow.SHRINK }
    if type(entry.action) == "function" then
        action.value = entry.action
    else
        action.text = entry.action
    end
    write(s, page, action, { x = x, top = CELL.action_top, w = CELL.width }, theme.color.muted)
end

local function footing(s, page)
    local caption = theme.type.caption
    fill(s, page, { x = TITLE.x, top = EXTRA.rule_top, w = TITLE.width, h = theme.grid.rule }, theme.color.line)
    if page.note then
        write(s, page, { text = page.note, size = caption }, { x = TITLE.x, top = EXTRA.key_top, w = TITLE.width }, theme.color.muted)
        return
    end
    local extra = page.extra
    write(s, page, { text = extra.key, size = caption }, { x = TITLE.x, top = EXTRA.key_top, w = TITLE.width }, theme.color.accent)
    write(s, page, { text = extra.name, size = theme.type.label }, { x = TITLE.x, top = EXTRA.name_top, w = TITLE.width }, theme.color.sub)
    local value = { ref = extra.ref, size = theme.type.body, overflow = props.text_overflow.SHRINK }
    write(s, page, value, { x = TITLE.x, top = EXTRA.value_top, w = TITLE.width }, theme.color.text)
end

function sheet.build(s, main_state)
    for _, page in ipairs(sheets(main_state)) do
        backdrop(s, page)
        heading(s, page)
        for index, entry in ipairs(page.cells) do
            cell(s, page, index, entry)
        end
        footing(s, page)
    end
end

return sheet
