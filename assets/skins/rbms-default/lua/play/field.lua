local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local layout = require("lua.play.layout")

local field = {}

local TRANSPARENT = 0
local SIDE_TIMER_STRIDE = 10

local KEYS = { release = 120, lit_alpha = 230 }
local BEAM = { height = 280, alpha = 200, release = 120 }
local BOMB = { side = 128, cycle = 250 }
local LINE = { open = 400, pulse_height = 16, pulse_alpha = 120, pulse_fade = 500, beat = 1000 }
local SECTION_LINE = 2

local NOTE_IMAGES = {
    { list = "note", sprite = "note_" },
    { list = "lnstart", sprite = "note_" },
    { list = "lnend", sprite = "note_" },
    { list = "lnbody", sprite = "held_" },
    { list = "lnactive", sprite = "long_" },
    { list = "hcnstart", sprite = "note_" },
    { list = "hcnend", sprite = "note_" },
    { list = "hcnbody", sprite = "hell_held_" },
    { list = "hcnactive", sprite = "hell_" },
    { list = "hcndamage", sprite = "hell_held_" },
}

local JUDGE = {
    top = 560,
    detail_top = 520,
    size = { wide = 64, narrow = 48 },
    wide_from = 480,
    gap_share = 0.25,
    digits = 5,
    pop = 0.15,
    pop_time = 60,
    hold = 440,
    gone = 500,
    with_count = 3,
    slots = {
        { word = "judge_pgreat", count = "combo_pgreat", cycle = theme.judge_cycle.flash },
        { word = "judge_great", count = "combo_great", cycle = theme.judge_cycle.blink },
        { word = "judge_good", count = "combo_good", cycle = theme.judge_cycle.blink },
        { word = "judge_bad", count = "combo_good", cycle = theme.judge_cycle.blink },
        { word = "judge_poor", count = "combo_good", cycle = theme.judge_cycle.blink },
        { word = "judge_poor", count = "combo_good", cycle = theme.judge_cycle.blink },
    },
}

local DETAIL = { size = 24, sprite = "snum24", digits = 4, fast = theme.timing.fast, slow = theme.timing.slow, even = theme.color.sub }

local COVER = {
    edge = 2,
    inset = 12,
    value_rise = 12,
    value_sprite = "num18",
    value_digits = 4,
    green = theme.tone.green,
    green_digits = 4,
    lift_rise = 8,
    hidden_drop = 12,
}

local NOTICE = {
    top = 400,
    loading = { size = theme.type.label, bar_top = 444, bar_inset = 48, bar_height = 4 },
    ready = { size = theme.type.heading, fade = 100, hold = 500, gone = 600 },
    combo = { size = theme.type.heading, slide = 300, hold = 1700, gone = 2000, rule_drop = 56, rule_height = 2 },
}

local function lane_timer(base, lane)
    return base + lane.slot + lane.side * SIDE_TIMER_STRIDE
end

local function lane_box(lane, top, height)
    return { x = lane.x, top = top, w = lane.w, h = height }
end

local function lanes_under(s, view)
    local grid = layout.FIELD
    for _, side in ipairs(view.fields) do
        s:rect({ x = side.x, top = side.top, w = side.w, h = side.h + grid.line + grid.keys_height }, theme.color.line)
        for _, lane in ipairs(side.shown) do
            s:rect(lane_box(lane, side.top, side.h), lane.shaded and theme.color.bg or theme.color.panel)
            s:rect(lane_box(lane, grid.keys_top, grid.keys_height), theme.color.raised)
        end
    end
end

local function key_lights(s, view)
    local grid = layout.FIELD
    local lifted = props.offset.LIFT
    for _, lane in ipairs(view.lanes) do
        local on = lane_timer(props.timer.KEYON_1P, lane)
        local off = lane_timer(props.timer.KEYOFF_1P, lane)
        local key = lane_box(lane, grid.keys_top, grid.keys_height)
        s:rect(key, lane.tint, { timer = on, a = KEYS.lit_alpha })
        s:rect(key, lane.tint, { timer = off, keys = { { time = 0, a = KEYS.lit_alpha }, { time = KEYS.release, a = TRANSPARENT } }, loop = props.loop.ONCE })
        local beam = lane_box(lane, grid.top + grid.height - BEAM.height, BEAM.height)
        s:image("beam", beam, { color = lane.tint, a = BEAM.alpha, timer = on, offset = lifted })
        local fading = { { time = 0, a = BEAM.alpha }, { time = BEAM.release, a = TRANSPARENT } }
        s:image("beam", beam, { color = lane.tint, timer = off, offset = lifted, keys = fading, loop = props.loop.ONCE })
    end
