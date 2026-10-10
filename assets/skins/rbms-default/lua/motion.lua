local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")

local motion = {}

local TRANSPARENT = 0

function motion.fade_in(start, duration, opts)
    local length = duration or theme.time.fade
    return ui.merge(opts, { keys = { { time = start, a = TRANSPARENT }, { time = start + length } } })
end

function motion.fade_out(start, duration, opts)
    local length = duration or theme.time.fade
    local keys = { { time = start }, { time = start + length, a = TRANSPARENT } }
    if start > 0 then
        table.insert(keys, 1, { time = 0 })
    end
    return ui.merge(opts, { keys = keys, loop = props.loop.ONCE })
end

function motion.slide_in(start, duration, dx, dtop, opts)
    local length = duration or theme.time.slide
    return ui.merge(opts, {
        keys = { { time = start, dx = dx or 0, dtop = dtop or 0, a = TRANSPARENT }, { time = start + length } },
        acc = props.acc.DECELERATE,
    })
end

function motion.grow_right(start, duration, opts)
    return ui.merge(opts, { keys = { { time = start, w = 0 }, { time = start + duration } }, acc = props.acc.DECELERATE })
end

function motion.grow_down(start, duration, opts)
    return ui.merge(opts, { keys = { { time = start, h = 0 }, { time = start + duration } }, acc = props.acc.DECELERATE })
end

function motion.grow_up(start, duration, height, opts)
    return ui.merge(opts, { keys = { { time = start, h = 0, dtop = height }, { time = start + duration } }, acc = props.acc.DECELERATE })
end

function motion.progress_right(duration, opts)
    return ui.merge(opts, { keys = { { time = 0, w = 0 }, { time = duration } } })
end

function motion.pulse(period, low, opts)
    return ui.merge(opts, { keys = { { time = 0 }, { time = period / 2, a = low }, { time = period } }, loop = 0 })
end

function motion.staggered(index, start)
    return (start or 0) + index * theme.time.stagger
end

return motion
