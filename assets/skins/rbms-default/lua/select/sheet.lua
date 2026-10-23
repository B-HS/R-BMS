local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local panel = require("lua.select.panel")
local words = require("lua.select.words")
local inform = require("lua.select.inform")
local i18n = require("lua.i18n")

local sheet = {}

local t = i18n.t

local TOP = 120
local LEFT = { x = 64, w = 408 }
local MAIN = { x = 504, w = 1352 }
local INFO = { tag_w = 232, tag_h = 32, body_top = 156, body_h = 80, inset = 24, title_drop = 14, line_drop = 48 }
local KEYS = { first = 836, pitch = 112, width = 92, height = 92, black_top = 264, white_top = 312, count = 7 }
local TURNTABLE = { x = 626, top = 284, w = 120, h = 120, pitch = 30 }
local LANE = { over = 248, high = 424, middle = 444, low = 464, detour = 766, beside = 1576 }
local BOX_TOP = 492
local PLAY_BOX = { width = 280, gap = 24 }
local DETAIL_BOX = { narrow = 160, wide = 280, field = 340 }
local LEFT_GAP = 22
local TARGET_ROWS = { pitch = 29, height = 27 }
local HELP = { tag_h = 32, gap = 12, name_size = theme.type.label, line_pitch = 26 }
local GRADE_ROW = { pitch = 40, gauge_w = 168 }
local CAPTION_PITCH = theme.type.caption + 8
local NO_NAME = "-"
local SCORE_ALGORITHM = -1

local COLUMN = theme.panel.column

local function key_x(index)
    return KEYS.first + (index - 1) * KEYS.pitch
end

local function is_black(index)
    return index % 2 == 0
end

local function key_box(index)
    local top = is_black(index) and KEYS.black_top or KEYS.white_top
    return { x = key_x(index) - KEYS.width / 2, top = top, w = KEYS.width, h = KEYS.height }
end

local function key_bottom(index)
    local box = key_box(index)
    return box.top + box.h
end

local function turntable_middle()
    return TURNTABLE.top + TURNTABLE.h / 2
end

local function down(index, lane, to_x)
    local x = key_x(index)
    return { { x, key_bottom(index) }, { x, lane }, { to_x, lane }, { to_x, BOX_TOP } }
end

local function straight(index)
    local x = key_x(index)
    return { { x, key_bottom(index) }, { x, BOX_TOP } }
end

local function from_turntable()
    local middle = turntable_middle()
    return { { TURNTABLE.x, middle }, { LEFT.x + LEFT.w, middle } }
end

local function centered(x, w)
    return { x = x - w / 2, top = BOX_TOP, w = w }
end

local function indexed(main_state, id)
    return function()
        return main_state.event_index(id)
    end
end

local function flagged(main_state, id)
    return function()
        return main_state.option(id) and 1 or 0
    end
end

local function listed(names, order)
    local values = {}
    for index, value in ipairs(order) do
        values[index] = { value = value, name = names[value] }
    end
    return values
end

local function numbered(names, first, last)
    local order = {}
    for value = first, last do
        table.insert(order, value)
    end
    return listed(names, order)
end

local OFF_ON = { 0, 1 }

local function option(spec)
    local read = spec.read
    spec.is = function(value)
        return function()
            return read() == value
        end
    end
    spec.is_not = function(value)
        return function()
            return read() ~= value
        end
    end
    local names = {}
    for _, entry in ipairs(spec.values) do
        names[entry.value] = entry.name
    end
    spec.watch = {
        read = read,
        tell = function(now)
            return spec.title, names[now] or NO_NAME, spec.notes[now] or ""
        end,
    }
    return spec
end

local function switch(main_state, spec, read)
    spec.read = read
    spec.values = listed(words.switch, OFF_ON)
    return option(spec)
end

local function out_of_reach(index)
    return { props.extension.UNREACHABLE_KEY_FIRST + index - 1 }
end

local function joined(lines)
    return function()
        local texts = {}
        for index, line in ipairs(lines) do
            texts[index] = line()
        end
        return i18n.join(texts)
    end
end

