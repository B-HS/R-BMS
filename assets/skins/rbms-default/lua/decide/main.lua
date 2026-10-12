local theme = require("lua.theme")
local props = require("lua.props")
local ui = require("lua.ui")
local motion = require("lua.motion")
local parts = require("lua.parts")

local decide = {}

local TIME = { input = 300, scene = 3200, fadeout = 800 }
local OPTION = { STAGE_SHOW = parts.stage_image.show, GRAPH_SHOW = 902, GRAPH_HIDE = 903 }

local HERO = { top = 256, height = 320, bar_width = 8, text_gap = 32, grow = 350, first_plain_line = 2 }
local HERO_LINES = {
    { ref = props.text.TABLE_FULL, top = 256, size = theme.type.label, color = theme.color.accent },
    { ref = props.text.GENRE, top = 296, size = theme.type.label, color = theme.color.sub },
    { ref = props.text.TITLE, top = 344, size = theme.type.display, color = theme.color.text },
    { ref = props.text.SUBTITLE, top = 464, size = theme.type.heading, color = theme.color.sub },
    { ref = props.text.FULLARTIST, top = 536, size = theme.type.body, color = theme.color.text },
}
local HERO_FIRST_AT = 100

local STAGE = { column = 8, span = 4, top = 200, height = 432, panel_at = 150, image_at = 250, image_fade = 400 }

local FACTS = {
    rule_top = 672,
    label_top = 696,
    baseline = 836,
    first_at = 350,
    level = { column = 0, span = 3, sprite = "num144", number_share = 0.5, name_gap = 24 },
    bpm = { column = 3, span = 3, sprite = "num64", dash_gap = 12, dash_width = 24, dash_height = 4 },
    notes = { column = 6, span = 2, sprite = "num64" },
    judge = { column = 8, span = 2 },
    keys = { column = 10, span = 2 },
}

local GRAPH = { caption_top = 872, top = 900, height = 84, at = 500 }

local FOOTER = { hint_top = 1022, hint_fade = 200 }
local CLOSING = { sweep = 300, cover_at = 200, cover = 500 }

function decide.header()
    return {
        type = props.skin_type.DECIDE,
        name = theme.name .. " (Decide)",
        author = theme.author,
        w = theme.width,
        h = theme.height,
        input = TIME.input,
        scene = TIME.scene,
        fadeout = TIME.fadeout,
        property = theme.properties({
            parts.stage_image_property(),
            parts.shown_or_hidden("Note Graph", OPTION.GRAPH_SHOW, OPTION.GRAPH_HIDE),
        }),
    }
end

local function hero(s, main_state)
    local grid = theme.grid
    local listed = main_state.text(props.text.TABLE_FULL) ~= ""
    local top = listed and HERO.top or HERO_LINES[HERO.first_plain_line].top
    local bar = { x = grid.margin, top = top, w = HERO.bar_width, h = HERO.top + HERO.height - top }
    parts.difficulty_rect(s, bar, motion.grow_down(0, HERO.grow))

    local stage_x = theme.column(STAGE.column, STAGE.span)
    local text_x = grid.margin + HERO.bar_width + HERO.text_gap
    local staged = parts.stage_image_shown()
    local text_end = staged and stage_x - grid.gutter * 2 or grid.margin + theme.content_width()
    local text_width = text_end - text_x
    for index, line in ipairs(HERO_LINES) do
        local start = motion.staggered(index - 1, HERO_FIRST_AT)
        local opts = motion.slide_in(start, theme.time.slide, -theme.time.slide_distance, 0, { color = line.color })
        s:text({ ref = line.ref, size = line.size, overflow = props.text_overflow.SHRINK }, { x = text_x, top = line.top, w = text_width }, opts)
    end
end

local function stage(s)
    local x, width = theme.column(STAGE.column, STAGE.span)
    local box = { x = x, top = STAGE.top, w = width, h = STAGE.height }
    local shown = { OPTION.STAGE_SHOW }
    parts.panel(s, box, motion.fade_in(STAGE.panel_at, theme.time.fade, { op = shown }))
    local image = motion.fade_in(STAGE.image_at, STAGE.image_fade, { op = { OPTION.STAGE_SHOW, props.option.STAGEFILE }, stretch = props.stretch.FIT_INNER })
    s:reference(props.image.STAGEFILE, box, image)
    local size = theme.type.caption
    local label = { x = x, top = theme.centered_top(box, size), w = width }
    local missing = motion.fade_in(STAGE.image_at, STAGE.image_fade, { op = { OPTION.STAGE_SHOW, props.option.NO_STAGEFILE }, color = theme.color.muted })
    s:text({ text = parts.stage_image.missing, size = size, align = "center" }, label, missing)
end

local function fact_label(s, fact, label, order)
    local x, width = theme.column(fact.column, fact.span)
    local start = motion.staggered(order, FACTS.first_at)
    parts.caption(s, label, { x = x, top = FACTS.label_top, w = width }, motion.fade_in(start))
    return x, width, start
end

local function level_fact(s, main_state)
    local fact = FACTS.level
    local x, width, start = fact_label(s, fact, "LEVEL", 0)
    local spec = { sprite = fact.sprite, ref = props.number.PLAYLEVEL, known = main_state.number(props.number.PLAYLEVEL), fit = width * fact.number_share }
    local box = { x = x, baseline = FACTS.baseline }
    parts.by_difficulty(function(_, op, color)
        s:number(spec, box, motion.fade_in(start, nil, { op = { op }, color = color }))
    end)
    local size = theme.type.body
    local name_x = x + s:number_box(spec, box).span + fact.name_gap
    local name = { x = name_x, top = theme.text_top(FACTS.baseline, size), w = x + width - name_x }
    parts.difficulty_text(s, { size = size, overflow = props.text_overflow.SHRINK }, name, motion.fade_in(start))
