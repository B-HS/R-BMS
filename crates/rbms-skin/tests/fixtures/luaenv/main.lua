main_loads = (main_loads or 0) + 1

local header = {
    type = 6,
    name = "luaenv fixture",
    w = 1280,
    h = 720,
    property = {
        { name = "Backdrop", item = { { name = "Plain", op = 900 }, { name = "Striped", op = 901 } } },
    },
    filepath = {
        { name = "Frame", path = "data/*.txt" },
    },
    offset = {
        { name = "Title", id = 40, x = true, y = true },
    },
}

local function main()
    local skin = {}
    for key, value in pairs(header) do
        skin[key] = value
    end
    skin.selected = skin_config.option["Backdrop"]
    skin.enabled = skin_config.enabled_options
    skin.frame = skin_config.file_path["Frame"]
    skin.title_x = skin_config.offset["Title"].x
    skin.title_fields = 0
    for _ in pairs(skin_config.offset["Title"]) do
        skin.title_fields = skin.title_fields + 1
    end
    skin.resolved = skin_config.get_path("data/notes.txt")
    skin.part = dofile(skin.resolved_part or "parts/part.lua").value
    skin.loads = main_loads
    return skin
end

return { header = header, main = main }