end

local function note_field(s, view)
    local grid = layout.FIELD
    local line_id = s:define_image("block")
    local def = { dst = {}, size = {}, group = {}, hcnreactive = {}, mine = {} }
    for _, entry in ipairs(NOTE_IMAGES) do
        def[entry.list] = {}
    end
    local drain = s:define_image("hell_drain")
    local mine = s:define_image("mine")
    for index, lane in ipairs(view.lanes) do
        for _, entry in ipairs(NOTE_IMAGES) do
            def[entry.list][index] = s:define_image(entry.sprite .. lane.color)
        end
        def.hcnreactive[index] = drain
        def.mine[index] = mine
        def.dst[index] = { x = lane.x, y = theme.height - grid.top - grid.height, w = lane.w, h = grid.height }
        def.size[index] = lane.note_height
    end
    for index, side in ipairs(view.fields) do
        local rule = { x = side.x, top = grid.top + grid.height - SECTION_LINE, w = side.w, h = SECTION_LINE }
        def.group[index] = s:dst(line_id, rule, { color = theme.color.line, offset = props.offset.LIFT })
    end
    if not view.mode.scratch then
        def.dst2 = theme.height - grid.keys_top - grid.keys_height
    end
    s:single("note", def, nil, { offset = props.offset.NOTES_1P })
end

local function judge_line(s, view)
    local grid = layout.FIELD
    local lifted = props.offset.LIFT
    for _, side in ipairs(view.fields) do
        local line = { x = side.x, top = grid.top + grid.height, w = side.w, h = grid.line }
        s:rect(line, theme.color.accent_dim, { offset = lifted })
        local opening = { { time = 0, x = side.x + side.w / 2, w = 0 }, { time = LINE.open } }
        s:rect(line, theme.color.accent, { timer = props.timer.READY, keys = opening, acc = props.acc.DECELERATE, offset = lifted })
        local pulse = { x = side.x, top = line.top - LINE.pulse_height, w = side.w, h = LINE.pulse_height }
        local beating = { { time = 0, a = LINE.pulse_alpha }, { time = LINE.pulse_fade, a = TRANSPARENT }, { time = LINE.beat, a = TRANSPARENT } }
        s:image("beam", pulse, { color = theme.color.accent, timer = props.timer.RHYTHM, keys = beating, loop = 0, offset = lifted })
    end
end

local function cover_number(s, spec, box, opts)
    s:number(ui.merge({ sprite = COVER.value_sprite, digit = COVER.value_digits }, spec), box, opts)
end

local function covers(s, view)
    local grid = layout.FIELD
    local block = s:sprite_source("block")
    local foot = theme.height - grid.keys_top
    local line = theme.height - grid.top - grid.height
    local lane_cover = s:object("slider", ui.merge(block, { type = props.rate.LANECOVER, angle = props.slider_angle.DOWN, range = grid.height, changeable = false }))
    local hidden = s:object("hiddenCover", ui.merge(block, { disapearLine = line, isDisapearLineLinkLift = true }))
    local lift = s:object("liftCover", ui.merge(block, { disapearLine = foot, isDisapearLineLinkLift = false }))
    for _, side in ipairs(view.fields) do
        local below = { x = side.x, top = grid.top + grid.height, w = side.w, h = grid.height }
        s:place(hidden, below, { color = theme.color.raised })
        s:place(lift, below, { color = theme.color.raised })
        local above = { x = side.x, top = grid.top - grid.height, w = side.w, h = grid.height }
        s:place(lane_cover, above, { color = theme.color.raised })

        local covered = { op = { props.option.LANECOVER1_ON }, offset = props.offset.LANECOVER }
        s:rect({ x = side.x, top = grid.top - COVER.edge, w = side.w, h = COVER.edge }, theme.color.accent, covered)
        local rise = grid.top - COVER.edge - COVER.value_rise
        local inner = { x = side.x + COVER.inset, baseline = rise, w = side.w - COVER.inset * 2 }
        cover_number(s, { ref = props.number.LANECOVER1 }, inner, ui.merge(covered, { color = theme.color.sub }))
        local green = { ref = props.number.DURATION_GREEN, digit = COVER.green_digits, saturate = true, align = "right" }
        cover_number(s, green, inner, ui.merge(covered, { color = COVER.green }))

        local lifted = { x = side.x + COVER.inset, baseline = grid.keys_top - COVER.lift_rise }
        cover_number(s, { ref = props.number.LIFT1 }, lifted, { op = { props.option.LIFT1_ON }, color = theme.color.sub })
        local hid = { x = side.x + COVER.inset, top = grid.top + grid.height + COVER.hidden_drop }
        cover_number(s, { ref = props.number.HIDDEN1 }, hid, { offsets = { props.offset.LIFT, props.offset.HIDDEN_COVER }, color = theme.color.sub })
    end
