frame_runs = (frame_runs or 0) + 1

local frame = {}

function frame.add(skin)
	table.insert(skin.destination, { id = "chara", dst = { { x = 200, y = 0, w = 32, h = 32 } } })
end

return frame