local function keyboard(page, labels)
    for index = 1, KEYS.count do
        local label = labels[index]
        page:key(key_box(index), label.text, label.color, is_black(index), out_of_reach(index))
    end
end

local function turntable(page, action, color)
    local box = { x = TURNTABLE.x, top = TURNTABLE.top, w = TURNTABLE.w, h = TURNTABLE.h }
    page:key(box, "", color, true)
    local size = theme.type.caption
    local first = box.top + (box.h - theme.cap_height(size) - TURNTABLE.pitch) / 2
    local spec = { size = size, weight = "bold", align = "center", overflow = props.text_overflow.SHRINK }
    page:write(ui.merge(spec, { text = t("sheet_turntable") }), { x = box.x, top = first, w = box.w }, theme.color.sub)
    page:write(ui.merge(spec, { text = action }), { x = box.x, top = first + TURNTABLE.pitch, w = box.w }, color)
end

local function information(page, main_state, def)
    local tag = { x = MAIN.x, top = TOP, w = INFO.tag_w, h = INFO.tag_h }
    page:tag(tag, t("sheet_information"), theme.panel.pick, theme.panel.pick_ink)
    local caption = theme.type.caption
    local hold = { text = def.hold, size = caption, align = "right", overflow = props.text_overflow.SHRINK }
    local right = { x = tag.x + tag.w + INFO.inset, top = theme.centered_top(tag, caption), w = MAIN.w - tag.w - INFO.inset }
    page:write(hold, right, theme.color.muted)
    local body = { x = MAIN.x, top = INFO.body_top, w = MAIN.w, h = INFO.body_h }
    page:body(body, theme.panel.pick)
    local observe = inform.observer(main_state, def)
    local size = theme.type.label
    local inner = { x = body.x + INFO.inset, w = body.w - INFO.inset * 2 }
    local title = { value = inform.title(observe), size = size, weight = "bold", overflow = props.text_overflow.SHRINK }
    page:write(title, { x = inner.x, top = body.top + INFO.title_drop, w = inner.w }, theme.color.text)
    local value = { value = inform.value(observe), size = size, weight = "bold", align = "right", overflow = props.text_overflow.SHRINK }
    page:write(value, { x = inner.x, top = body.top + INFO.title_drop, w = inner.w }, theme.panel.pick)
    local line = { value = inform.line(observe), size = size, overflow = props.text_overflow.SHRINK }
    page:write(line, { x = inner.x, top = body.top + INFO.line_drop, w = inner.w }, theme.color.sub)
end

local function boxed_list(page, box, spec)
    local frame = page:list(box, spec)
    if spec.label then
        page:column_label(frame, spec.label, spec.color)
    end
    return frame
end

local function help_tag(page, box, title, color)
    page:body(box, color)
    page:tag({ x = box.x, top = box.top, w = box.w, h = HELP.tag_h }, title, color, theme.panel.pick_ink)
    return box.top + HELP.tag_h + HELP.gap
end

local function target(page, color)
    local rows = props.target_reach * 2 + 1
    local height = panel.HEADER.height + panel.ROWS.first + rows * TARGET_ROWS.pitch + panel.ROWS.foot
    local frame = { x = LEFT.x, top = TOP, w = LEFT.w, h = height }
    page:body(frame, color)
    page:heading(frame, { t("common_target") }, color)
    local size = theme.type.label
    local refs = {}
    for step = 1, props.target_reach do
        table.insert(refs, props.text.TARGET_AHEAD_FIRST + step - 1)
    end
    table.insert(refs, props.text.TARGET_NAME)
    for step = 1, props.target_reach do
        table.insert(refs, props.text.TARGET_BEHIND_FIRST + step - 1)
    end
    for index, ref in ipairs(refs) do
        local top = frame.top + panel.HEADER.height + panel.ROWS.first + (index - 1) * TARGET_ROWS.pitch
        local row = { x = frame.x + panel.ROWS.inset, top = top, w = frame.w - panel.ROWS.inset * 2, h = TARGET_ROWS.height }
        local line = { x = row.x + panel.ROWS.inset, top = theme.centered_top(row, size), w = row.w - panel.ROWS.inset * 2 }
        local spec = { ref = ref, size = size, overflow = props.text_overflow.SHRINK }
        if ref == props.text.TARGET_NAME then
            page:fill(row, theme.panel.pick)
            page:write(ui.merge(spec, { weight = "bold" }), line, theme.panel.pick_ink)
        else
            page:write(spec, line, theme.color.sub)
        end
    end
    return frame
