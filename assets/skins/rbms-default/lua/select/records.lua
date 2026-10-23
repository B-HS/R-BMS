local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local i18n = require("lua.i18n")
local kit = require("lua.select.kit")

local records = {}

local ON_CHART = props.option.SONGBAR
local RANKING_ON = props.extension.RANKING_MODE_ON
local LISTING_OPS = { ON_CHART, -RANKING_ON }
local RANKING_OPS = { ON_CHART, RANKING_ON }
local NOTHING = "-"
local BEST_ROW = 0
local DATE_LENGTH = 10
local PLAYER_YOU = 1
local PLAYER_RIVAL = 2

local IR_RANKING = {
    name_first = 120,
    ex_first = 380,
    place_first = 390,
    player_first = 380,
    clear_first = 390,
}

local AREA = theme.select_layout.records

local LAYOUT = {
    title_drop = 12,
    rows_top = AREA.top + AREA.header_height,
    row_inset = 2,
    lamp = { x = 0, width = 6, inset = 6 },
    label = { x = 14, width = 100 },
    badge = { x = 118, width = 52, inset = 6 },
    ex = { x = 176, width = 80, sprite = "num24", digits = 5 },
    miss = { x = 264, width = 48, sprite = "num18", digits = 4 },
    place = { x = 14, width = 36, sprite = "num24", digits = 3 },
    name = { x = 60, width = 150, size = theme.type.label },
    ranking_ex = { x = 216, width = 96, sprite = "num24", digits = 5 },
}

local function row_box(row)
    local top = LAYOUT.rows_top + row * AREA.row_height
    return { x = AREA.x, top = top + LAYOUT.row_inset, w = AREA.width, h = AREA.row_height - LAYOUT.row_inset * 2 }
end

local function cell(row, column, inset)
    local box = row_box(row)
    local trim = inset or 0
    return { x = box.x + column.x, top = box.top + trim, w = column.width, h = box.h - trim * 2 }
end

local function lamp_set(s, id, selector)
    local strip = s:sprite(theme.sprite("lamp_strip"))
    local images = {}
    for index = 0, strip.divx - 1 do
        local image = { id = id .. ".lamp" .. index, src = strip.src, x = strip.x + index * strip.cell_w, y = strip.y, w = strip.cell_w, h = strip.h, divx = 1, divy = 1 }
        images[index + 1] = s:object("image", image)
    end
    return s:object("imageset", ui.merge({ id = id, images = images }, selector))
end

local function centered_number(s, spec, box, opts)
    local sprite = s:sprite(spec.sprite)
    local rise = sprite.cell_h - sprite.pad * 2
    local baseline = box.top + (box.h + rise) / 2
    s:number(spec, { x = box.x, baseline = baseline, w = box.w }, opts)
end

local function centered_text(s, spec, box, opts)
    local placed = { x = box.x, top = theme.centered_top(box, spec.size), w = box.w }
    s:text(spec, placed, opts)
end

local function level_is(main_state, row, level)
    local ref = props.extension_number.MY_RECORD_DJ_LEVEL_FIRST + row
    return function()
        return main_state.number(ref) == level
    end
end

local function holds_row(main_state, row)
    local ref = props.extension_number.MY_RECORD_EX_FIRST + row
    return function()
        return ui.has_value(main_state.number(ref))
    end
end

local function holds_no_best(main_state)
    local ref = props.extension_number.MY_RECORD_EX_FIRST + BEST_ROW
    return function()
        return not ui.has_value(main_state.number(ref))
    end
end

local function date_of(main_state, row)
    local ref = props.extension_text.MY_RECORD_DATE_FIRST + row
    return function()
        return string.sub(main_state.text(ref), 1, DATE_LENGTH)
    end
end

