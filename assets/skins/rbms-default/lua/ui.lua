local theme = require("lua.theme")
local props = require("lua.props")

local ui = {}

local Builder = {}
Builder.__index = Builder

local OPAQUE = 255
local FILTER_LINEAR = 1
local DIGIT_BASE = 10
local SIGNED_STRIP_ROWS = 2
local ATLAS_MODULE_PREFIX = "lua.atlas."
local OBJECT_LISTS = {
    "source",
    "font",
    "image",
    "imageset",
    "value",
    "floatvalue",
    "text",
    "slider",
    "graph",
    "gaugegraph",
    "judgegraph",
    "bpmgraph",
    "hiterrorvisualizer",
    "timingvisualizer",
    "timingdistributiongraph",
    "hiddenCover",
    "liftCover",
    "judge",
    "destination",
}
local TEXT_ALIGN = { left = props.text_align.LEFT, center = props.text_align.CENTER, right = props.text_align.RIGHT }
local NUMBER_ALIGN = { left = props.number_align.LEFT, center = props.number_align.CENTER, right = props.number_align.RIGHT }
local DESTINATION_FIELDS = { "timer", "loop", "op", "draw", "blend", "filter", "center", "offset", "offsets", "stretch" }
local IMAGE_FIELDS = { "timer", "cycle", "len", "ref", "act", "click" }

local function round(value)
    return math.floor(value + 0.5)
end

function ui.merge(opts, overrides)
    local result = {}
    for key, value in pairs(opts or {}) do
        result[key] = value
    end
    for key, value in pairs(overrides or {}) do
        result[key] = value
    end
    return result
end

local function has_value(value)
    return value ~= nil and value ~= props.no_value.LOW and value ~= props.no_value.HIGH
end

local function digits_of(value)
    if not has_value(value) then
        return "0"
    end
    return tostring(math.floor(math.abs(value)))
end

function ui.digit_count(value)
    return #digits_of(value)
end

function ui.box(x, top, w, h)
    return { x = x, top = top, w = w, h = h }
end

function ui.inset(box, left, top, right, bottom)
    local t = top or left
    local r = right or left
    local b = bottom or t
    return { x = box.x + left, top = box.top + t, w = box.w - left - r, h = box.h - t - b }
end

local function filtered(opts)
    local result = ui.merge(opts)
    if result.filter == nil then
        result.filter = FILTER_LINEAR
    end
    return result
end

local function frames_of(box, opts, parent_height)
    local base_color = opts.color or theme.color.white
    local base_alpha = opts.a or OPAQUE
    local keys = opts.keys or { { time = 0 } }
    local frames = {}
    for index, key in ipairs(keys) do
        local w = (key.w or box.w) + (key.dw or 0)
        local h = (key.h or box.h) + (key.dh or 0)
        local x = (key.x or box.x) + (key.dx or 0)
        local top = (key.top or box.top) + (key.dtop or 0)
        local color = key.color or base_color
        local frame = {
            time = key.time or 0,
            x = round(x),
            y = round(parent_height - top - h),
            w = round(w),
            h = round(h),
            a = round(key.a or base_alpha),
            r = color[1],
            g = color[2],
            b = color[3],
            angle = key.angle or opts.angle or 0,
        }
        if index == 1 and opts.acc then
            frame.acc = opts.acc
        end
        frames[index] = frame
    end
    return frames
end

function ui.new(header)
    local skin = {}
    for key, value in pairs(header) do
        skin[key] = value
    end
    for _, list in ipairs(OBJECT_LISTS) do
        skin[list] = {}
    end
    skin.font[1] = { id = theme.font.id, path = theme.font.path }
    return setmetatable({ skin = skin, sprites = {}, defined = {}, serial = 0 }, Builder)
end

function Builder:next_id(kind)
    self.serial = self.serial + 1
    return kind .. ".#" .. self.serial
end

function Builder:atlas(name)
    local atlas = require(ATLAS_MODULE_PREFIX .. name)
    table.insert(self.skin.source, { id = name, path = atlas.path })
    for sprite_name, sprite in pairs(atlas.sprites) do
        self.sprites[sprite_name] = {
            src = name,
            x = sprite.x,
            y = sprite.y,
            w = sprite.w,
            h = sprite.h,
            divx = sprite.divx,
            divy = sprite.divy,
            cell_w = sprite.w / sprite.divx,
            cell_h = sprite.h / sprite.divy,
            pad = sprite.pad or 0,
            size = sprite.size,
            ink = sprite.ink,
        }
    end
    return atlas
end

function Builder:sprite(name)
    local sprite = self.sprites[name]
    if sprite == nil then
        error("unknown sprite: " .. tostring(name), 2)
    end
    return sprite
end

