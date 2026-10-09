local status, part = pcall(function()
    return dofile("parts/fails.lua").load()
end)
return { status = status, part = part }
