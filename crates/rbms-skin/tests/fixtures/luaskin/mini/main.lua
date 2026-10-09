module_runs = (module_runs or 0) + 1

local main_state = require("main_state")
local frame = require("parts.frame")

local header = {
	type = 0,
	name = "Mini",
	author = "rbms",
	w = 1920,
	h = 1080,
	loadend = 3500,
	playstart = 1000,
	close = 3000,
	fadeout = 500,
	scene = 3600000,
	judgetimer = 2,
	finishmargin = 300,
	category = {
		{ name = "Look", item = { 1, 2, 3 } },
	},
	property = {
		{ name = "Panel", category = 1, def = "Off", item = { { name = "On", op = 900 }, { name = "Off", op = 901 } } },
		{ name = "Lucky", category = 1, item = { { name = "A", op = 910 }, { name = "B", op = 911 }, { name = "C", op = 912 } } },
	},
	filepath = {
		{ name = "Background", category = 2, path = "bg/*.png", def = "Night" },
		{ name = "Chara", category = 2, path = "chara/*|1P|", def = "bob" },
	},
	offset = {
		{ name = "Shift", category = 3, id = 40, x = 0, y = 0 },
	},
}

local function main()
	main_runs = (main_runs or 0) + 1
	seen_config = skin_config

	local skin = {}
	for key, value in pairs(header) do
		skin[key] = value
	end

	skin.source = {
		{ id = 0, path = "bg/*.png" },
		{ id = 1, path = "parts/sheet.png" },
		{ id = 2, path = "chara/*|1P|/body.png" },
	}
	skin.image = {
		{ id = "bg", src = 0, x = 0, y = 0, w = 1920, h = 1080 },
		{ id = "panel", src = 1, x = 0, y = 0, w = 64, h = 64 },
		{ id = "chara", src = 2, x = 0, y = 0, w = 32, h = 32 },
	}
	skin.value = {
		{
			id = "combo",
			src = 1,
			x = 0,
			y = 64,
			w = 100,
			h = 10,
			divx = 10,
			digit = 4,
			value = function()
				return main_state.number(71) * 2
			end,
		},
	}
	skin.note = { id = "notes" }
	skin.destination = {
		{ id = "bg", dst = { { x = 0, y = 0, w = 1920, h = 1080 } } },
		{ id = "panel", op = { 900 }, dst = { { x = 0, y = 0, w = 64, h = 64 } } },
		{ id = "panel", op = { 901 }, dst = { { x = 100, y = 0, w = 64, h = 64 } } },
		{
			id = "combo",
			draw = function()
				return main_state.option(33)
			end,
			timer = function()
				return main_state.timer(41)
			end,
			dst = { { time = 0, x = 10, y = 20, w = 30 / 2, h = 40 }, { time = 1000, x = 110 } },
		},
		{ id = "notes", dst = { { x = 0, y = 0, w = 1920, h = 1080 } } },
	}

	frame.add(skin)

	building = skin
	extra_ran = pcall(dofile, skin_config.get_path("parts/extra.lua"))
	broken_ran = pcall(dofile, skin_config.get_path("parts/broken.lua"))
	building = nil

	local log = io.open(skin_config.get_path("log/loaded.txt"), "w")
	wrote_log = log ~= nil
	if log then
		log:write("loaded")
		log:close()
	end

	return skin
end

return { header = header, main = main }