function Builder:dst(id, box, opts, parent)
    opts = opts or {}
    local destination = { id = id }
    for _, field in ipairs(DESTINATION_FIELDS) do
        destination[field] = opts[field]
    end
    if box == nil then
        return destination
    end
    local frames = frames_of(box, opts, parent and parent.h or theme.height)
    destination.dst = frames
    if destination.loop == nil and #frames > 1 then
        destination.loop = frames[#frames].time
    end
    if opts.hover then
        local hover = opts.hover
        destination.mouseRect = { x = hover.x, y = box.h - hover.top - hover.h, w = hover.w, h = hover.h }
    end
    return destination
end

function Builder:place(id, box, opts)
    local destination = self:dst(id, box, opts)
    table.insert(self.skin.destination, destination)
    return destination
end

function Builder:rect(box, color, opts)
    return self:place(props.image.WHITE, box, ui.merge(opts, { color = color }))
end

function Builder:reference(image_id, box, opts)
    return self:place(image_id, box, opts)
end

function Builder:define_image(sprite_name, def)
    def = def or {}
    local id = def.id or ("img." .. sprite_name)
    if self.defined[id] then
        return id
    end
    local sprite = self:sprite(sprite_name)
    local image = { id = id, src = sprite.src, x = sprite.x, y = sprite.y, w = sprite.w, h = sprite.h, divx = sprite.divx, divy = sprite.divy }
    for _, field in ipairs(IMAGE_FIELDS) do
        image[field] = def[field]
    end
    table.insert(self.skin.image, image)
    self.defined[id] = true
    return id
end

function Builder:image(sprite_name, box, opts)
    opts = opts or {}
    local id = self:define_image(sprite_name, opts.def)
    local sprite = self:sprite(sprite_name)
    local placed = { x = box.x, top = box.top, w = box.w or sprite.cell_w, h = box.h or sprite.cell_h }
    return self:place(id, placed, filtered(opts))
end

function Builder:define_text(spec)
    local id = spec.id or self:next_id("txt")
    local text = {
        id = id,
        font = theme.font.id,
        size = spec.size,
        align = TEXT_ALIGN[spec.align or "left"],
        overflow = spec.overflow or props.text_overflow.OVERFLOW,
        wrapping = spec.wrap or false,
    }
    if spec.value ~= nil then
        text.value = spec.value
    elseif spec.ref ~= nil then
        text.ref = spec.ref
    else
        text.constantText = spec.text or ""
    end
    table.insert(self.skin.text, text)
    return id
end

function Builder:text_box(spec, box)
    local align = spec.align or "left"
    local anchor = box.x
    if align == "center" then
        anchor = box.x + box.w / 2
    elseif align == "right" then
        anchor = box.x + box.w
    end
    return { x = anchor, top = box.top, w = box.w, h = box.h or spec.size }
end

function Builder:text(spec, box, opts)
    return self:place(self:define_text(spec), self:text_box(spec, box), opts)
end

local function saturating(ref, places)
    local main_state = require("main_state")
    local most = DIGIT_BASE ^ places - 1
    return function()
        local value = main_state.number(ref)
        if not has_value(value) then
            return value
        end
        return math.max(-most, math.min(most, value))
    end
end

function Builder:define_number(spec, digits)
    local id = spec.id or self:next_id("num")
    local sprite = self:sprite(spec.sprite)
    local value = {
        id = id,
        src = sprite.src,
        x = sprite.x,
        y = sprite.y,
        w = sprite.w,
        h = sprite.h,
        divx = sprite.divx,
        divy = sprite.divy,
        digit = digits,
        align = NUMBER_ALIGN[spec.align or "left"],
        padding = spec.padding or 0,
        zeropadding = spec.zeropadding or 0,
        space = spec.space or 0,
    }
    if spec.value ~= nil then
        value.value = spec.value
    elseif spec.saturate then
        value.value = saturating(spec.ref, sprite.divy == SIGNED_STRIP_ROWS and digits - 1 or digits)
    else
        value.ref = spec.ref
    end
    table.insert(self.skin.value, value)
    return id
end

local function side_insets(sprite, digit)
    if sprite.ink == nil then
        return 0, 0
    end
    local ink = digit and sprite.ink[digit + 1] or math.max(table.unpack(sprite.ink))
    local left = math.floor((sprite.cell_w - ink) / 2)
    return left, sprite.cell_w - ink - left
end

function Builder:number_box(spec, box)
    local sprite = self:sprite(spec.sprite)
    local shown = spec.known ~= nil and digits_of(spec.known) or nil
    local digits = spec.digit or #shown
    local lead = side_insets(sprite, shown and tonumber(shown:sub(1, 1)))
    local _, trail = side_insets(sprite, shown and tonumber(shown:sub(-1)))
    local gaps = (spec.space or 0) * (digits - 1)
    local ink = sprite.cell_w * digits - lead - trail
    local scale = (spec.size or sprite.size) / sprite.size
    if spec.fit and ink * scale + gaps > spec.fit then
        scale = (spec.fit - gaps) / ink
    end
    local cell_w = math.floor(sprite.cell_w * scale)
    scale = cell_w / sprite.cell_w
    local span = ink * scale + gaps
    local align = spec.align or "left"
    local x = box.x
    if box.w and align == "center" then
        x = box.x + (box.w - span) / 2
    elseif box.w and align == "right" then
        x = box.x + box.w - span
    end
    local rise = (sprite.cell_h - sprite.pad * 2) * scale
    local top = box.top or box.baseline - rise
    return { x = x - lead * scale, top = top - sprite.pad * scale, w = cell_w, h = sprite.cell_h * scale, digits = digits, span = span, rise = rise }
end

function Builder:number(spec, box, opts)
    local layout = self:number_box(spec, box)
    self:place(self:define_number(spec, layout.digits), layout, filtered(opts))
    return layout
end

function Builder:object(list, def, box, opts)
    def.id = def.id or self:next_id(list)
    table.insert(self.skin[list], def)
    if box ~= nil then
        self:place(def.id, box, opts)
    end
    return def.id
end

function Builder:single(key, def, box, opts)
    def.id = def.id or key
    self.skin[key] = def
    self:place(def.id, box, opts)
    return def.id
end

function Builder:finish()
    for _, list in ipairs(OBJECT_LISTS) do
        if list ~= "destination" and #self.skin[list] == 0 then
            self.skin[list] = nil
        end
    end
    return self.skin
end

return ui
