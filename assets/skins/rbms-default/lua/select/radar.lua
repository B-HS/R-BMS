local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local i18n = require("lua.i18n")
local kit = require("lua.select.kit")

local radar = {}

local ON_CHART = props.option.SONGBAR
local NOTHING = "-"

local HALF_TURN = 180
local AXIS_STEP = 60
local FIRST_AXIS_ANGLE = -90
local EDGE_TO_SIDE = math.sqrt(3) / 2

local LAYOUT = {
    title_drop = 12,
    label_size = theme.type.caption,
    value_sprite = "num18",
    value_digits = 3,
    line_gap = 6,
    side_label_lift = 41,
    side_label_drop = 12,
    center_split = 4,
    star_side = 16,
}

local AXIS_LABEL_KEYS = {
    notes = "info_radar_notes",
    peak = "info_radar_peak",
    scratch = "info_radar_scratch",
    soflan = "info_radar_soflan",
    charge = "info_radar_charge",
    chord = "info_radar_chord",
}

local function vertex(index)
    local angle = math.rad(FIRST_AXIS_ANGLE + (index - 1) * AXIS_STEP)
    local center = theme.radar.center
    return center.x + theme.radar.radius * math.cos(angle), center.top + theme.radar.radius * math.sin(angle)
end

local function value_ref(index)
    return props.extension_number.NOTES_RADAR_FIRST + index - 1
end

local function present(main_state)
    return function()
        return ui.has_value(main_state.number(props.extension_number.NOTES_RADAR_FIRST))
    end
end

local function absent(main_state)
    return function()
        return not ui.has_value(main_state.number(props.extension_number.NOTES_RADAR_FIRST))
    end
end

local function largest(main_state, index)
    return function()
        local mine = main_state.number(value_ref(index))
        if not ui.has_value(mine) or mine <= 0 then
            return false
        end
        for other = 1, #theme.radar.axis_order do
            local theirs = main_state.number(value_ref(other))
            if ui.has_value(theirs) and (theirs > mine or (theirs == mine and other < index)) then
                return false
            end
        end
        return true
    end
end

local function line(s, center_x, center_top, length, thickness, angle, color, when)
    local box = { x = center_x - length / 2, top = center_top - thickness / 2, w = length, h = thickness }
    s:rect(box, color, ui.merge(when, { angle = angle }))
end

local function grid(s, when)
    local center = theme.radar.center
    local radius = theme.radar.radius
    for axis = 0, 2 do
        local angle = FIRST_AXIS_ANGLE + axis * AXIS_STEP
        line(s, center.x, center.top, radius * 2, theme.radar.axis_line, -angle, theme.radar.axis_color, when)
    end
end

local function outline(s, when)
    local center = theme.radar.center
    local radius = theme.radar.radius
    for edge = 1, #theme.radar.axis_order do
        local middle = math.rad(FIRST_AXIS_ANGLE + (edge - 1) * AXIS_STEP + AXIS_STEP / 2)
        local x = center.x + radius * EDGE_TO_SIDE * math.cos(middle)
        local top = center.top + radius * EDGE_TO_SIDE * math.sin(middle)
        local along = FIRST_AXIS_ANGLE + (edge - 1) * AXIS_STEP + AXIS_STEP / 2 + HALF_TURN / 2
        line(s, x, top, radius + theme.radar.outline, theme.radar.outline, -along, theme.radar.outline_color, when)
    end
end

local function polygon(s, when)
    local center = theme.radar.center
    local side = theme.radar.side
    local box = { x = center.x - side / 2, top = center.top - side / 2, w = side, h = side }
    s:reference(props.extension_image.NOTES_RADAR, box, kit.arriving(ui.merge(when, { color = theme.radar.fill, a = theme.radar.fill_alpha })))
end

local function label_block(index)
    local x, top = vertex(index)
    local area = theme.radar.area
    local gap = theme.radar.label_gap
    local cap = theme.cap_height(LAYOUT.label_size)
    if index == 1 then
        return { kind = "centered", x = x, top = top - gap - cap }
    end
    if index == 4 then
        return { kind = "centered", x = x, top = top + gap }
    end
    local lifted = (index == 2 or index == 6)
    local block_top = lifted and top - LAYOUT.side_label_lift or top + LAYOUT.side_label_drop
    if index <= 3 then
        return { kind = "right", x = area.x + area.width, top = block_top }
    end
    return { kind = "left", x = area.x, top = block_top }
end

local function axis_label(s, index, key, color, when)
    local block = label_block(index)
    local area = theme.radar.area
    local size = LAYOUT.label_size
    local spec = { value = i18n.t(AXIS_LABEL_KEYS[key]), size = size, overflow = props.text_overflow.OVERFLOW }
    local number = { sprite = LAYOUT.value_sprite, ref = value_ref(index), digit = LAYOUT.value_digits }
    local labelled = ui.merge(when, { color = color })
    local valued = kit.arriving(ui.merge(when, { color = theme.color.text }))
    if block.kind == "centered" then
        local left = { x = area.x, top = block.top, w = block.x - LAYOUT.center_split - area.x }
        s:text(ui.merge(spec, { align = "right" }), left, labelled)
        local baseline = block.top + theme.cap_height(size)
        s:number(ui.merge(number, { align = "left" }), { x = block.x + LAYOUT.center_split, baseline = baseline, w = area.width / 2 }, valued)
        return
    end
    local width = area.width / 2
    local box = { x = block.kind == "right" and block.x - width or block.x, top = block.top, w = width }
    s:text(ui.merge(spec, { align = block.kind }), box, labelled)
    local sprite = s:sprite(LAYOUT.value_sprite)
    local baseline = block.top + theme.cap_height(size) + LAYOUT.line_gap + sprite.cell_h - sprite.pad * 2
    s:number(ui.merge(number, { align = block.kind }), { x = box.x, baseline = baseline, w = box.w }, valued)
end

local function star(s, main_state, index, when)
    local x, top = vertex(index)
    local side = LAYOUT.star_side
    local box = { x = x - side / 2, top = top - side / 2, w = side, h = side }
    s:image("star_on", box, kit.arriving(ui.merge(when, { color = theme.radar.star, draw = largest(main_state, index) })))
end

function radar.build(s, main_state)
    local area = theme.radar.area
    local on = { op = { ON_CHART } }
    s:rect({ x = area.x, top = area.top, w = area.width, h = theme.grid.rule }, theme.color.line, on)
    local title = { value = i18n.t("info_notes_radar"), size = theme.type.caption }
    s:text(title, { x = area.x, top = area.top + LAYOUT.title_drop, w = area.width }, { color = theme.color.sub, op = { ON_CHART } })

    local shown = { op = { ON_CHART }, draw = present(main_state) }
    grid(s, shown)
    polygon(s, shown)
    outline(s, shown)
    for index, key in ipairs(theme.radar.axis_order) do
        axis_label(s, index, key, theme.radar.axis[key], shown)
        star(s, main_state, index, shown)
    end

    local size = theme.type.heading
    local center = theme.radar.center
    local dash = { x = area.x, top = center.top - theme.cap_height(size) / 2, w = area.width }
    s:text({ text = NOTHING, size = size, align = "center" }, dash, { color = theme.color.muted, op = { ON_CHART }, draw = absent(main_state) })
end

return radar