end

local function bpm_fact(s, main_state)
    local fact = FACTS.bpm
    local x, width, start = fact_label(s, fact, "BPM", 1)
    local box = { x = x, baseline = FACTS.baseline }
    local low = { sprite = fact.sprite, ref = props.number.MINBPM, known = main_state.number(props.number.MINBPM) }
    local high = { sprite = fact.sprite, ref = props.number.MAXBPM, known = main_state.number(props.number.MAXBPM) }
    local steady = motion.fade_in(start, nil, { op = { props.option.NO_BPMCHANGE }, color = theme.color.text })
    s:number(ui.merge(high, { fit = width }), box, steady)

    local joint = fact.dash_gap * 2 + fact.dash_width
    local natural = s:number_box(low, box).span + s:number_box(high, box).span
    local size = s:sprite(fact.sprite).size * math.min(1, (width - joint) / natural)
    local changing = { props.option.BPMCHANGE }
    local placed = s:number(ui.merge(low, { size = size }), box, motion.fade_in(start, nil, { op = changing, color = theme.color.sub }))
    local dash_x = x + placed.span + fact.dash_gap
    local dash = { x = dash_x, top = FACTS.baseline - (placed.rise + fact.dash_height) / 2, w = fact.dash_width, h = fact.dash_height }
    s:rect(dash, theme.color.muted, motion.fade_in(start, nil, { op = changing }))
    local after = { x = dash_x + joint - fact.dash_gap, baseline = FACTS.baseline }
    s:number(ui.merge(high, { size = size }), after, motion.fade_in(start, nil, { op = changing, color = theme.color.text }))
end

local function notes_fact(s, main_state)
    local fact = FACTS.notes
    local x, width, start = fact_label(s, fact, "NOTES", 2)
    local spec = { sprite = fact.sprite, ref = props.number.TOTALNOTES, known = main_state.number(props.number.TOTALNOTES), fit = width }
    s:number(spec, { x = x, baseline = FACTS.baseline }, motion.fade_in(start, nil, { color = theme.color.text }))
end

local function word_fact(s, fact, label, order, place)
    local x, width, start = fact_label(s, fact, label, order)
    local size = theme.type.heading
    local box = { x = x, top = theme.text_top(FACTS.baseline, size), w = width }
    place(s, { size = size, overflow = props.text_overflow.SHRINK }, box, motion.fade_in(start, nil, { color = theme.color.text }))
end

local function facts(s, main_state)
    parts.rule(s, FACTS.rule_top, motion.grow_right(FACTS.first_at - theme.time.fade, theme.time.fade))
    level_fact(s, main_state)
    bpm_fact(s, main_state)
    notes_fact(s, main_state)
    word_fact(s, FACTS.judge, "JUDGE", 3, parts.judge_rank_text)
    word_fact(s, FACTS.keys, "KEYS", 4, parts.key_mode_text)
end

local function graph(s)
    local grid = theme.grid
    local width = theme.content_width()
    local shown = { op = { OPTION.GRAPH_SHOW } }
    local appear = motion.fade_in(GRAPH.at, nil, shown)
    parts.caption(s, "NOTE DISTRIBUTION", { x = grid.margin, top = GRAPH.caption_top, w = width }, appear)
    local box = { x = grid.margin, top = GRAPH.top, w = width, h = GRAPH.height }
    parts.panel(s, box, appear)
    s:object("judgegraph", { type = props.judge_graph.NOTES, backTexOff = props.back_texture.OFF }, box, appear)
    s:object("bpmgraph", parts.bpm_graph(), box, appear)
end

local function footer(s)
    local grid = theme.grid
    local width = theme.content_width()
    local progress = { x = grid.margin, top = grid.footer_rule, w = width, h = grid.rule }
    s:rect(progress, theme.color.accent, motion.progress_right(TIME.scene))
    local hint = motion.fade_in(0, FOOTER.hint_fade, { timer = props.timer.STARTINPUT, color = theme.color.muted })
    s:text({ text = "ENTER  SKIP        ESC  BACK", size = theme.type.caption }, { x = grid.margin, top = FOOTER.hint_top, w = width }, hint)
end

local function closing(s)
    local grid = theme.grid
    local sweep = { x = grid.margin, top = FACTS.rule_top, w = theme.content_width(), h = grid.rule }
    s:rect(sweep, theme.color.accent, motion.grow_right(0, CLOSING.sweep, { timer = props.timer.FADEOUT }))
    parts.fade_to_black(s, CLOSING.cover_at, CLOSING.cover)
end

function decide.build()
    local header = decide.header()
    if skin_config == nil then
        return header
    end
    local main_state = require("main_state")
    theme.apply(skin_config.option)
    local s = ui.new(header)
    s:atlas("common")
    s:atlas("decide")
    parts.backdrop(s)
    parts.frame(s, "CHART SELECTED")
    hero(s, main_state)
    stage(s)
    facts(s, main_state)
    graph(s)
    footer(s)
    closing(s)
    return s:finish()
end

return decide