end

local function gauge_help(page, main_state, top, color)
    local height = HELP.tag_h + HELP.gap * 3 + HELP.name_size + HELP.line_pitch * #words.gauge[0]
    local box = { x = LEFT.x, top = top, w = LEFT.w, h = height }
    local read = indexed(main_state, props.index.GAUGE_1P)
    local first = help_tag(page, box, t("sheet_groove_gauge"), color)
    local inner = { x = box.x + panel.ROWS.inset * 2, w = box.w - panel.ROWS.inset * 4 }
    local name = { size = HELP.name_size, weight = "bold", overflow = props.text_overflow.SHRINK }
    name.value = function()
        return theme.gauge_name[read()] or NO_NAME
    end
    page:write(name, { x = inner.x, top = first, w = inner.w }, color)
    for line = 1, #words.gauge[0] do
        local spec = { size = theme.type.caption, overflow = props.text_overflow.SHRINK }
        spec.value = function()
            local note = (words.gauge[read()] or {})[line]
            return note and note() or ""
        end
        page:write(spec, { x = inner.x, top = first + HELP.name_size + HELP.gap + (line - 1) * HELP.line_pitch, w = inner.w }, theme.color.sub)
    end
end

local function play_page(page, main_state)
    local random = option({
        title = "RANDOM",
        name = { "RANDOM" },
        label = t("sheet_label_style"),
        color = COLUMN.orange,
        read = indexed(main_state, props.index.RANDOM_1P),
        values = numbered(theme.random_name, 0, #theme.random_name),
        notes = words.random,
    })
    local gauge = option({
        title = t("common_gauge"),
        name = { t("common_gauge") },
        label = t("common_gauge"),
        color = COLUMN.green,
        read = indexed(main_state, props.index.GAUGE_1P),
        values = numbered(theme.gauge_name, 0, 5),
        notes = {},
    })
    for value, lines in pairs(words.gauge) do
        gauge.notes[value] = joined(lines)
    end
    local fix = option({
        title = "HI-SPEED FIX",
        name = { "HI-SPEED FIX" },
        label = t("sheet_label_speed"),
        color = COLUMN.violet,
        read = indexed(main_state, props.index.HSFIX),
        values = numbered(words.fix, 0, #words.fix),
        notes = words.fix_notes,
    })
    local target_color = COLUMN.sky
    local target_watch = {
        read = function()
            return main_state.text(props.text.TARGET_NAME)
        end,
        tell = function(now)
            return t("common_target"), now, words.target
        end,
    }

    local def = page.page
    def.watch = { random.watch, gauge.watch, fix.watch, target_watch }
    information(page, main_state, def)

    local down, up = t("sheet_key_down"), t("sheet_key_up")
    keyboard(page, {
        { text = down, color = random.color },
        { text = up, color = random.color },
        { text = down, color = gauge.color },
        { text = up, color = gauge.color },
        { text = down, color = fix.color },
        { text = up, color = fix.color },
        { text = down, color = random.color },
    })
    turntable(page, t("common_target"), target_color)

    local columns = { random, gauge, fix }
    local width, gap = PLAY_BOX.width, PLAY_BOX.gap
    local first_x = MAIN.x + MAIN.w / 2 - (#columns * width + (#columns - 1) * gap) / 2 + width / 2
    local box_x = {}
    for index, spec in ipairs(columns) do
        local first, second = index * 2 - 1, index * 2
        local pair = (key_x(first) + key_x(second)) / 2
        box_x[index] = first_x + (index - 1) * (width + gap)
        page:wire({ { key_x(first), key_bottom(first) }, { key_x(first), LANE.high } }, spec.color)
        page:wire({ { key_x(second), key_bottom(second) }, { key_x(second), LANE.high } }, spec.color)
        page:wire({ { key_x(first), LANE.high }, { key_x(second), LANE.high } }, spec.color)
        page:wire({ { pair, LANE.high }, { pair, LANE.low }, { box_x[index], LANE.low }, { box_x[index], BOX_TOP } }, spec.color)
        boxed_list(page, centered(box_x[index], width), spec)
    end
    local last = KEYS.count
    local over = { { key_x(last), key_box(last).top }, { key_x(last), LANE.over }, { LANE.detour, LANE.over } }
    table.insert(over, { LANE.detour, LANE.low })
    table.insert(over, { box_x[1], LANE.low })
    page:wire(over, random.color)

    page:wire(from_turntable(), target_color)
    local list = target(page, target_color)
    gauge_help(page, main_state, list.top + list.h + LEFT_GAP, gauge.color)
end

local function assist_help(page, top, color)
    local rule = words.assist_rule
    local box = { x = LEFT.x, top = top, w = LEFT.w }
    local inner = { x = box.x + panel.ROWS.inset * 2, w = box.w - panel.ROWS.inset * 4, align = "left" }
    local lead_top = top + HELP.tag_h + HELP.gap
    local rows_top = lead_top + #rule.lead * CAPTION_PITCH + HELP.gap
    local foot_top = rows_top + #rule.rows * GRADE_ROW.pitch + HELP.gap
    box.h = foot_top + #rule.foot * CAPTION_PITCH + HELP.gap - top
    help_tag(page, box, rule.title, color)
    page:caption_lines(ui.merge(inner, { top = lead_top }), rule.lead, theme.color.sub)
    for index, row in ipairs(rule.rows) do
        local row_top = rows_top + (index - 1) * GRADE_ROW.pitch
        local size = theme.type.label
        local gauge = { text = row.gauge, size = theme.type.caption, overflow = props.text_overflow.SHRINK }
        page:write(gauge, { x = inner.x, top = row_top + theme.cap_height(size) - theme.cap_height(theme.type.caption), w = GRADE_ROW.gauge_w }, theme.color.muted)
        local grade = theme.assist_grade[row.grade]
        local name = { text = grade.name, size = size, weight = "bold", align = "right" }
        page:write(name, { x = inner.x + GRADE_ROW.gauge_w, top = row_top, w = inner.w - GRADE_ROW.gauge_w }, grade.color)
    end
    page:caption_lines(ui.merge(inner, { top = foot_top }), rule.foot, theme.color.muted)
end

local function assist_page(page, main_state)
    local ext = props.extension
    local ex_judge = switch(main_state, {
        title = "EX JUDGE",
        name = { "EX JUDGE" },
        label = t("common_judge"),
        color = COLUMN.locked,
        notes = words.ex_judge,
        lines = { t("sheet_read_only_1"), t("sheet_read_only_2") },
    }, indexed(main_state, props.index.ASSIST_EXJUDGE))
    local constant = switch(main_state, {
        title = "CONSTANT",
        name = { "CONSTANT" },
        label = t("sheet_label_speed"),
        color = COLUMN.orange,
        notes = words.constant,
    }, indexed(main_state, props.index.ASSIST_CONSTANT))
    local scratch = switch(main_state, {
        title = "SCRATCH AUTO",
        name = { "SCRATCH", "AUTO" },
        label = t("sheet_label_assist"),
        color = COLUMN.green,
        notes = words.scratch_auto,
        lines = { t("sheet_scratch_auto_1"), t("sheet_scratch_auto_2") },
        line_color = COLUMN.green,
    }, flagged(main_state, ext.SCRATCH_AUTO_ON))
    local legacy = switch(main_state, {
        title = "LEGACY NOTE",
        name = { "LEGACY", "NOTE" },
        label = t("sheet_label_assist"),
        color = COLUMN.violet,
        notes = words.legacy,
    }, indexed(main_state, props.index.ASSIST_LEGACY))
    local keep = switch(main_state, {
        title = "KEEP GREEN NUMBER",
        name = { "KEEP GREEN", "NUMBER" },
        label = t("sheet_label_speed"),
        color = COLUMN.sky,
        notes = words.keep_green,
    }, flagged(main_state, ext.KEEP_GREEN_NUMBER_ON))
    local cover = switch(main_state, {
        title = t("sheet_lane_cover"),
        name = { t("sheet_lane_cover_1"), t("sheet_lane_cover_2") },
        label = t("sheet_label_range"),
        color = COLUMN.orange,
        notes = words.lane_cover,
    }, flagged(main_state, ext.LANE_COVER_ON))
    local lift = switch(main_state, {
        title = "LIFT",
        name = { "LIFT" },
        label = t("sheet_label_range"),
        color = COLUMN.green,
        notes = words.lift,
    }, flagged(main_state, ext.LIFT_ON))
    local hidden = switch(main_state, {
        title = "HIDDEN+",
        name = { "HIDDEN+" },
        color = COLUMN.violet,
        notes = words.hidden,
        lines = { t("sheet_hidden_1"), t("sheet_hidden_2") },
    }, flagged(main_state, ext.HIDDEN_ON))

    local columns = { ex_judge, constant, scratch, legacy, keep, cover, lift }
    local def = page.page
    def.watch = {}
    for _, spec in ipairs(columns) do
        table.insert(def.watch, spec.watch)
    end
    table.insert(def.watch, hidden.watch)
    information(page, main_state, def)

    local labels = {}
    for index, spec in ipairs(columns) do
        labels[index] = { text = "ON/OFF", color = spec.color }
    end
    labels[1].text = t("sheet_key_locked")
    keyboard(page, labels)
    turntable(page, "ON/OFF", hidden.color)

    local pitch = (MAIN.w - LEFT_GAP / 2) / #columns
    local width = pitch - LEFT_GAP / 2 * 1.5
    local middle = (#columns + 1) / 2
    local lanes = { LANE.high, LANE.middle, LANE.low }
    for index, spec in ipairs(columns) do
        local x = key_x(middle) + (index - middle) * pitch
        local reach = math.abs(index - middle)
        if reach == 0 then
            page:wire(straight(index), spec.color)
        else
            page:wire(down(index, lanes[#lanes + 1 - reach], x), spec.color)
        end
        page:dim(boxed_list(page, centered(x, width), spec), out_of_reach(index))
    end

    page:wire(from_turntable(), hidden.color)
    local box = boxed_list(page, { x = LEFT.x, top = TOP, w = LEFT.w }, hidden)
    assist_help(page, box.top + box.h + LEFT_GAP, scratch.color)
end

local function shift_help(page, main_state, top, color)
    local box = { x = LEFT.x, top = top, w = LEFT.w }
    local inner = { x = box.x + panel.ROWS.inset * 2, w = box.w - panel.ROWS.inset * 4, align = "left" }
    local read = indexed(main_state, props.index.GAUGEAUTOSHIFT)
    local entry_top = top + HELP.tag_h + HELP.gap
    local tops = {}
    for value = 0, #words.shift do
        tops[value] = entry_top
        entry_top = entry_top + HELP.name_size + HELP.gap + #words.shift_notes[value] * CAPTION_PITCH
    end
    box.h = entry_top - top
    help_tag(page, box, t("sheet_gauge_auto_shift"), color)
    for value = 0, #words.shift do
        local name = { text = words.shift[value], size = HELP.name_size, weight = "bold", overflow = props.text_overflow.SHRINK }
        local line = { x = inner.x, top = tops[value], w = inner.w }
        page:write(name, line, theme.panel.pick, {
            draw = function()
                return read() == value
            end,
        })
        page:write(name, line, theme.color.text, {
            draw = function()
                return read() ~= value
            end,
        })
        local notes = ui.merge(inner, { top = tops[value] + HELP.name_size + HELP.gap })
        page:caption_lines(notes, words.shift_notes[value], theme.color.muted)
    end
end

local function display_time(main_state)
    local held = function()
        local duration = main_state.number(props.number.DURATION)
        if ui.has_value(duration) then
            return duration
        end
    end
    local kept = function()
        return main_state.option(props.extension.KEEP_GREEN_NUMBER_ON)
    end
    return {
        read = function()
            return held() or props.no_value.LOW
        end,
        first = function()
            local duration = held()
            if duration ~= nil then
                return string.format("%d MS", duration)
            end
            if kept() then
                return NO_NAME
            end
            return words.display_time.off
        end,
        second = function()
            if held() == nil then
                return ""
            end
            return tostring(main_state.number(props.number.DURATION_GREEN))
        end,
        held = function()
            return held() ~= nil
        end,
        away = function()
            return held() == nil and kept()
        end,
        off = function()
            return held() == nil and not kept()
        end,
        tell = function(now)
            local name = words.display_time.name
            if not ui.has_value(now) then
                if kept() then
                    return name, NO_NAME, words.display_time.away_note
                end
                return name, words.display_time.off, words.display_time.off_note
            end
            local green = main_state.number(props.number.DURATION_GREEN)
            return name, string.format("%d MS", now), string.format(words.display_time.green, green)
        end,
    }
end

local function judge_timing(main_state)
    local read = function()
        return main_state.number(props.number.JUDGETIMING)
    end
    local shown = function(value)
        if not ui.has_value(value) then
            return NO_NAME
        end
        if value == 0 then
            return "0 MS"
        end
        return string.format("%+d MS", value)
    end
    local said = words.judge_timing
    return {
        read = read,
        first = function()
            return shown(read())
        end,
        tell = function(now)
            local value = shown(now)
            if not ui.has_value(now) or now == 0 then
                return said.name, value, said.zero
            end
            if now > 0 then
                return said.name, value, function()
                    return said.earlier(now)
                end
            end
            return said.name, value, function()
                return said.later(-now)
            end
        end,
    }
end

local function detail_page(page, main_state)
    local bga = option({
        title = "BGA",
        name = { "BGA" },
        label = t("sheet_label_display"),
        color = COLUMN.orange,
        read = indexed(main_state, props.index.BGA),
        values = listed(words.bga, { 0, 2 }),
        notes = words.bga_notes,
    })
    local shift = option({
        title = t("sheet_gauge_auto_shift"),
        name = { t("sheet_gauge_auto_shift_1"), t("sheet_gauge_auto_shift_2") },
        label = t("common_gauge"),
        color = COLUMN.green,
        read = indexed(main_state, props.index.GAUGEAUTOSHIFT),
        values = numbered(words.shift, 0, #words.shift),
        notes = {},
    })
    for value, lines in pairs(words.shift_notes) do
        shift.notes[value] = joined(lines)
    end
    local adjust = switch(main_state, {
        title = t("sheet_auto_adjust"),
        name = { t("sheet_auto_adjust_1"), t("sheet_auto_adjust_2") },
        label = t("common_timing"),
        color = COLUMN.violet,
        notes = words.auto_adjust,
    }, indexed(main_state, props.index.TIMING_AUTO_ADJUST))
    local algorithm = option({
        title = t("sheet_judge_algorithm"),
        name = { t("sheet_judge_algorithm") },
        color = COLUMN.violet,
        read = indexed(main_state, props.index.JUDGEALGORITHM),
        values = numbered(words.algorithm, 0, #words.algorithm),
        notes = words.algorithm_notes,
    })
    local duration = display_time(main_state)
    local timing = judge_timing(main_state)
    local duration_color = COLUMN.orange
    local timing_color = COLUMN.sky

    local def = page.page
    def.watch = { bga.watch, shift.watch, adjust.watch, duration, timing, algorithm.watch }
    information(page, main_state, def)

    local left, right = t("sheet_key_left"), t("sheet_key_right")
    keyboard(page, {
        { text = "ON/OFF", color = bga.color },
        { text = t("sheet_key_down"), color = shift.color },
        { text = "ON/OFF", color = adjust.color },
        { text = left, color = duration_color },
        { text = left, color = timing_color },
        { text = right, color = duration_color },
        { text = right, color = timing_color },
    })
    turntable(page, t("sheet_key_up_down"), algorithm.color)

    local narrow, wide, field = DETAIL_BOX.narrow, DETAIL_BOX.wide, DETAIL_BOX.field
    local gap = (MAIN.w - narrow * 2 - wide - field * 2) / 4
    local x = MAIN.x
    local function next_box(width)
        local box = { x = x, top = BOX_TOP, w = width }
        x = x + width + gap
        return box
    end
    local bga_box, shift_box, adjust_box = next_box(narrow), next_box(wide), next_box(narrow)
    local timing_box, duration_box = next_box(field), next_box(field)
    local middle = function(box)
        return box.x + box.w / 2
    end

    page:wire(down(1, LANE.high, middle(bga_box)), bga.color)
    boxed_list(page, bga_box, bga)
    page:wire(down(2, LANE.middle, middle(shift_box)), shift.color)
    boxed_list(page, shift_box, shift)
    page:wire(down(3, LANE.high, middle(adjust_box)), adjust.color)
    boxed_list(page, adjust_box, adjust)

    page:wire({ { key_x(5), key_bottom(5) }, { key_x(5), LANE.high }, { key_x(7), LANE.high }, { key_x(7), key_bottom(7) } }, timing_color)
    page:wire({ { middle(timing_box), LANE.high }, { middle(timing_box), BOX_TOP } }, timing_color)
    local timing_spec = { name = { t("sheet_judge_timing") }, color = timing_color, first = timing.first }
    timing_spec.lines = words.judge_timing.advice
    timing_spec.line_color = theme.panel.caution
    page:column_label(page:field(timing_box, timing_spec), t("common_timing"), timing_color)

    local over = { { key_x(4), key_box(4).top }, { key_x(4), LANE.over }, { LANE.beside, LANE.over } }
    table.insert(over, { LANE.beside, LANE.low })
    table.insert(over, { middle(duration_box), LANE.low })
    table.insert(over, { middle(duration_box), BOX_TOP })
    page:wire(over, duration_color)
    page:wire({ { key_x(6), key_box(6).top }, { key_x(6), LANE.over } }, duration_color)
    local duration_spec = { name = { t("sheet_note_display_time") }, color = duration_color, first = duration.first, second = duration.second }
    duration_spec.reserve = #words.display_time.off_lines
    local duration_frame, duration_lines = page:field(duration_box, duration_spec)
    page:caption_lines(duration_lines, words.display_time.off_lines, theme.panel.caution, { draw = duration.off })
    page:caption_lines(duration_lines, words.display_time.away_lines, theme.color.muted, { draw = duration.away })
    page:caption_lines(duration_lines, words.display_time.held_lines, theme.color.muted, { draw = duration.held })
    page:column_label(duration_frame, t("sheet_label_speed"), duration_color)

    page:wire(from_turntable(), algorithm.color)
    algorithm.reserve = 1
    local box, lines = page:list({ x = LEFT.x, top = TOP, w = LEFT.w }, algorithm)
    page:caption_lines(lines, { t("sheet_algorithm_score") }, theme.panel.caution, { draw = algorithm.is(SCORE_ALGORITHM) })
    page:caption_lines(lines, { t("sheet_algorithm_step") }, theme.color.muted, { draw = algorithm.is_not(SCORE_ALGORITHM) })
    shift_help(page, main_state, box.top + box.h + LEFT_GAP, shift.color)
end

local PAGES = {
    { hold = t("sheet_hold_start"), option = props.option.PANEL1, on = props.timer.PANEL1_ON, off = props.timer.PANEL1_OFF, build = play_page },
    { hold = t("sheet_hold_select"), option = props.option.PANEL2, on = props.timer.PANEL2_ON, off = props.timer.PANEL2_OFF, build = assist_page },
    {
        hold = t("sheet_hold_detail"),
        option = props.option.PANEL3,
        on = props.timer.PANEL3_ON,
        off = props.timer.PANEL3_OFF,
        build = detail_page,
    },
}

function sheet.build(s, main_state)
    for index, def in ipairs(PAGES) do
        local page = panel.new(s, ui.merge(def, { guide = words.guide[index] }))
        page:veil()
        def.build(page, main_state)
    end
end

return sheet
