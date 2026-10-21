local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")

local panel = {}

local MOTION = { veil = 248, rise = 24, move = 150 }
local BODY_ALPHA = 236
local LINE = 2
local WIRE = 4
local HEADER = { height = 72, pitch = 30, rule = 2 }
local ROWS = { first = 8, pitch = 44, height = 38, inset = 10, foot = 6 }
local GLOW = 4
local LABEL_GAP = 8
local KEY = { frame = 4, label = theme.type.caption }
local FIELD = { height = 52, inset = 10, arrow = 20, arrow_inset = 14, gap = 12 }

panel.HEADER = HEADER
panel.ROWS = ROWS
panel.WIRE = WIRE

local Sheet = {}
Sheet.__index = Sheet

function panel.new(s, page)
    return setmetatable({ s = s, page = page }, Sheet)
end

function Sheet:rising(opts)
    local keys = { { time = 0, dtop = MOTION.rise, a = 0 }, { time = MOTION.move } }
    return ui.merge(opts, { timer = self.page.on, op = { self.page.option }, keys = keys, acc = props.acc.DECELERATE })
end

function Sheet:sinking(opts)
    local keys = { { time = 0 }, { time = MOTION.move, dtop = MOTION.rise, a = 0 } }
    return ui.merge(opts, { timer = self.page.off, keys = keys, acc = props.acc.ACCELERATE, loop = props.loop.ONCE })
end

function Sheet:both(id, box, opts)
    self.s:place(id, box, self:rising(opts))
    self.s:place(id, box, self:sinking(opts))
end

function Sheet:fill(box, color, opts)
    self:both(props.image.WHITE, box, ui.merge(opts, { color = color }))
end

function Sheet:write(spec, box, color, opts)
    self:both(self.s:define_text(spec), self.s:text_box(spec, box), ui.merge(opts, { color = color }))
end

function Sheet:outline(box, color, width, opts)
    local edge = width or LINE
    self:fill({ x = box.x, top = box.top, w = box.w, h = edge }, color, opts)
    self:fill({ x = box.x, top = box.top + box.h - edge, w = box.w, h = edge }, color, opts)
    self:fill({ x = box.x, top = box.top + edge, w = edge, h = box.h - edge * 2 }, color, opts)
    self:fill({ x = box.x + box.w - edge, top = box.top + edge, w = edge, h = box.h - edge * 2 }, color, opts)
end

function Sheet:veil()
    local screen = { x = 0, top = 0, w = theme.width, h = theme.height }
    local page = self.page
    local veiling = { timer = page.on, op = { page.option }, keys = { { time = 0, a = 0 }, { time = MOTION.move, a = MOTION.veil } } }
    local clearing = { timer = page.off, loop = props.loop.ONCE, keys = { { time = 0, a = MOTION.veil }, { time = MOTION.move, a = 0 } } }
    self.s:rect(screen, theme.color.bg, veiling)
    self.s:rect(screen, theme.color.bg, clearing)
end

function Sheet:wire(points, color)
    local half = WIRE / 2
    for index = 2, #points do
        local from, to = points[index - 1], points[index]
        local x = math.min(from[1], to[1]) - half
        local top = math.min(from[2], to[2]) - half
        local box = { x = x, top = top, w = math.abs(to[1] - from[1]) + WIRE, h = math.abs(to[2] - from[2]) + WIRE }
        self:fill(box, color)
    end
end

function Sheet:body(box, color)
    self:fill(box, theme.color.bg, { a = BODY_ALPHA })
    self:outline(box, color)
end

function Sheet:tag(box, text, fill, ink)
    self:fill(box, fill)
    local size = theme.type.caption
    local spec = { text = text, size = size, weight = "bold", align = "center", overflow = props.text_overflow.SHRINK }
    self:write(spec, { x = box.x, top = theme.centered_top(box, size), w = box.w }, ink)
end

function Sheet:key(box, label, color, black)
    local frame = KEY.frame
    self:fill({ x = box.x - frame, top = box.top - frame, w = box.w + frame * 2, h = box.h + frame * 2 }, color)
    local face = black and theme.panel.key_black or theme.panel.key_white
    local ink = black and theme.panel.key_black_ink or theme.panel.key_white_ink
    self:fill(box, face)
    local size = KEY.label
    local spec = { text = label, size = size, weight = "bold", align = "center", overflow = props.text_overflow.SHRINK }
    self:write(spec, { x = box.x, top = theme.centered_top(box, size), w = box.w }, ink)
end

