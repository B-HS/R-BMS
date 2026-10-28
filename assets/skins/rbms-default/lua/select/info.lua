local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")
local i18n = require("lua.i18n")
local kit = require("lua.select.kit")
local radar = require("lua.select.radar")
local records = require("lua.select.records")

local info = {}

local OPTION = { STAGE_SHOW = parts.stage_image.show }
local ON_CHART = props.option.SONGBAR
local ON_FOLDER = props.option.FOLDERBAR
local ON_COURSE = props.option.GRADEBAR
local UNPLAYED = props.option.BAR_NOT_PLAYED
local NOTHING = "-"
local FIT = props.text_overflow.SHRINK_CUT

local REGION = { x = 64, width = 1032 }

local LAYOUT = theme.select_layout
local SONG = LAYOUT.song
local FACTS = LAYOUT.facts
local VIEW = LAYOUT.chart_view
local ROWS = LAYOUT.score_rows
local CELLS = LAYOUT.target_cells

local HEAD_LINES = {
    { ref = props.text.GENRE, top = SONG.lines.genre, size = theme.type.label, color = theme.color.sub },
    { ref = props.text.TITLE, top = SONG.lines.title, size = theme.type.title, color = theme.color.text },
    { ref = props.text.SUBTITLE, top = SONG.lines.subtitle, size = theme.type.label, color = theme.color.sub },
    { ref = props.text.FULLARTIST, top = SONG.lines.artist, size = theme.type.body, color = theme.color.text },
}
local HEAD_COURSE = { top = SONG.lines.title, size = theme.type.title, label = i18n.t("common_course") }
local HEAD_META = { label_size = theme.type.caption, bpm = "BPM" }

local SECTION = { title_drop = 12 }
local EX_PER_NOTE = 2
local RANK_STEPS = 9
local PERCENT = 100

local STAGE = { fade = 200, missing_at = 300 }

local CLEAR_OPTIONS = {
    [0] = props.option.BAR_NOT_PLAYED,
    props.option.BAR_FAILED,
    props.option.BAR_ASSIST_EASY,
    props.option.BAR_LIGHT_ASSIST_EASY,
    props.option.BAR_EASY,
    props.option.BAR_NORMAL,
    props.option.BAR_HARD,
    props.option.BAR_EXHARD,
    props.option.BAR_FULL_COMBO,
    props.option.BAR_PERFECT,
    props.option.BAR_MAX,
}

local CHART = {
    tabs = { width = 64, gap = 4, top_drop = 8, height = 32, band = 3, sprite = "num24", digits = 3, fill_share = 0.35 },
    lanes = { slots = 8, bar_width = 23, sprite = "num18", digits = 4, number_gap = 6, inset = 6 },
}

local SCORE = {
    big_sprite = "num44",
    small_sprite = "num32",
    digits = 5,
    progress = { height = 8, label_drop = 12, tick = 2, first_tick = 2 },
    pairs = { sprite = "num24", digits = 5 },
    judges = { sprite = "num18", digits = 5 },
}

local TARGETS = {
    steps = { 8, 7, 6 },
    gutter = 32,
    sprite = "num24",
    digits = 5,
    title_drop = 8,
    content_drop = 28,
}

local FOLDER = {
    top = 368,
    count_baseline = 448,
    count_sprite = "num64",
    digits = 5,
    lamps_top = 480,
    first_row = 512,
    pitch = 72,
    row_height = 40,
    per_column = 6,
    columns = { 64, 580 },
    width = 484,
    chip_width = 8,
    name_x = 24,
    size = theme.type.body,
    sprite = "num32",
}

local COURSE = { top = 368, first_row = 400, pitch = 56, stages = 5, constraints_top = 898, constraints_value_top = 930 }
local CONSTRAINTS = {
    { op = 1002, label = "CLASS" },
    { op = 1003, label = "MIRROR" },
    { op = 1004, label = "RANDOM" },
    { op = 1005, label = "NO SPEED" },
    { op = 1006, label = "NO GOOD" },
    { op = 1007, label = "NO GREAT" },
    { op = 1010, label = "LR2 GAUGE" },
    { op = 1011, label = "5 KEYS GAUGE" },
    { op = 1012, label = "7 KEYS GAUGE" },
    { op = 1013, label = "9 KEYS GAUGE" },
    { op = 1014, label = "24 KEYS GAUGE" },
    { op = 1015, label = "LN" },
    { op = 1016, label = "CN" },
    { op = 1017, label = "HCN" },
}
local CONSTRAINT_JOINT = "   /   "
local NO_CONSTRAINT = i18n.t("common_none")

