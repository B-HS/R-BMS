local props = require("lua.props")
local main_state = require("main_state")

local AREA_MODULE_PREFIX = "lua.i18n."
local FALLBACK_LANGUAGE = "en"

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

function i18n.translator(areas)
    local entries = i18n.merge(areas)
    return function(key)
        local entry = entries[key]
        if entry == nil then
            error("unknown i18n key: " .. tostring(key), 2)
        end
        return function()
            return entry[i18n.language()] or entry[FALLBACK_LANGUAGE]
        end
    end
end

local areas = {}
for _, name in ipairs(i18n.AREAS) do
    table.insert(areas, require(AREA_MODULE_PREFIX .. name))
end

i18n.t = i18n.translator(areas)

return i18n
