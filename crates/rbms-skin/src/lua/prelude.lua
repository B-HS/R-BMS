local main_state = ...

local error, tonumber, type = error, tonumber, type
local floor, ceil = math.floor, math.ceil
local time, timer = main_state.time, main_state.timer
local OFF = main_state.timer_off_value
local MICROS_PER_MILLI = 1000

local function truncated(number)
    if number < 0 then
        return ceil(number)
    end
    return floor(number)
end

local function to_long(value)
    local number = tonumber(value)
    if number == nil or number ~= number then
        return 0
    end
    if number <= OFF then
        return OFF
    end
    if number >= -OFF then
        return -OFF
    end
    return truncated(number)
end

local function expect_function(value)
    if type(value) ~= "function" then
        error("bad argument: function expected, got " .. type(value), 3)
    end
    return value
end

local timer_util = {}

function timer_util.now_timer(value)
    local since = to_long(value)
    if since == OFF then
        return 0
    end
    return time() - since
end

function timer_util.is_timer_on(value)
    return to_long(value) ~= OFF
end

function timer_util.is_timer_off(value)
    return to_long(value) == OFF
end

function timer_util.timer_function(id)
    return function()
        return timer(id)
    end
end

function timer_util.timer_observe_boolean(observed)
    expect_function(observed)
    local value = OFF
    return function()
        local on = observed()
        if on and value == OFF then
            value = time()
        elseif not on and value ~= OFF then
            value = OFF
        end
        return value
    end
end

function timer_util.new_passive_timer()
    local value = OFF
    return {
        timer = function()
            return value
        end,
        turn_on = function()
            if value == OFF then
                value = time()
            end
            return true
        end,
        turn_on_reset = function()
            value = time()
            return true
        end,
        turn_off = function()
            value = OFF
            return true
        end,
    }
end

local event_util = {}

function event_util.event_observe_turn_true(observed, action)
    expect_function(observed)
    expect_function(action)
    local was_on = false
    return function()
        local on = not not observed()
        if was_on ~= on then
            was_on = on
            if on then
                action()
            end
        end
        return true
    end
end

function event_util.event_observe_timer(observed, action)
    expect_function(observed)
    expect_function(action)
    local value = OFF
    return function()
        local current = to_long(observed())
        if current ~= value and current ~= OFF then
            value = current
            action()
        end
        return true
    end
end

function event_util.event_observe_timer_on(observed, action)
    expect_function(observed)
    expect_function(action)
    local was_on = false
    return function()
        local on = to_long(observed()) ~= OFF
        if was_on ~= on then
            was_on = on
            if on then
                action()
            end
        end
        return true
    end
end

function event_util.event_observe_timer_off(observed, action)
    expect_function(observed)
    expect_function(action)
    local was_off = false
    return function()
        local off = to_long(observed()) == OFF
        if was_off ~= off then
            was_off = off
            if off then
                action()
            end
        end
        return true
    end
end

function event_util.event_min_interval(interval, action)
    local minimum = to_long(interval)
    expect_function(action)
    local last = OFF
    return function()
        if last == OFF or truncated((time() - last) / MICROS_PER_MILLI) >= minimum then
            last = time()
            action()
        end
        return true
    end
end

return timer_util, event_util