local function record_row(s, main_state, row)
    local on = { op = LISTING_OPS }
    local set = lamp_set(s, "set.record.lamp" .. row, { value = props.extension_number.MY_RECORD_LAMP_FIRST + row })
    s:place(set, cell(row, LAYOUT.lamp, LAYOUT.lamp.inset), kit.arriving(on))

    local label_box = cell(row, LAYOUT.label)
    local label_size = theme.type.caption
    if row == BEST_ROW then
        local best = { value = i18n.t("info_records_best"), size = label_size, overflow = props.text_overflow.SHRINK }
        centered_text(s, best, label_box, kit.arriving({ color = theme.color.text, op = LISTING_OPS, draw = holds_row(main_state, row) }))
        local dash = { text = NOTHING, size = label_size }
        centered_text(s, dash, label_box, { color = theme.color.muted, op = LISTING_OPS, draw = holds_no_best(main_state) })
    else
        local date = { value = date_of(main_state, row), size = label_size, overflow = props.text_overflow.SHRINK }
        centered_text(s, date, label_box, kit.arriving(ui.merge(on, { color = theme.color.sub })))
    end

    local badge = cell(row, LAYOUT.badge, LAYOUT.badge.inset)
    for level = 0, theme.rank_last do
        local key = theme.rank_key[level]
        local when = { op = LISTING_OPS, draw = level_is(main_state, row, level) }
        s:rect(badge, theme.rank[key], kit.arriving(when))
        local name = { text = string.upper(key), size = label_size, weight = "bold", align = "center" }
        centered_text(s, name, badge, kit.arriving(ui.merge(when, { color = theme.rank_ink[key] })))
    end

    local ex = { sprite = LAYOUT.ex.sprite, ref = props.extension_number.MY_RECORD_EX_FIRST + row, digit = LAYOUT.ex.digits, align = "right" }
    centered_number(s, ex, cell(row, LAYOUT.ex), kit.arriving(ui.merge(on, { color = theme.color.text })))
    local miss = { sprite = LAYOUT.miss.sprite, ref = props.extension_number.MY_RECORD_MISS_COUNT_FIRST + row, digit = LAYOUT.miss.digits, align = "right" }
    centered_number(s, miss, cell(row, LAYOUT.miss), kit.arriving(ui.merge(on, { color = theme.color.sub })))
end

local function player_is(main_state, row, player)
    local ref = IR_RANKING.player_first + row
    return function()
        return main_state.event_index(ref) == player
    end
end

local function player_is_other(main_state, row)
    local ref = IR_RANKING.player_first + row
    return function()
        local player = main_state.event_index(ref)
        return player ~= PLAYER_YOU and player ~= PLAYER_RIVAL
    end
end

local function ranking_row(s, main_state, row)
    local on = { op = RANKING_OPS }
    local you = player_is(main_state, row, PLAYER_YOU)
    s:rect(row_box(row), theme.ranking.you_face, { op = RANKING_OPS, draw = you })

    local set = lamp_set(s, "set.ranking.lamp" .. row, { ref = IR_RANKING.clear_first + row })
    s:place(set, cell(row, LAYOUT.lamp, LAYOUT.lamp.inset), kit.arriving(on))

    local place = { sprite = LAYOUT.place.sprite, ref = IR_RANKING.place_first + row, digit = LAYOUT.place.digits, align = "right" }
    centered_number(s, place, cell(row, LAYOUT.place), kit.arriving(ui.merge(on, { color = theme.color.sub })))

    local name = { ref = IR_RANKING.name_first + row, size = LAYOUT.name.size, overflow = props.text_overflow.SHRINK }
    local name_box = cell(row, LAYOUT.name)
    centered_text(s, name, name_box, kit.arriving({ color = theme.ranking.you, op = RANKING_OPS, draw = you }))
    centered_text(s, name, name_box, kit.arriving({ color = theme.ranking.rival, op = RANKING_OPS, draw = player_is(main_state, row, PLAYER_RIVAL) }))
    centered_text(s, name, name_box, kit.arriving({ color = theme.color.text, op = RANKING_OPS, draw = player_is_other(main_state, row) }))

    local ex = { sprite = LAYOUT.ranking_ex.sprite, ref = IR_RANKING.ex_first + row, digit = LAYOUT.ranking_ex.digits, align = "right" }
    centered_number(s, ex, cell(row, LAYOUT.ranking_ex), kit.arriving(ui.merge(on, { color = theme.color.text })))
end

local function heading(s, key, column, ops)
    local box = { x = AREA.x + column.x, top = AREA.top + LAYOUT.title_drop, w = column.width }
    local spec = { value = i18n.t(key), size = theme.type.caption, align = "right", overflow = props.text_overflow.SHRINK }
    s:text(spec, box, { color = theme.color.muted, op = ops })
end

function records.build(s, main_state)
    local on = { op = { ON_CHART } }
    s:rect({ x = AREA.x, top = AREA.top, w = AREA.width, h = theme.grid.rule }, theme.color.line, on)
    local title_box = { x = AREA.x, top = AREA.top + LAYOUT.title_drop, w = AREA.width }
    s:text({ value = i18n.t("info_my_records"), size = theme.type.caption }, title_box, { color = theme.color.sub, op = LISTING_OPS })
    s:text({ value = i18n.t("info_ir_ranking"), size = theme.type.caption }, title_box, { color = theme.color.sub, op = RANKING_OPS })
    heading(s, "info_records_ex_score", LAYOUT.ex, LISTING_OPS)
    heading(s, "info_records_miss", LAYOUT.miss, LISTING_OPS)
    heading(s, "info_records_ex_score", LAYOUT.ranking_ex, RANKING_OPS)

    for row = 0, AREA.rows - 1 do
        record_row(s, main_state, row)
        ranking_row(s, main_state, row)
    end
end

return records