local function caption(s, label, box, opts)
    local spec = { text = label, size = theme.type.caption, overflow = FIT, align = box.align }
    s:text(spec, box, ui.merge({ color = theme.color.muted, op = { ON_CHART } }, opts))
end

local function bpm_range(main_state)
    return function()
        local low = main_state.number(props.number.MINBPM)
        local high = main_state.number(props.number.MAXBPM)
        if not ui.has_value(high) then
            return ""
        end
        if not ui.has_value(low) or low == high then
            return string.format("%d", high)
        end
        return string.format("%d - %d", low, high)
    end
end

local function head_meta(s, main_state)
    local box = { x = REGION.x + REGION.width - SONG.meta_width, w = SONG.meta_width, align = "right" }
    local genre, title, subtitle, artist = table.unpack(HEAD_LINES)
    local function label_top(line)
        return line.top + theme.cap_height(line.size) - theme.cap_height(HEAD_META.label_size)
    end

    caption(s, i18n.t("common_level"), ui.merge(box, { top = label_top(genre) }))
    local level = { sprite = SONG.level_sprite, ref = props.number.PLAYLEVEL, digit = SONG.level_digits, align = "right", saturate = true }
    local baseline = title.top + theme.cap_height(title.size)
    parts.by_difficulty(function(_, op, color)
        s:number(level, { x = box.x, baseline = baseline, w = box.w }, kit.arriving({ color = color, op = { ON_CHART, op } }))
    end)

    caption(s, HEAD_META.bpm, ui.merge(box, { top = label_top(subtitle) }))
    local bpm = { value = bpm_range(main_state), size = artist.size, align = "right", overflow = FIT }
    s:text(bpm, { x = box.x, top = artist.top, w = box.w }, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
end

local function head(s, main_state)
    local bar = { x = REGION.x, top = SONG.top, w = SONG.bar_width, h = SONG.height }
    s:rect(bar, theme.color.line)
    parts.difficulty_rect(s, bar, { op = { ON_CHART } })
    s:rect(bar, theme.color.accent, { op = { ON_FOLDER } })
    s:rect(bar, theme.color.accent, { op = { ON_COURSE } })

    local x = REGION.x + SONG.bar_width + SONG.text_gap
    local right = REGION.x + REGION.width
    local width = right - SONG.meta_width - SONG.meta_gap - x
    for _, line in ipairs(HEAD_LINES) do
        local spec = { ref = line.ref, size = line.size, overflow = FIT }
        s:text(spec, { x = x, top = line.top, w = width }, kit.sliding({ color = line.color }))
    end
    local course_title = { x = x, top = HEAD_COURSE.top, w = right - x }
    s:text({ text = HEAD_COURSE.label, size = HEAD_COURSE.size, overflow = FIT }, course_title, kit.sliding({ color = theme.color.text, op = { ON_COURSE } }))
    head_meta(s, main_state)
end

local function stage(s)
    local cover = LAYOUT.cover
    local box = { x = cover.x, top = cover.top, w = cover.side, h = cover.side }
    local shown = { OPTION.STAGE_SHOW, ON_CHART }
    parts.panel(s, box, { op = shown })
    local arriving = motion.fade_in(0, STAGE.fade, { op = shown, timer = props.timer.SONGBAR_CHANGE, stretch = props.stretch.FIT_INNER })
    s:reference(props.image.STAGEFILE, box, arriving)
    local size = theme.type.caption
    local unpictured = { op = { OPTION.STAGE_SHOW, ON_CHART, props.option.NO_STAGEFILE }, timer = props.timer.SONGBAR_CHANGE, color = theme.color.muted }
    local label = { x = box.x, top = theme.centered_top(box, size), w = box.w }
    s:text({ text = parts.stage_image.missing, size = size, align = "center", overflow = FIT }, label, motion.fade_in(STAGE.missing_at, STAGE.fade, unpictured))
end

local function section(s, area, key, title_drop)
    local on = { op = { ON_CHART } }
    s:rect({ x = area.x, top = area.top, w = area.width, h = theme.grid.rule }, theme.color.line, on)
    local title = { value = i18n.t(key), size = theme.type.caption, overflow = FIT }
    s:text(title, { x = area.x, top = area.top + (title_drop or SECTION.title_drop), w = area.width }, { color = theme.color.sub, op = { ON_CHART } })
end

local function chart_length(main_state)
    return function()
        local minutes = main_state.number(props.number.SONGLENGTH_MINUTE)
        local seconds = main_state.number(props.number.SONGLENGTH_SECOND)
        if not (ui.has_value(minutes) and ui.has_value(seconds)) then
            return ""
        end
        return string.format("%d:%02d", minutes, seconds)
    end
end

local function tabs(s)
    local area = LAYOUT.chart_data
    local tab = CHART.tabs
    local slots = props.option.DIFFICULTY_LAST - props.option.DIFFICULTY_FIRST
    local left = area.x + area.width - slots * tab.width - (slots - 1) * tab.gap
    for slot = 0, slots - 1 do
        local difficulty = slot + 1
        local color = theme.difficulty[difficulty]
        local box = { x = left + slot * (tab.width + tab.gap), top = area.top + tab.top_drop, w = tab.width, h = tab.height }
        kit.button(s, { id = "btn.sibling." .. difficulty, act = props.extension_event.SIBLING_CHART_FIRST + slot }, box, { op = { ON_CHART } })
        local current = { op = { ON_CHART, props.option.DIFFICULTY_FIRST + difficulty } }
        s:rect(box, theme.mix(color, theme.color.panel, tab.fill_share), current)
        local band = { x = box.x, top = box.top + box.h - tab.band, w = box.w, h = tab.band }
        s:rect(band, color, { op = { ON_CHART, props.extension.SIBLING_CHART_FIRST + slot } })
        local level = { sprite = tab.sprite, ref = props.extension_number.SIBLING_CHART_LEVEL_FIRST + slot, digit = tab.digits, align = "center" }
        local sprite = s:sprite(tab.sprite)
        local baseline = box.top + (box.h - tab.band + sprite.cell_h - sprite.pad * 2) / 2
        s:number(level, { x = box.x, baseline = baseline, w = box.w }, kit.arriving({ color = color, op = { ON_CHART } }))
    end
end

local function density(main_state, whole_ref, rest_ref)
    return function()
        local whole = main_state.number(whole_ref)
        local rest = main_state.number(rest_ref)
        if not (ui.has_value(whole) and ui.has_value(rest)) then
            return ""
        end
        if whole == 0 and rest == 0 then
            return NOTHING
        end
        return string.format("%d.%02d", whole, rest)
    end
end

local function facts(s, main_state)
    local area = LAYOUT.chart_data
    local size = theme.type.label
    local cells = {}
    local function place(id, label)
        local cell = FACTS.cells[id]
        local top = area.top + FACTS.label_drop + cell.row * FACTS.row_pitch
        local baseline = top + FACTS.value_drop
        caption(s, label, { x = area.x + cell.x, top = top, w = cell.width })
        cells[id] = { x = area.x + cell.x, top = theme.text_top(baseline, size), w = cell.width, baseline = baseline }
        return cells[id]
    end
    local word = { size = size, overflow = FIT }
    local shown = kit.arriving({ color = theme.color.text, op = { ON_CHART } })

    local notes = { sprite = FACTS.sprite, ref = props.number.TOTALNOTES, digit = FACTS.notes_digits, saturate = true }
    s:number(notes, place("notes", i18n.t("common_notes")), shown)
    s:text(ui.merge(word, { value = density(main_state, props.number.DENSITY_AVERAGE, props.number.DENSITY_AVERAGE_AFTERDOT) }), place("average", i18n.t("info_nps_average")), shown)
    s:text(ui.merge(word, { value = density(main_state, props.number.DENSITY_PEAK, props.number.DENSITY_PEAK_AFTERDOT) }), place("peak", i18n.t("info_nps_peak")), shown)

    s:text(ui.merge(word, { value = chart_length(main_state) }), place("length", i18n.t("common_length")), shown)
    local total = { sprite = FACTS.sprite, ref = props.number.SONGGAUGE_TOTAL, digit = FACTS.total_digits, saturate = true }
    s:number(total, place("total", "TOTAL"), shown)
    parts.key_mode_text(s, word, place("keys", i18n.t("common_keys")), shown)
    parts.judge_rank_text(s, word, place("judge", i18n.t("common_judge")), shown)
end

local function graph(s)
    local area = LAYOUT.chart_data
    local spec = VIEW.graph
    local box = { x = area.x + spec.x, top = area.top + spec.top_drop, w = spec.width, h = spec.height }
    local on = { op = { ON_CHART } }
    parts.panel(s, box, on)
    s:object("judgegraph", { type = props.judge_graph.NOTES, backTexOff = props.back_texture.OFF }, box, kit.arriving(on))
    s:object("bpmgraph", parts.bpm_graph(), box, kit.arriving(on))
end

local function hand(first, keys, scratch, scratch_left)
    local lanes = {}
    for key = 0, keys - 1 do
        table.insert(lanes, { lane = first + key, kind = key % 2 == 1 and "black" or "white" })
    end
    local turntable = { lane = scratch, kind = "scratch" }
    if scratch_left then
        table.insert(lanes, 1, turntable)
    else
        table.insert(lanes, turntable)
    end
    return lanes
end

local LANE_MODES = {
    { op = props.option.KEYS_5, rows = { hand(0, 5, 5, true) } },
    { op = props.option.KEYS_7, rows = { hand(0, 7, 7, true) } },
    { op = props.option.KEYS_10, rows = { hand(0, 5, 5, true), hand(6, 5, 11, false) } },
    { op = props.option.KEYS_14, rows = { hand(0, 7, 7, true), hand(8, 7, 15, false) } },
}

local function lane_colors()
    return { white = theme.color.text, black = theme.color.sub, scratch = theme.color.accent }
end

local function lanes(s)
    local area = LAYOUT.chart_data
    local spec = ui.merge(CHART.lanes, VIEW.lanes)
    local panel = { x = area.x + spec.x, top = area.top + spec.top_drop, w = spec.width, h = spec.height }
    parts.panel(s, panel, { op = { ON_CHART } })
    local pitch = spec.width / spec.slots
    local sprite = s:sprite(spec.sprite)
    local rise = sprite.cell_h - sprite.pad * 2
    local colors = lane_colors()
    for _, mode in ipairs(LANE_MODES) do
        local on = { op = { ON_CHART, mode.op } }
        local row_height = (spec.height - spec.inset) / #mode.rows
        for row_index, row in ipairs(mode.rows) do
            local row_top = panel.top + spec.inset + (row_index - 1) * row_height
            local left = panel.x + (spec.width - #row * pitch) / 2
            local baseline = row_top + rise
            local bar_top = baseline + spec.number_gap
            local bar_height = row_top + row_height - spec.inset - bar_top
            for index, entry in ipairs(row) do
                local x = left + (index - 1) * pitch
                local count = { sprite = spec.sprite, ref = props.extension_number.LANE_NOTES_FIRST + entry.lane, digit = spec.digits, align = "center", fit = pitch }
                s:number(count, { x = x, baseline = baseline, w = pitch }, kit.arriving(ui.merge(on, { color = theme.color.sub })))
                local bar = { x = x + (pitch - spec.bar_width) / 2, top = bar_top, w = spec.bar_width, h = bar_height }
                s:rect(bar, theme.color.raised, on)
                local meter = ui.merge(s:sprite_source("block"), { angle = props.graph_angle.UP, type = props.extension_rate.LANE_NOTES_FIRST + entry.lane })
                s:object("graph", meter, bar, kit.arriving(ui.merge(on, { color = colors[entry.kind] })))
            end
        end
    end
end

local function playback_rate(s, main_state)
    local area = LAYOUT.chart_data
    local tab = CHART.tabs
    local slots = props.option.DIFFICULTY_LAST - props.option.DIFFICULTY_FIRST
    local tabs_left = area.x + area.width - slots * tab.width - (slots - 1) * tab.gap
    local label = i18n.template("play_playback_rate")
    local rate = {
        value = function()
            return label(main_state.number(props.extension_number.PLAYBACK_PERCENT))
        end,
        size = theme.type.caption,
        align = "right",
        overflow = FIT,
    }
    local box = { x = area.x, top = area.top + SECTION.title_drop, w = tabs_left - tab.gap - area.x }
    s:text(rate, box, { color = theme.tone.yellow, op = { ON_CHART, props.extension.PLAYBACK_RATE_CHANGED } })
end

local function chart_data(s, main_state)
    section(s, LAYOUT.chart_data, "info_chart_data")
    playback_rate(s, main_state)
    tabs(s)
    facts(s, main_state)
    graph(s)
    lanes(s)
end

local function score_rate(main_state)
    return function()
        local whole = main_state.number(props.number.SCORE_RATE)
        local rest = main_state.number(props.number.SCORE_RATE_AFTERDOT)
        if not (ui.has_value(whole) and ui.has_value(rest)) then
            return ""
        end
        return string.format("%d.%02d%%", whole, rest)
    end
end

local function holds_score(main_state)
    return function()
        return ui.has_value(main_state.number(props.number.PERFECT_RATE))
    end
end

local function holds_no_score(main_state)
    return function()
        return not ui.has_value(main_state.number(props.number.PERFECT_RATE))
    end
end

local function score_shown(main_state)
    return { shown = { op = { ON_CHART }, draw = holds_score(main_state) }, blank = { op = { ON_CHART }, draw = holds_no_score(main_state) } }
end

local COUNT_SHOWN = { shown = { op = { ON_CHART } }, blank = { op = { ON_CHART, UNPLAYED } } }

local function dash(s, size, box, when, align)
    s:text({ text = NOTHING, size = size, align = align }, box, ui.merge({ color = theme.color.muted }, when.blank))
end

local function rank_label(level)
    return string.upper(theme.rank_key[level])
end

local function next_rank(main_state)
    return function()
        local ex = main_state.number(props.number.SCORE)
        local gap = main_state.number(props.number.DIFF_NEXTRANK)
        local notes = main_state.number(props.number.TOTALNOTES)
        if not (ui.has_value(ex) and ui.has_value(gap) and ui.has_value(notes)) or notes <= 0 then
            return ""
        end
        local most = notes * EX_PER_NOTE
        local target = ex + gap
        if target >= most then
            return string.format("MAX -%d", gap)
        end
        local step = math.floor(target * RANK_STEPS / most)
        local level = math.max(0, math.min(theme.rank_last, step - (RANK_STEPS - 1 - theme.rank_last)))
        return string.format("%s -%d", rank_label(level), gap)
    end
end

local function score_numbers(s, when)
    local area = LAYOUT.score
    local top = area.top + ROWS.label_drop
    local baseline = area.top + ROWS.baseline_drop
    local columns = {
        { label = "EX SCORE", ref = props.number.SCORE, sprite = SCORE.big_sprite, width = ROWS.ex_width, size = theme.type.heading },
        { label = "MISS COUNT", ref = props.number.MISSCOUNT, sprite = SCORE.small_sprite, width = ROWS.miss_width, size = theme.type.body },
    }
    local x = area.x
    for _, column in ipairs(columns) do
        local box = { x = x, top = top, w = column.width }
        caption(s, column.label, box)
        local spec = { sprite = column.sprite, ref = column.ref, digit = SCORE.digits, saturate = true }
        s:number(spec, { x = x, baseline = baseline }, kit.arriving(ui.merge({ color = theme.color.text }, when.shown)))
        dash(s, column.size, { x = x, top = theme.text_top(baseline, column.size), w = column.width }, when)
        x = x + column.width + LAYOUT.gap
    end
end

local function rank_badge(s, when)
    local area = LAYOUT.score
    local box = { x = area.x + area.width - ROWS.badge_width, top = area.top + ROWS.label_drop, w = ROWS.badge_width, h = ROWS.badge_height }
    local size = theme.type.heading
    local text_box = { x = box.x, top = theme.centered_top(box, size), w = box.w }
    for level = 0, theme.rank_last do
        local key = theme.rank_key[level]
        local op = { ON_CHART, -UNPLAYED, props.option.RANK_F - level }
        local shown = { op = op, draw = when.shown.draw }
        s:rect(box, theme.rank[key], kit.arriving(shown))
        s:text({ text = rank_label(level), size = size, weight = "bold", align = "center" }, text_box, kit.arriving(ui.merge(shown, { color = theme.rank_ink[key] })))
    end
    s:rect(box, theme.color.panel, when.blank)
    dash(s, size, text_box, when, "center")
end

local function clear(s)
    local area = LAYOUT.score
    local size = theme.type.label
    local box = { x = area.x, top = area.top + ROWS.lamp_drop, w = area.width - ROWS.badge_width - LAYOUT.gap }
    local function place(name, color, op, graded)
        local spec = { text = name, size = size, overflow = FIT }
        s:text(spec, box, kit.arriving({ color = color, op = { ON_CHART, op, table.unpack(graded or {}) } }))
    end
    for index = 0, theme.clear_last do
        local lamp = theme.clear[index]
        if index == theme.graded_clear then
            parts.lamp_names(props.extension.SELECTED_BEST_ASSIST_GRADE_FIRST, function(name, color, graded)
                place(name, color, CLEAR_OPTIONS[index], graded)
            end)
        else
            place(lamp.name, theme.clear_text(lamp), CLEAR_OPTIONS[index])
        end
    end
end

local function next_rank_text(s, main_state, when)
    local area = LAYOUT.score
    local size = theme.type.label
    local box = { x = area.x + area.width - ROWS.badge_width, top = area.top + ROWS.lamp_drop, w = ROWS.badge_width }
    local spec = { value = next_rank(main_state), size = size, align = "right", overflow = FIT }
    s:text(spec, box, kit.arriving(ui.merge({ color = theme.color.sub }, when.shown)))
end

local function progress(s, when)
    local area = LAYOUT.score
    local bar = SCORE.progress
    local track = { x = area.x, top = area.top + ROWS.progress_drop, w = area.width, h = bar.height }
    s:rect(track, theme.color.panel, { op = { ON_CHART } })
    local filled = motion.grow_right(0, theme.time.slide, { color = theme.color.sub, op = { ON_CHART }, draw = when.shown.draw, timer = props.timer.SONGBAR_CHANGE })
    s:object("graph", kit.meter(s, props.rate.SCORE_RATE), track, filled)
    local step = area.width / RANK_STEPS
    for border = bar.first_tick, RANK_STEPS - 1 do
        local tick = { x = area.x + border * step - bar.tick / 2, top = track.top, w = bar.tick, h = track.h }
        s:rect(tick, theme.color.bg, { op = { ON_CHART } })
    end
    local label_top = track.top + track.h + bar.label_drop - theme.cap_height(theme.type.caption) / 2
    for level = 0, theme.rank_last do
        local first = level == 0 and 0 or level + bar.first_tick - 1
        local last = level + bar.first_tick
        local band = { x = area.x + first * step, top = label_top, w = (last - first) * step, align = "center" }
        caption(s, rank_label(level), band)
    end
end

local function stats(s, main_state, when)
    local area = LAYOUT.score
    local row = ROWS.pairs
    local size = theme.type.label
    local function place(column, line, label)
        local x = area.x + column * (row.width + row.gap)
        local baseline = area.top + row.baseline_drop[line]
        caption(s, label, { x = x, top = theme.text_top(baseline, theme.type.caption), w = row.width })
        return { x = x, baseline = baseline, top = theme.text_top(baseline, size), w = row.width }
    end
    local function count(column, line, label, ref, shown)
        local at = place(column, line, label)
        local spec = { sprite = SCORE.pairs.sprite, ref = ref, digit = SCORE.pairs.digits, saturate = true, align = "right" }
        s:number(spec, at, kit.arriving(ui.merge({ color = theme.color.text }, shown.shown)))
        dash(s, size, at, shown, "right")
    end
    count(0, 1, "MAX COMBO", props.number.MAXCOMBO, when)
    local rate_at = place(0, 2, i18n.t("common_rate"))
    local spec = { value = score_rate(main_state), size = size, align = "right", overflow = FIT }
    s:text(spec, rate_at, kit.arriving(ui.merge({ color = theme.color.text }, when.shown)))
    dash(s, size, rate_at, when, "right")
    count(1, 1, i18n.t("info_play_count"), props.number.PLAYCOUNT, COUNT_SHOWN)
    count(1, 2, i18n.t("info_clear_count"), props.number.CLEARCOUNT, COUNT_SHOWN)
end

local function judges(s, when)
    local area = LAYOUT.score
    local row = ROWS.judges
    local top = area.top + row.top_drop
    local baseline = top + row.baseline_rise
    local size = theme.type.caption
    for index, judge in ipairs(parts.judges) do
        local box = { x = area.x + (index - 1) * row.width, top = top, w = row.width, align = "center" }
        caption(s, judge.name, box, { color = judge.color })
        local spec = { sprite = SCORE.judges.sprite, ref = judge.count[1], digit = SCORE.judges.digits, saturate = true, align = "center" }
        local at = { x = box.x, baseline = baseline, w = box.w }
        s:number(spec, at, kit.arriving(ui.merge({ color = theme.color.text }, when.shown)))
        dash(s, size, { x = box.x, top = theme.text_top(baseline, size), w = box.w }, when, "center")
    end
end

local function my_score(s, main_state)
    section(s, LAYOUT.score, "info_my_score")
    local scored = score_shown(main_state)
    score_numbers(s, scored)
    rank_badge(s, scored)
    clear(s)
    next_rank_text(s, main_state, scored)
    progress(s, scored)
    stats(s, main_state, scored)
    judges(s, scored)
end

local function target_ex(main_state, step)
    local notes = main_state.number(props.number.TOTALNOTES)
    if not ui.has_value(notes) or notes <= 0 then
        return nil
    end
    return math.ceil(notes * EX_PER_NOTE * step / RANK_STEPS)
end

local function target_number(main_state, step)
    return function()
        return target_ex(main_state, step) or props.no_value.LOW
    end
end

local function target_gap(main_state, step)
    return function()
        local needed = target_ex(main_state, step)
        local ex = main_state.number(props.number.SCORE)
        if needed == nil or not ui.has_value(ex) or not ui.has_value(main_state.number(props.number.PERFECT_RATE)) then
            return ""
        end
        return string.format("%+d", ex - needed)
    end
end

local function target_reached(main_state, step, wanted)
    return function()
        local needed = target_ex(main_state, step)
        local ex = main_state.number(props.number.SCORE)
        local scored = ui.has_value(main_state.number(props.number.PERFECT_RATE))
        local reached = needed ~= nil and scored and ui.has_value(ex) and ex >= needed
        return reached == wanted
    end
end

local function clear_rate(main_state)
    return function()
        local plays = main_state.number(props.number.PLAYCOUNT)
        local clears = main_state.number(props.number.CLEARCOUNT)
        if not (ui.has_value(plays) and ui.has_value(clears)) or plays <= 0 then
            return NOTHING
        end
        return string.format("%.1f%%", clears * PERCENT / plays)
    end
end

local function targets(s, main_state)
    local area = LAYOUT.targets
    local title_area = { x = area.x, top = area.top, width = CELLS.rate_x - TARGETS.gutter }
    section(s, title_area, "info_score_targets", TARGETS.title_drop)
    local top = area.top + TARGETS.content_drop
    local size = theme.type.caption
    for index, step in ipairs(TARGETS.steps) do
        local medal = theme.medal_order[index]
        local x = area.x + (index - 1) * CELLS.pitch
        local chip = { x = x, top = top, w = CELLS.chip_width, h = CELLS.chip_height }
        local text_box = { x = chip.x, top = theme.centered_top(chip, size), w = chip.w }
        local name = { text = rank_label(theme.rank_last - (index - 1)), size = size, weight = "bold", align = "center" }
        local reached = { op = { ON_CHART }, draw = target_reached(main_state, step, true) }
        local short = { op = { ON_CHART }, draw = target_reached(main_state, step, false) }
        s:rect(chip, theme.medal[medal], kit.arriving(reached))
        s:text(name, text_box, kit.arriving(ui.merge(reached, { color = theme.medal_ink[medal] })))
        s:rect(chip, theme.color.raised, kit.arriving(short))
        s:text(name, text_box, kit.arriving(ui.merge(short, { color = theme.medal[medal] })))

        local sprite = s:sprite(TARGETS.sprite)
        local baseline = chip.top + (chip.h + sprite.cell_h - sprite.pad * 2) / 2
        local needed = { sprite = TARGETS.sprite, value = target_number(main_state, step), digit = TARGETS.digits, align = "right" }
        s:number(needed, { x = x + CELLS.number_x, baseline = baseline, w = CELLS.number_width }, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
        local gap = { value = target_gap(main_state, step), size = size, align = "right", overflow = FIT }
        local gap_box = { x = x + CELLS.diff_x, top = theme.text_top(baseline, size), w = CELLS.diff_width }
        s:text(gap, gap_box, kit.arriving({ color = theme.color.sub, op = { ON_CHART } }))
    end

    local rate_area = { x = area.x + CELLS.rate_x, top = area.top, width = area.width - CELLS.rate_x }
    section(s, rate_area, "info_clear_rate", TARGETS.title_drop)
    local value_size = theme.type.label
    local value_box = { x = rate_area.x, top = top + (CELLS.chip_height - theme.cap_height(value_size)) / 2, w = rate_area.width }
    local value = { value = clear_rate(main_state), size = value_size, align = "right", overflow = FIT }
    s:text(value, value_box, kit.arriving({ color = theme.color.text, op = { ON_CHART } }))
end

local function folder(s)
    local on = { op = { ON_FOLDER } }
    caption(s, i18n.t("common_charts"), { x = REGION.x, top = FOLDER.top, w = REGION.width }, on)
    local total = { sprite = FOLDER.count_sprite, ref = props.number.FOLDER_TOTALSONGS, digit = FOLDER.digits, saturate = true }
    s:number(total, { x = REGION.x, baseline = FOLDER.count_baseline }, kit.arriving({ color = theme.color.text, op = { ON_FOLDER } }))
    caption(s, i18n.t("common_clear_lamps"), { x = REGION.x, top = FOLDER.lamps_top, w = REGION.width }, on)
    for index = 1, theme.clear_last + 1 do
        local lamp = theme.clear[theme.clear_last + 1 - index]
        local column = math.floor((index - 1) / FOLDER.per_column)
        local x = FOLDER.columns[column + 1]
        local top = FOLDER.first_row + ((index - 1) % FOLDER.per_column) * FOLDER.pitch
        local row = { x = x, top = top, w = FOLDER.width, h = FOLDER.row_height }
        local text_top = theme.centered_top(row, FOLDER.size)
        s:rect({ x = x, top = top, w = FOLDER.chip_width, h = FOLDER.row_height }, lamp.color, on)
        local name = { x = x + FOLDER.name_x, top = text_top, w = FOLDER.width - FOLDER.name_x }
        s:text({ text = lamp.name, size = FOLDER.size, overflow = FIT }, name, { color = theme.color.sub, op = { ON_FOLDER } })
        local ref = props.number.FOLDER_MAX - (index - 1)
        local count = { sprite = FOLDER.sprite, ref = ref, digit = FOLDER.digits, saturate = true, align = "right" }
        s:number(count, { x = x, top = text_top, w = FOLDER.width }, kit.arriving({ color = theme.color.text, op = { ON_FOLDER } }))
    end
end

local function constraints(main_state)
    return function()
        local held = {}
        for _, constraint in ipairs(CONSTRAINTS) do
            if main_state.option(constraint.op) then
                table.insert(held, constraint.label)
            end
        end
        if #held == 0 then
            return NO_CONSTRAINT()
        end
        return table.concat(held, CONSTRAINT_JOINT)
    end
end

local function course(s, main_state)
    local on = { op = { ON_COURSE } }
    caption(s, i18n.t("common_stages"), { x = REGION.x, top = COURSE.top, w = REGION.width }, on)
    for stage_index = 1, COURSE.stages do
        local spec = { ref = props.text.COURSE1_TITLE + stage_index - 1, size = theme.type.body, overflow = FIT }
        local box = { x = REGION.x, top = COURSE.first_row + (stage_index - 1) * COURSE.pitch, w = REGION.width }
        s:text(spec, box, kit.sliding({ color = theme.color.text, op = { ON_COURSE } }))
    end
    caption(s, i18n.t("info_constraints"), { x = REGION.x, top = COURSE.constraints_top, w = REGION.width }, on)
    local spec = { value = constraints(main_state), size = theme.type.label, overflow = FIT }
    local box = { x = REGION.x, top = COURSE.constraints_value_top, w = REGION.width }
    s:text(spec, box, kit.arriving({ color = theme.color.sub, op = { ON_COURSE } }))
end

function info.build(s, main_state)
    head(s, main_state)
    stage(s)
    chart_data(s, main_state)
    my_score(s, main_state)
    radar.build(s, main_state)
    records.build(s, main_state)
    targets(s, main_state)
    folder(s)
    course(s, main_state)
end

return info
