local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local layout = require("lua.play.layout")
local colors = require("lua.play.colors")

local field = {}

local TRANSPARENT = 0
local SIDE_TIMER_STRIDE = 10

local BEAM = { height = 280, release = 120 }
local BOMB = { cycle = 250 }
local LINE = { open = 400, pulse_height = 16, pulse_fade = 500, beat = 1000 }
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
    top = 624,
    size = { largest = 64, smallest = 16, step = 2 },
    gap_share = 0.25,
    digits = 5,
    centred_digits = 3,
    shown = 500,
    slots = {
        { word = "judge_pgreat", count = "combo_pgreat" },
        { word = "judge_great", count = "combo_great" },
        { word = "judge_good", count = "combo_good" },
        { word = "judge_bad", count = "combo_good" },
        { word = "judge_poor", count = "combo_good" },
        { word = "judge_poor", count = "combo_good" },
    },
}

local ABOVE = {
    top = 584,
    size = 24,
    sprite = "snum24",
    gap = 16,
    word_width = 72,
    difference_digits = 5,
    difference = theme.color.text,
    timing_digits = 4,
    fast = theme.timing.fast,
    slow = theme.timing.slow,
    even = theme.color.sub,
    refs = {
        [layout.OPTION.DIFFERENCE_TARGET] = props.number.DIFF_EXSCORE,
        [layout.OPTION.DIFFERENCE_BEST] = props.number.DIFF_HIGHSCORE,
    },
}

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
    top = 464,
    loading = { size = theme.type.label, bar_top = 508, bar_inset = 48, bar_height = 4 },
    ready = { size = theme.type.heading, fade = 100, hold = 500, gone = 600 },
    combo = { size = theme.type.heading, slide = 300, hold = 1700, gone = 2000, rule_drop = 56, rule_height = 2 },
}

local function round(value)
    return math.floor(value + 0.5)
end

local function lane_timer(base, lane)
    return base + lane.slot + lane.side * SIDE_TIMER_STRIDE
end

local function lane_box(lane, top, height)
    return { x = lane.x, top = top, w = lane.w, h = height }
end

local function lane_faces(s, view)
    for _, side in ipairs(view.fields) do
        for _, lane in ipairs(side.shown) do
            local face = colors.base(lane.shaded and theme.color.bg or theme.color.panel, lane.face_tint)
            colors.tinted(s:rect(lane_box(lane, side.top, side.h), face), lane.face_tint)
        end
    end
end

local function seams(side, top, height, each)
    for index = 1, #side.shown - 1 do
        local lane = side.shown[index]
        local seam = lane.x + lane.w
        each({ x = seam, top = top, w = side.shown[index + 1].x - seam, h = height })
    end
end

local function lane_dividers(s, view)
    local tint = view.tint.LANE_DIVIDER
    for _, side in ipairs(view.fields) do
        seams(side, side.top, side.h, function(gap)
            colors.tinted(s:rect(gap, colors.base(theme.color.line, tint)), tint)
        end)
    end
end

local function key_beams(s, view)
    local grid = layout.FIELD
    local lifted = props.offset.LIFT
    for _, lane in ipairs(view.lanes) do
        local beam = lane_box(lane, grid.top + grid.height - BEAM.height, BEAM.height)
        local color, tint = colors.base(lane.tint, lane.beam_tint), lane.beam_tint
        colors.tinted(s:image("beam", beam, { color = color, timer = lane_timer(props.timer.KEYON_1P, lane), offset = lifted }), tint)
        local fading = { { time = 0 }, { time = BEAM.release, a = TRANSPARENT } }
        local released = { color = color, timer = lane_timer(props.timer.KEYOFF_1P, lane), offset = lifted, keys = fading, loop = props.loop.ONCE }
        colors.tinted(s:image("beam", beam, released), tint)
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
    local tinted = false
    for index, lane in ipairs(view.lanes) do
        for _, entry in ipairs(NOTE_IMAGES) do
            def[entry.list][index] = s:define_image(entry.sprite .. (lane.note_tint and colors.PURE_NOTES or lane.color))
        end
        tinted = tinted or lane.note_tint ~= nil
        def.hcnreactive[index] = drain
        def.mine[index] = mine
        def.dst[index] = { x = lane.x, y = theme.height - grid.top - grid.height, w = lane.w, h = grid.height }
        def.size[index] = lane.note_height
    end
    if tinted then
        def.tint = {}
        for index, lane in ipairs(view.lanes) do
            def.tint[index] = lane.note_tint or colors.NO_TINT
        end
    end
    local bar = view.tint.BAR_LINE
    s:within(layout.PART.BAR_LINE, function()
        for index, side in ipairs(view.fields) do
            local rule = { x = side.x, top = grid.top + grid.height - SECTION_LINE, w = side.w, h = SECTION_LINE }
            def.group[index] = colors.tinted(s:dst(line_id, rule, { color = colors.base(theme.color.line, bar), offset = props.offset.LIFT }), bar)
        end
    end)
    s:within(layout.PART.NOTE, function()
        s:single("note", def, nil, { offset = props.offset.NOTES_1P })
    end)
end

