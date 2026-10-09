local files = {}

function files.touch(path)
    local handle = io.open(path, "w")
    handle:close()
end

function files.exists(path)
    local handle = io.open(path, "r")
    if handle == nil then
        return false
    end
    handle:close()
    return true
end

function files.count(path, start)
    local handle = io.open(path)
    local count = start
    for _ in handle:lines() do
        count = count + 1
    end
    handle:close()
    return count
end

function files.overwrite(path, text)
    local handle = io.open(path, "w")
    handle:write(text)
    handle:close()
end

function files.append(path, text)
    io.open(path, "a"):write(text):close()
end

function files.lines(path)
    local found = {}
    local handle = io.open(path, "r")
    for line in handle:lines() do
        found[#found + 1] = line
    end
    handle:close()
    return found
end

function files.reset(path)
    local handle = io.open(path, "w")
    handle:write()
    handle:close()
end

function files.rotate(folder, candidates)
    local all = folder .. "/pathList.txt"
    local used = folder .. "/excludeList.txt"
    if not files.exists(all) then
        files.touch(all)
    end
    if not files.exists(used) then
        files.touch(used)
    end
    if files.count(all, 0) ~= #candidates then
        files.overwrite(all, table.concat(candidates, "\n") .. "\n")
        files.reset(used)
    end
    if files.count(used, 0) >= files.count(all, 0) then
        files.reset(used)
    end
    local spent = {}
    for _, name in ipairs(files.lines(used)) do
        spent[name] = true
    end
    for _, name in ipairs(files.lines(all)) do
        if not spent[name] then
            files.append(used, name .. "\n")
            return name
        end
    end
end

return files
