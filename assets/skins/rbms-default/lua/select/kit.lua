local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")

local kit = {}

kit.ARRIVAL = { fade = 150, slide = 150, distance = 16 }

function kit.arriving(opts)
    return motion.fade_in(0, kit.ARRIVAL.fade, ui.merge(opts, { timer = props.timer.SONGBAR_CHANGE }))
end

function kit.sliding(opts)
    local timed = ui.merge(opts, { timer = props.timer.SONGBAR_CHANGE })
    return motion.slide_in(0, kit.ARRIVAL.slide, -kit.ARRIVAL.distance, 0, timed)
end

function kit.button(s, spec, box, opts)
    local base = ui.merge(opts, { color = spec.color or theme.color.panel, def = { id = spec.id, act = spec.act } })
    s:image("block", box, base)
    local lit = ui.merge(opts, { color = spec.lit or theme.color.raised, hover = { x = 0, top = 0, w = box.w, h = box.h } })
    s:image("block", box, lit)
end

function kit.meter(s, rate)
    return ui.merge(s:sprite_source("meter"), { angle = props.graph_angle.RIGHT, type = rate })
end

function kit.slider(s, rate, angle, range)
    return ui.merge(s:sprite_source("block"), { angle = angle, range = range, type = rate, changeable = true })
end

return kit