local function judge_line(s, view)
    local grid = layout.FIELD
    local lifted = props.offset.LIFT
    local tint = view.tint.JUDGE_LINE
    local dim = tint == nil and theme.color.accent_dim or theme.accent_dim.white
    local bright = colors.base(theme.color.accent, tint)
    for _, side in ipairs(view.fields) do
        local line = { x = side.x, top = grid.top + grid.height, w = side.w, h = grid.line }
        colors.tinted(s:rect(line, dim, { offset = lifted }), tint)
        local opening = { { time = 0, x = side.x + side.w / 2, w = 0 }, { time = LINE.open } }
        colors.tinted(s:rect(line, bright, { timer = props.timer.READY, keys = opening, acc = props.acc.DECELERATE, offset = lifted }), tint)
        local pulse = { x = side.x, top = line.top - LINE.pulse_height, w = side.w, h = LINE.pulse_height }
        local beating = { { time = 0 }, { time = LINE.pulse_fade, a = TRANSPARENT }, { time = LINE.beat, a = TRANSPARENT } }
        colors.tinted(s:image("pulse", pulse, { color = bright, timer = props.timer.RHYTHM, keys = beating, loop = 0, offset = lifted }), tint)
    end
end

local function cover_number(s, spec, box, opts)
    s:number(ui.merge({ sprite = COVER.value_sprite, digit = COVER.value_digits }, spec), box, opts)
end

local function covers(s, view)
    local grid = layout.FIELD
    local block = s:sprite_source("block")
    local foot = theme.height - grid.bottom
    local line = theme.height - grid.top - grid.height
    local lane_cover = s:object("slider", ui.merge(block, { type = props.rate.LANECOVER, angle = props.slider_angle.DOWN, range = grid.height, changeable = false }))
    local hidden = s:object("hiddenCover", ui.merge(block, { disapearLine = line, isDisapearLineLinkLift = true }))
    local lift = s:object("liftCover", ui.merge(block, { disapearLine = foot, isDisapearLineLinkLift = false }))
    local tint = view.tint.LANE_COVER
    local face = { color = colors.base(theme.color.raised, tint) }
    for _, side in ipairs(view.fields) do
        local below = { x = side.x, top = grid.top + grid.height, w = side.w, h = grid.height }
        colors.tinted(s:place(hidden, below, face), tint)
        colors.tinted(s:place(lift, below, face), tint)
        local above = { x = side.x, top = grid.top - grid.height, w = side.w, h = grid.height }
        colors.tinted(s:place(lane_cover, above, face), tint)

        local covered = { op = { props.option.LANECOVER1_ON }, offset = props.offset.LANECOVER }
        s:rect({ x = side.x, top = grid.top - COVER.edge, w = side.w, h = COVER.edge }, theme.color.accent, covered)
        local rise = grid.top - COVER.edge - COVER.value_rise
        local inner = { x = side.x + COVER.inset, baseline = rise, w = side.w - COVER.inset * 2 }
        cover_number(s, { ref = props.number.LANECOVER1 }, inner, ui.merge(covered, { color = theme.color.sub }))
        local green = { ref = props.number.DURATION_GREEN, digit = COVER.green_digits, saturate = true, align = "right" }
        cover_number(s, green, inner, ui.merge(covered, { color = COVER.green }))

        local lifted = { x = side.x + COVER.inset, baseline = grid.bottom - COVER.lift_rise }
        cover_number(s, { ref = props.number.LIFT1 }, lifted, { op = { props.option.LIFT1_ON }, color = theme.color.sub })
        local hid = { x = side.x + COVER.inset, top = grid.top + grid.height + COVER.hidden_drop }
        cover_number(s, { ref = props.number.HIDDEN1 }, hid, { offsets = { props.offset.LIFT, props.offset.HIDDEN_COVER }, color = theme.color.sub })
    end
end

local function bombs(s, view)
    local grid = layout.FIELD
    local ring = s:sprite("bomb")
    local tint = view.tint.BOMB
    for _, lane in ipairs(view.lanes) do
        local box = { x = lane.x + (lane.w - ring.cell_w) / 2, top = grid.top + grid.height + (grid.line - ring.cell_h) / 2 }
        local hit = lane_timer(props.timer.BOMB_1P, lane)
        local hold = lane_timer(props.timer.HOLD_1P, lane)
        local shared = { color = colors.base(view.bomb_tint or lane.tint, tint), blend = props.blend.ADD, offset = props.offset.LIFT }
        local once = { { time = 0 }, { time = BOMB.cycle - 1 } }
        local burst = { timer = hit, keys = once, loop = props.loop.ONCE, def = { id = "img.bomb." .. hit, timer = hit, cycle = BOMB.cycle } }
        colors.tinted(s:image("bomb", box, ui.merge(shared, burst)), tint)
        colors.tinted(s:image("bomb", box, ui.merge(shared, { timer = hold, def = { id = "img.bomb." .. hold, timer = hold, cycle = BOMB.cycle } })), tint)
    end
end