end

local function bombs(s, view)
    local grid = layout.FIELD
    local middle = grid.top + grid.height + grid.line / 2
    for _, lane in ipairs(view.lanes) do
        local box = { x = lane.x + (lane.w - BOMB.side) / 2, top = middle - BOMB.side / 2, w = BOMB.side, h = BOMB.side }
        local hit = lane_timer(props.timer.BOMB_1P, lane)
        local hold = lane_timer(props.timer.HOLD_1P, lane)
        local shared = { color = lane.tint, blend = props.blend.ADD, offset = props.offset.LIFT }
        local once = { { time = 0 }, { time = BOMB.cycle - 1 } }
        s:image("bomb", box, ui.merge(shared, { timer = hit, keys = once, loop = props.loop.ONCE, def = { id = "img.bomb." .. hit, timer = hit, cycle = BOMB.cycle } }))
        s:image("bomb", box, ui.merge(shared, { timer = hold, def = { id = "img.bomb." .. hold, timer = hold, cycle = BOMB.cycle } }))
    end
end

local function popping(box)
    local grow_w, grow_h = box.w * JUDGE.pop, box.h * JUDGE.pop
    return {
        { time = 0, dx = -grow_w / 2, dtop = -grow_h / 2, dw = grow_w, dh = grow_h },
        { time = JUDGE.pop_time },
        { time = JUDGE.hold },
        { time = JUDGE.gone, a = TRANSPARENT },
    }
end

local function beside_popping(word)
    return {
        { time = 0, dx = word.w * JUDGE.pop, dtop = -word.h * JUDGE.pop / 2 },
        { time = JUDGE.pop_time },
        { time = JUDGE.hold },
        { time = JUDGE.gone, a = TRANSPARENT },
    }
end

local function judge_of(s, side)
    local size = side.w >= JUDGE.wide_from and JUDGE.size.wide or JUDGE.size.narrow
    local timer = props.timer.JUDGE_1P + side.side
    local offsets = { props.offset.LIFT, props.offset.JUDGE_1P }
    local gap = size * JUDGE.gap_share
    local middle = side.x + side.w / 2
    local def = { index = side.side, shift = true, images = {}, numbers = {} }
    local counts = {}
    for slot, entry in ipairs(JUDGE.slots) do
        local sprite = s:sprite(entry.word)
        local strip = s:sprite(entry.count)
        local scale = size / sprite.size
        local width, height = sprite.cell_w * scale, sprite.cell_h * scale
        local left = slot <= JUDGE.with_count and middle - (width + gap) / 2 or middle - width / 2
        local word = { x = left, top = JUDGE.top - sprite.pad * scale, w = width, h = height }
        local shared = { timer = timer, loop = props.loop.ONCE, offsets = offsets, filter = props.filter.LINEAR }
        local word_id = s:define_image(entry.word, { cycle = entry.cycle })
        def.images[slot] = s:dst(word_id, word, ui.merge(shared, { keys = popping(word) }))
        local count = { id = "num." .. entry.count .. "." .. side.side, sprite = entry.count, ref = props.number.COMBO, cycle = entry.cycle }
        counts[entry.count] = counts[entry.count] or s:define_number(count, JUDGE.digits)
        local cell = { x = width + gap, top = height - strip.cell_h * scale, w = strip.cell_w * scale, h = strip.cell_h * scale }
        def.numbers[slot] = s:dst(counts[entry.count], cell, ui.merge(shared, { keys = beside_popping(word) }), word)
    end
    local id = s:object("judge", def)
    s:place(id, nil, {})
end

