part_runs = (part_runs or 0) + 1
return { value = part_runs, load = function() return part_runs end }