local function judge_row_at(s, side, size)
    local widest = s:sprite(JUDGE.slots[1].word)
    local digit = s:sprite(JUDGE.slots[1].count)
    local scale = size / widest.size
    local place = { w = round(digit.cell_w * scale), h = round(digit.cell_h * scale) }
    local gap = round(size * JUDGE.gap_share)
    local count_x = round(widest.cell_w * scale) + gap
    local left = round(side.x + (side.w - count_x - place.w * JUDGE.centred_digits) / 2)
    return { size = size, scale = scale, place = place, gap = gap, left = left, right = left + count_x + place.w * JUDGE.digits }
end

local function judge_row(s, side)
    local row = judge_row_at(s, side, JUDGE.size.largest)
    while row.right > side.x + side.w and row.size > JUDGE.size.smallest do
        row = judge_row_at(s, side, row.size - JUDGE.size.step)
    end
    return row
end

local function judge_of(s, side, row)
    local timer = props.timer.JUDGE_1P + side.side
    local offsets = { props.offset.LIFT, layout.JUDGE_POSITION.id, props.offset.JUDGE_1P }
    local shown = { timer = timer, keys = { { time = 0 }, { time = JUDGE.shown } }, loop = props.loop.ONCE, offsets = offsets, filter = props.filter.LINEAR }
    local place = row.place
    local def = { index = side.side, shift = false, numberAlign = props.number_align.LEFT, images = {}, numbers = {} }
    local counts = {}
    for slot, entry in ipairs(JUDGE.slots) do
        local sprite = s:sprite(entry.word)
        local width, height = round(sprite.cell_w * row.scale), round(sprite.cell_h * row.scale)
        local word = { x = row.left, top = JUDGE.top - sprite.pad * row.scale, w = width, h = height }
        def.images[slot] = s:dst(s:define_image(entry.word), word, shown)
        local count = { id = "num." .. entry.count .. "." .. side.side, sprite = entry.count, ref = props.number.COMBO }
        counts[entry.count] = counts[entry.count] or s:define_number(count, JUDGE.digits)
        local cell = { x = width + row.gap + math.floor(place.w * JUDGE.digits / 2), top = height - place.h, w = place.w, h = place.h }
        def.numbers[slot] = s:dst(counts[entry.count], cell, shown, word)
    end
    local id = s:object("judge", def)
    s:place(id, nil, {})
end

local function above_of(s, view, side, row)
    local option = layout.OPTION
    local first = side.side == 0
    local perfect = first and props.option.JUDGE_1P_PERFECT or props.option.JUDGE_2P_PERFECT
    local early = first and props.option.JUDGE_1P_EARLY or props.option.JUDGE_2P_EARLY
    local late = first and props.option.JUDGE_1P_LATE or props.option.JUDGE_2P_LATE
    local placed = function(op, color)
        return {
            op = op,
            color = color,
            timer = props.timer.JUDGE_1P + side.side,
            keys = { { time = 0 }, { time = JUDGE.shown } },
            loop = props.loop.ONCE,
            offsets = { props.offset.LIFT, layout.JUDGE_POSITION.id, props.offset.JUDGEDETAIL_1P },
        }
    end
    local ref = ABOVE.refs[view.difference]
    local slot = { x = row.left, top = ABOVE.top, w = ABOVE.word_width }
    if ref ~= nil then
        local difference = { sprite = ABOVE.sprite, ref = ref, digit = ABOVE.difference_digits, zeropadding = props.padding.ZEROS, saturate = true }
        slot.x = slot.x + s:number(difference, slot, placed(nil, ABOVE.difference)).span + ABOVE.gap
    end
    if view.detail == option.DETAIL_FAST_SLOW then
        s:text({ text = "FAST", size = ABOVE.size }, slot, placed({ early }, ABOVE.fast))
        s:text({ text = "SLOW", size = ABOVE.size }, slot, placed({ late }, ABOVE.slow))
    elseif view.detail == option.DETAIL_MILLISECONDS then
        local timing = { sprite = ABOVE.sprite, ref = props.number.JUDGE_1P_DURATION + side.side, digit = ABOVE.timing_digits, saturate = true }
        s:number(timing, slot, placed({ perfect }, ABOVE.even))
        s:number(timing, slot, placed({ early }, ABOVE.fast))
        s:number(timing, slot, placed({ late }, ABOVE.slow))
    end
end

local function judges(s, view)
    if not view.judge_shown then
        return
    end
    for _, side in ipairs(view.fields) do
        local row = judge_row(s, side)
        judge_of(s, side, row)
        above_of(s, view, side, row)
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

local function part(s, id, build, view)
    s:within(id, function()
        build(s, view)
    end)
end

function field.under(s, view)
    part(s, layout.PART.LANE_BACKGROUND, lane_faces, view)
    part(s, layout.PART.LANE_DIVIDER, lane_dividers, view)
    part(s, layout.PART.KEY_BEAM, key_beams, view)
    note_field(s, view)
end

function field.over(s, view)
    part(s, layout.PART.LANE_COVER, covers, view)
    part(s, layout.PART.JUDGE_LINE, judge_line, view)
    part(s, layout.PART.BOMB, bombs, view)
    judges(s, view)
    notices(s, view)
end

return field