function Sheet:heading(box, lines, color)
    local size = theme.type.label
    local first = box.top + (HEADER.height - theme.cap_height(size) - (#lines - 1) * HEADER.pitch) / 2
    for index, line in ipairs(lines) do
        local spec = { text = line, size = size, weight = "bold", align = "center", overflow = props.text_overflow.SHRINK }
        self:write(spec, { x = box.x + ROWS.inset, top = first + (index - 1) * HEADER.pitch, w = box.w - ROWS.inset * 2 }, theme.color.text)
    end
    self:fill({ x = box.x, top = box.top + HEADER.height, w = box.w, h = HEADER.rule }, color)
end

local CAPTION_PITCH = theme.type.caption + LABEL_GAP

local function lines_height(count)
    if count == 0 then
        return 0
    end
    return ROWS.first + count * CAPTION_PITCH
end

function panel.list_height(count, lines)
    return HEADER.height + ROWS.first + count * ROWS.pitch + ROWS.foot + lines_height(lines or 0)
end

function panel.field_height(lines)
    return HEADER.height + ROWS.first + FIELD.height + ROWS.foot + lines_height(lines or 0)
end

local function reserved(spec)
    return spec.reserve or (spec.lines and #spec.lines) or 0
end

function Sheet:cell(row, text, picked, missed)
    local size = theme.type.label
    self:fill(row, theme.color.raised)
    local glow = { x = row.x - GLOW, top = row.top - GLOW, w = row.w + GLOW * 2, h = row.h + GLOW * 2 }
    self:fill(glow, theme.panel.pick, { a = theme.panel.glow, draw = picked })
    self:fill(row, theme.panel.pick, { draw = picked })
    local line = { x = row.x + ROWS.inset, top = theme.centered_top(row, size), w = row.w - ROWS.inset * 2 }
    local plain = { size = size, align = "center", overflow = props.text_overflow.SHRINK }
    local strong = ui.merge(plain, { weight = "bold" })
    for key, value in pairs(text) do
        plain[key] = value
        strong[key] = value
    end
    self:write(plain, line, theme.color.sub, { draw = missed })
    self:write(strong, line, theme.panel.pick_ink, { draw = picked })
end

function Sheet:list(box, spec)
    local frame = { x = box.x, top = box.top, w = box.w, h = panel.list_height(#spec.values, reserved(spec)) }
    self:body(frame, spec.color)
    self:heading(frame, spec.name, spec.color)
    local rows_top = box.top + HEADER.height + ROWS.first
    for index, entry in ipairs(spec.values) do
        local row = { x = box.x + ROWS.inset, top = rows_top + (index - 1) * ROWS.pitch, w = box.w - ROWS.inset * 2, h = ROWS.height }
        self:cell(row, { text = entry.name }, spec.is(entry.value), spec.is_not(entry.value))
    end
    local lines = { x = box.x + ROWS.inset, top = rows_top + #spec.values * ROWS.pitch + ROWS.first, w = box.w - ROWS.inset * 2 }
    if spec.lines then
        self:caption_lines(lines, spec.lines, spec.line_color or theme.color.muted)
    end
    return frame, lines
end

function Sheet:column_label(frame, text, color)
    local size = theme.type.caption
    local spec = { text = text, size = size, weight = "bold", align = "center" }
    self:write(spec, { x = frame.x, top = frame.top + frame.h + LABEL_GAP, w = frame.w }, color)
end

function Sheet:caption_lines(box, lines, color, opts)
    local size = theme.type.caption
    local pitch = size + LABEL_GAP
    for index, line in ipairs(lines) do
        local spec = { text = line, size = size, align = box.align or "center", overflow = props.text_overflow.SHRINK }
        self:write(spec, { x = box.x, top = box.top + (index - 1) * pitch, w = box.w }, color, opts)
    end
    return box.top + #lines * pitch
end

function Sheet:field(box, spec)
    local frame = { x = box.x, top = box.top, w = box.w, h = panel.field_height(reserved(spec)) }
    self:body(frame, spec.color)
    self:heading(frame, spec.name, spec.color)
    local slot = { x = box.x + FIELD.inset, top = box.top + HEADER.height + ROWS.first, w = box.w - FIELD.inset * 2, h = FIELD.height }
    self:fill(slot, theme.color.raised)
    local size = theme.type.label
    local mark = { size = size, weight = "bold" }
    local mark_top = theme.centered_top(slot, size)
    self:write(ui.merge(mark, { text = "<", align = "left" }), { x = slot.x + FIELD.arrow_inset, top = mark_top, w = FIELD.arrow }, theme.panel.arrow)
    self:write(ui.merge(mark, { text = ">", align = "right" }), { x = slot.x + slot.w - FIELD.arrow_inset - FIELD.arrow, top = mark_top, w = FIELD.arrow }, theme.panel.arrow)
    local value_size = theme.type.body
    local value_top = theme.centered_top(slot, value_size)
    local lines = { x = box.x + ROWS.inset, top = slot.top + slot.h + ROWS.first + ROWS.foot, w = box.w - ROWS.inset * 2 }
    if spec.lines then
        self:caption_lines(lines, spec.lines, spec.line_color or theme.color.muted)
    end
    if spec.second == nil then
        local shown = { value = spec.first, size = value_size, weight = "bold", align = "center" }
        self:write(shown, { x = slot.x, top = value_top, w = slot.w }, theme.color.text)
        return frame, lines
    end
    local reach = FIELD.arrow_inset + FIELD.arrow + FIELD.gap
    local half = slot.w / 2 - FIELD.gap / 2 - reach
    local first = { value = spec.first, size = value_size, weight = "bold", align = "right", overflow = props.text_overflow.SHRINK }
    self:write(first, { x = slot.x + reach, top = value_top, w = half }, theme.color.text)
    local second = { value = spec.second, size = value_size, weight = "bold", align = "left", overflow = props.text_overflow.SHRINK }
    self:write(second, { x = slot.x + slot.w / 2 + FIELD.gap / 2, top = value_top, w = half }, theme.panel.green_number)
    return frame, lines
end

return panel
