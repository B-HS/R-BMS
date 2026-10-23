local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local parts = require("lua.parts")
local list = require("lua.select.list")
local controls = require("lua.select.controls")
local info = require("lua.select.info")
local side = require("lua.select.side")
local sheet = require("lua.select.sheet")

local screen = {}

local TIME = { input = 300 }

function screen.header()
    return {
        type = props.skin_type.MUSIC_SELECT,
        name = theme.name .. " (Select)",
        author = theme.author,
        rbms_extensions = props.rbms_extensions,
        w = theme.width,
        h = theme.height,
        input = TIME.input,
        scene = 0,
        fadeout = 0,
        property = theme.properties({ parts.stage_image_property() }),
    }
end

local function bands(s)
    local grid = theme.grid
    s:rect({ x = 0, top = 0, w = theme.width, h = grid.header_rule }, theme.color.bg)
    s:rect({ x = 0, top = grid.footer_rule, w = theme.width, h = theme.height - grid.footer_rule }, theme.color.bg)
    parts.frame(s, "MUSIC SELECT")
end

function screen.build()
    local header = screen.header()
    if skin_config == nil then
        return header
    end
    local main_state = require("main_state")
    theme.apply(skin_config.option)
    local s = ui.new(header)
    s:atlas("common")
    s:atlas("select")
    parts.backdrop(s)
    list.build(s, main_state)
    bands(s)
    controls.header(s, main_state)
    info.build(s, main_state)
    controls.footer(s, main_state)
    side.build(s, main_state)
    sheet.build(s, main_state)
    parts.fade_from_black(s, theme.time.fade)
    return s:finish()
end

return screen
