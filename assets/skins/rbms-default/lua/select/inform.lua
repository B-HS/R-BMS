local inform = {}

local function settle(seen, page)
    seen.title = page.guide.title
    seen.value = ""
    seen.line = page.guide.line
    for index, watch in ipairs(page.watch) do
        seen.last[index] = watch.read()
    end
end

local function spoken(text)
    if type(text) == "function" then
        return text()
    end
    return text
end

function inform.observer(main_state, page)
    local seen = { opened = nil, last = {}, title = page.guide.title, value = "", line = page.guide.line }
    return function()
        if not main_state.option(page.option) then
            return seen
        end
        local opened = main_state.timer(page.on)
        if opened ~= seen.opened then
            seen.opened = opened
            settle(seen, page)
            return seen
        end
        for index, watch in ipairs(page.watch) do
            local now = watch.read()
            if now ~= seen.last[index] then
                seen.last[index] = now
                seen.title, seen.value, seen.line = watch.tell(now)
            end
        end
        return seen
    end
end

function inform.title(observe)
    return function()
        return spoken(observe().title)
    end
end

function inform.value(observe)
    return function()
        return observe().value
    end
end

function inform.line(observe)
    return function()
        return spoken(observe().line)
    end
end

return inform