local function detail_of(s, view, side)
    local option = layout.OPTION
    local first = side.side == 0
    local timer = props.timer.JUDGE_1P + side.side
    local perfect = first and props.option.JUDGE_1P_PERFECT or props.option.JUDGE_2P_PERFECT
    local early = first and props.option.JUDGE_1P_EARLY or props.option.JUDGE_2P_EARLY
    local late = first and props.option.JUDGE_1P_LATE or props.option.JUDGE_2P_LATE
    local offsets = { props.offset.LIFT, props.offset.JUDGEDETAIL_1P }
    local shown = { { time = 0 }, { time = JUDGE.gone } }
    local box = { x = side.x, top = JUDGE.detail_top, w = side.w }
    local placed = function(op, color)
        return { op = op, color = color, timer = timer, keys = shown, loop = props.loop.ONCE, offsets = offsets }
    end
    if view.detail == option.DETAIL_FAST_SLOW then
        s:text({ text = "FAST", size = DETAIL.size, align = "center" }, box, placed({ early }, DETAIL.fast))
        s:text({ text = "SLOW", size = DETAIL.size, align = "center" }, box, placed({ late }, DETAIL.slow))
    elseif view.detail == option.DETAIL_MILLISECONDS then
        local ref = props.number.JUDGE_1P_DURATION + side.side
        local spec = { sprite = DETAIL.sprite, ref = ref, digit = DETAIL.digits, saturate = true, align = "center" }
        s:number(spec, box, placed({ perfect }, DETAIL.even))
        s:number(spec, box, placed({ early }, DETAIL.fast))
        s:number(spec, box, placed({ late }, DETAIL.slow))
    end
end

local function judges(s, view)
    for _, side in ipairs(view.fields) do
        judge_of(s, side)
        detail_of(s, view, side)
    end
end

local function notices(s, view)
    local bar = s:sprite_source("bar_h")
    local loading_bar = s:object("graph", ui.merge(bar, { type = props.rate.LOAD_PROGRESS, angle = props.graph_angle.RIGHT }))
    for _, side in ipairs(view.fields) do
        local row = { x = side.x, top = NOTICE.top, w = side.w }
        local loading = { op = { props.option.NOW_LOADING } }
        s:text({ text = "LOADING", size = NOTICE.loading.size, align = "center" }, row, ui.merge(loading, { color = theme.color.sub }))
        local inset = NOTICE.loading.bar_inset
        local track = { x = side.x + inset, top = NOTICE.loading.bar_top, w = side.w - inset * 2, h = NOTICE.loading.bar_height }
        s:rect(track, theme.color.raised, loading)
        s:place(loading_bar, track, ui.merge(loading, { color = theme.color.accent }))

        local ready = NOTICE.ready
        local steps = { { time = 0, a = TRANSPARENT }, { time = ready.fade }, { time = ready.hold }, { time = ready.gone, a = TRANSPARENT } }
        local readying = { timer = props.timer.READY, keys = steps, loop = props.loop.ONCE, color = theme.color.text }
        s:text({ text = "READY", size = ready.size, align = "center" }, row, readying)

        local combo = NOTICE.combo
        local timer = props.timer.FULLCOMBO_1P
        local rising = {
            { time = 0, dtop = theme.time.slide_distance, a = TRANSPARENT },
            { time = combo.slide },
            { time = combo.hold },
            { time = combo.gone, a = TRANSPARENT },
        }
        local cheering = { timer = timer, keys = rising, loop = props.loop.ONCE, acc = props.acc.DECELERATE, color = theme.color.accent }
        s:text({ text = "FULL COMBO", size = combo.size, align = "center", overflow = props.text_overflow.SHRINK }, row, cheering)
        local rule = { x = side.x, top = NOTICE.top + combo.rule_drop, w = side.w, h = combo.rule_height }
        local sweeping = {
            { time = 0, x = side.x + side.w / 2, w = 0 },
            { time = combo.slide },
            { time = combo.hold },
            { time = combo.gone, a = TRANSPARENT },
        }
        s:rect(rule, theme.color.accent, { timer = timer, keys = sweeping, loop = props.loop.ONCE, acc = props.acc.DECELERATE })
    end
end

function field.under(s, view)
    lanes_under(s, view)
    key_lights(s, view)
    note_field(s, view)
end

function field.over(s, view)
    covers(s, view)
    judge_line(s, view)
    bombs(s, view)
    judges(s, view)
    notices(s, view)
end

return field
