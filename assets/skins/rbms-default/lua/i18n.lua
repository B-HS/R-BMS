local props = require("lua.props")
local main_state = require("main_state")

local AREA_MODULE_PREFIX = "lua.i18n."
local FALLBACK_LANGUAGE = "en"
local PLACEHOLDER = "{(%d+)}"
local FIRST_PLACEHOLDER = 0
local JOINERS = { en = " ", ko = " ", ja = "" }

local i18n = {}

i18n.AREAS = { "common", "select_bar", "select_info", "select_other", "play", "result", "decide" }
i18n.LANGUAGES = { "en", "ko", "ja" }

function i18n.merge(areas)
    local entries = {}
    for _, area in ipairs(areas) do
        for key, entry in pairs(area) do
            if entries[key] ~= nil then
                error("i18n key defined in two areas: " .. tostring(key), 2)
            end
            entries[key] = entry
        end
    end
    return entries
end

function i18n.language()
    local option = main_state.option
    if type(option) ~= "function" then
        return FALLBACK_LANGUAGE
    end
    for index, code in ipairs(i18n.LANGUAGES) do
        if option(props.extension.LANGUAGE_FIRST + index - 1) then
            return code
        end
    end
    return FALLBACK_LANGUAGE
end

function i18n.fill(template, ...)
    local values = { ... }
    return (template:gsub(PLACEHOLDER, function(place)
        local value = values[tonumber(place) - FIRST_PLACEHOLDER + 1]
        return value == nil and "" or tostring(value)
    end))
end

function i18n.join(parts)
    return table.concat(parts, JOINERS[i18n.language()] or JOINERS[FALLBACK_LANGUAGE])
end

local function text_of(entry)
    return entry[i18n.language()] or entry[FALLBACK_LANGUAGE]
end

function i18n.translator(areas)
    local entries = i18n.merge(areas)
    local function entry_of(key)
        local entry = entries[key]
        if entry == nil then
            error("unknown i18n key: " .. tostring(key), 3)
        end
        return entry
    end
    local function text(key)
        local entry = entry_of(key)
        return function()
            return text_of(entry)
        end
    end
    local function template(key)
        local entry = entry_of(key)
        return function(...)
            return i18n.fill(text_of(entry), ...)
        end
    end
    return text, template
end

local areas = {}
for _, name in ipairs(i18n.AREAS) do
    table.insert(areas, require(AREA_MODULE_PREFIX .. name))
end

i18n.t, i18n.template = i18n.translator(areas)

return i18n
