# 부록: 기반 조사 원본

## 메커닉(원본 구조화)

### mechanics[0]

```json
{
  "positionFormula": "SCREEN Y POSITION OF A NOTE (the reference implementation, LaneRenderer.drawLane, <reference>/play/LaneRenderer.java:240-484)\n\n--- 0. Time normalization (LaneRenderer.java:252-258) ---\n  // `time` arg is the raw absolute clock (ms). Subtract the play-start timer, add user judge-timing offset.\n  time      = (timerOn(TIMER_PLAY) ? time - timer(TIMER_PLAY)\n              : timerOn(141)       ? time - timer(141) : 0) + config.getJudgetiming()   // ms\n  microtime = time * 1000                                                                // µs  (current play time)\n  // microtime is the player's current position; every note/timeline has tl.getMicroTime() in µs.\n\n--- 1. Constants for this frame ---\n  hispeed = playconfig.getHispeed()                         // float multiplier (1.0 = \"equal speed\")  (LaneRenderer.java:261)\n  nbpm    = BPM of the LAST timeline with microTime <= microtime   (the BPM currently under the judge line)  (LaneRenderer.java:266-270)\n  nscroll = SCROLL value of that same timeline                                                              (LaneRenderer.java:266-269)\n\n  // region = on-screen \"length\" (in pixel-equivalent ms units) of ONE 4/4 measure at the head BPM/scroll:\n  region  = nscroll > 0 ? (240000 / nbpm / hispeed) / nscroll : 0     // ms; 240000/nbpm = ms per measure at hispeed 1  (LaneRenderer.java:271)\n\n  // Lane geometry (lanes[0].region is the GDX rect of the play lane; y is bottom, +y is up):\n  hu  = lanes[0].region.y + lanes[0].region.height                                   // top of lane (far end)   (LaneRenderer.java:274)\n  hl  = enablelift ? lanes[0].region.y + lanes[0].region.height * lift               // judge line (note arrival)\n                   : lanes[0].region.y                                               // (LaneRenderer.java:275)\n  rxhs = (hu - hl) * hispeed     // PIXEL length of one full measure on screen at this hispeed  (LaneRenderer.java:276)\n  // NOTE: rxhs already includes hispeed; region (ms) is only used for the green-number/duration readout, NOT for y.\n\n--- 2. Accumulated Y by integrating timeline segments (LaneRenderer.java:277, 472-483) ---\n  // Walk timelines[] forward from `pos` (a cached cursor). Start at the judge line:\n  y = hl\n  for each timeline tl = timelines[i], i from pos upward, while y <= hu:\n     if (tl.getMicroTime() >= microtime):                 // this segment endpoint is in the future\n        if (i > 0):\n           prevtl = timelines[i-1]\n           dSection = tl.getSection() - prevtl.getSection()        // measures between the two timelines (double; 1.0 = one 4/4 measure)\n           if (prevtl.getMicroTime() + prevtl.getMicroStop() > microtime):\n                // current time is still inside prevtl's STOP region -> full segment height, frozen\n                y += dSection * prevtl.getScroll() * rxhs                                   // (LaneRenderer.java:475-476)\n           else:\n                // partial first segment: scale by the not-yet-elapsed fraction of this segment's real time\n                y += dSection * prevtl.getScroll()\n                     * (tl.getMicroTime() - microtime)\n                     / (tl.getMicroTime() - prevtl.getMicroTime() - prevtl.getMicroStop())\n                     * rxhs                                                                  // (LaneRenderer.java:477-479)\n        else:  // i == 0, first timeline\n           y += tl.getSection() * (tl.getMicroTime() - microtime) / tl.getMicroTime() * rxhs   // (LaneRenderer.java:481-482)\n     // the note(s) on timeline tl are then drawn at this accumulated y:\n     dsty = y + offsetY - offsetH/2     // final pixel Y of the note quad  (LaneRenderer.java:492)\n     // (note is only drawn if tl.getMicroTime() >= microtime, i.e. still above/at judge line)  (LaneRenderer.java:511,516,522,557)\n\n--- 3. Equivalent closed form (no BPM/SCROLL/STOP changes between now and the note) ---\n  // For a single uniform segment the partial-segment branch collapses to a clean linear law.\n  // Let Δt_us = note.getMicroTime() - microtime  (>=0, µs until the note reaches the judge line),\n  //     scroll = the scroll speed in effect, bpm = head bpm. Then:\n  //\n  //   y_note = hl  +  (Δt_us / (240000000 / bpm))  * scroll * hispeed * (hu - hl)\n  //          = hl  +  Δt_us * bpm * scroll * hispeed * (hu - hl) / 240000000\n  //\n  // i.e. note travels at  pixels_per_microsecond = bpm * scroll * hispeed * (hu-hl) / 240000000,\n  // reaching y = hl (judge line) exactly when Δt_us = 0, and is culled once y > hu.\n  // (Derivation matches LaneRenderer.java:481-482 with section = bpm*t/240000 measures; the PMS-miss-poor\n  //  fallback in LaneRenderer.java:600-614 spells out \"pixels/sec = rxhs2 * BPM / 240\" explicitly.)\n\nDEFINITIONS: hl = judge/arrival line (bottom, or lifted); hu = top of visible lane; rxhs = (hu-hl)*hispeed = pixels per measure; section = cumulative measure count (double); microtime = current play time (µs); tl.getMicroTime() = note/segment absolute time (µs); microStop = STOP freeze duration (µs); scroll = per-segment scroll-speed multiplier.",
  "variables": [
    {
      "name": "time",
      "meaning": "Current play time in ms after subtracting TIMER_PLAY start and adding config.getJudgetiming() offset",
      "sourceRef": "<reference>/play/LaneRenderer.java:252"
    },
    {
      "name": "microtime",
      "meaning": "Current play position in microseconds = time*1000; compared against each timeline's getMicroTime()",
      "sourceRef": "<reference>/play/LaneRenderer.java:258"
    },
    {
      "name": "hispeed",
      "meaning": "Hi-speed multiplier (PlayConfig.hispeed, 0.01..20, default 1.0). 1.0 = equal speed. Directly scales pixel travel.",
      "sourceRef": "<reference>/play/LaneRenderer.java:261"
    },
    {
      "name": "nbpm / nowbpm",
      "meaning": "BPM of the last timeline at or before current time (BPM under the judge line)",
      "sourceRef": "<reference>/play/LaneRenderer.java:266"
    },
    {
      "name": "nscroll",
      "meaning": "SCROLL value of the head timeline; divides region for the duration readout",
      "sourceRef": "<reference>/play/LaneRenderer.java:268"
    },
    {
      "name": "region",
      "meaning": "On-screen length of one 4/4 measure in ms-units = (240000/nbpm/hispeed)/nscroll; basis of the green number / currentduration",
      "sourceRef": "<reference>/play/LaneRenderer.java:271"
    },
    {
      "name": "hu",
      "meaning": "Top (far) edge of the lane in pixels = lane.region.y + lane.region.height; notes are culled past here",
      "sourceRef": "<reference>/play/LaneRenderer.java:274"
    },
    {
      "name": "hl",
      "meaning": "Judge line (note arrival) Y in pixels; lifted by lift fraction when enablelift",
      "sourceRef": "<reference>/play/LaneRenderer.java:275"
    },
    {
      "name": "rxhs",
      "meaning": "(hu-hl)*hispeed = pixel length of one full measure on screen at current hispeed; the y-integrator multiplier",
      "sourceRef": "<reference>/play/LaneRenderer.java:276"
    },
    {
      "name": "y",
      "meaning": "Accumulated vertical pixel position of the note, integrated segment by segment from hl upward",
      "sourceRef": "<reference>/play/LaneRenderer.java:277"
    },
    {
      "name": "tl.getSection()",
      "meaning": "Cumulative measure index of a timeline (double); delta of 1.0 == one 4/4 measure",
      "sourceRef": "<reference>/play/LaneRenderer.java:363"
    },
    {
      "name": "tl.getMicroTime()",
      "meaning": "Absolute time of a timeline/note in microseconds",
      "sourceRef": "<reference>/play/LaneRenderer.java:329"
    },
    {
      "name": "prevtl.getMicroStop()",
      "meaning": "STOP freeze duration of the previous segment in microseconds; subtracted from s
```

### mechanics[1]

```json
{
  "windows": [
    {
      "judge": "SEVENKEYS (default 7K/14K, judgerank=100) NOTE — PGREAT",
      "window": "[-20000, +20000] µs (±20 ms); JudgeProperty.java:21"
    },
    {
      "judge": "SEVENKEYS NOTE — GREAT",
      "window": "[-60000, +60000] µs (±60 ms); JudgeProperty.java:21"
    },
    {
      "judge": "SEVENKEYS NOTE — GOOD",
      "window": "[-150000, +150000] µs (±150 ms); JudgeProperty.java:21"
    },
    {
      "judge": "SEVENKEYS NOTE — BAD",
      "window": "[-280000, +220000] µs (early -280 ms .. late +220 ms, ASYMMETRIC); JudgeProperty.java:21"
    },
    {
      "judge": "SEVENKEYS NOTE — empty-POOR / MISS window (index4, used for poor & MS detection)",
      "window": "[-150000, +500000] µs (-150 ms .. +500 ms); JudgeProperty.java:21"
    },
    {
      "judge": "Sign convention",
      "window": "dmtime = note.getMicroTime() - pressTime. dmtime>0 = note still in future = EARLY/FAST press; dmtime<0 = LATE/SLOW. Stored array entry is {LATE-lower-bound(negative), EARLY-upper-bound(positive)}; JudgeProperty.java:57, JudgeManager.java:400,673 (fast = mfast>=0)"
    },
    {
      "judge": "SEVENKEYS SCRATCH — PG/GR/GD/BD/MS",
      "window": "PG[-30000,+30000] GR[-70000,+70000] GD[-160000,+160000] BD[-290000,+230000] MS[-160000,+500000] µs; JudgeProperty.java:22"
    },
    {
      "judge": "SEVENKEYS LONGNOTE_END (normal-LN release) — PG/GR/GD/BD",
      "window": "PG[-120000,+120000] GR[-160000,+160000] GD[-200000,+200000] BD[-280000,+220000] µs; JudgeProperty.java:23"
    },
    {
      "judge": "SEVENKEYS LONGSCRATCH_END (BSS release) — PG/GR/GD/BD",
      "window": "PG[-130000,+130000] GR[-170000,+170000] GD[-210000,+210000] BD[-290000,+230000] µs; JudgeProperty.java:25"
    },
    {
      "judge": "FIVEKEYS NOTE — PG/GR/GD/BD/MS",
      "window": "PG[-20000,+20000] GR[-50000,+50000] GD[-100000,+100000] BD[-150000,+150000] MS[-150000,+500000] µs (symmetric); JudgeProperty.java:10"
    },
    {
      "judge": "FIVEKEYS LONGNOTE_END / LONGSCRATCH_END",
      "window": "LN-end PG/GR/GD/BD = [-120000,+120000]/[-150000,+150000]/[-200000,+200000]/[-250000,+250000]; LS-end = [-130000,+130000]/[-160000,+160000]/[-110000,+110000]/[-260000,+260000] µs; JudgeProperty.java:12,14"
    },
    {
      "judge": "PMS (popn 5K/9K) NOTE — PG/GR/GD/BD/MS",
      "window": "PG[-20000,+20000] GR[-50000,+50000] GD[-117000,+117000] BD[-183000,+183000] MS[-175000,+500000] µs (symmetric). scratch[]/longnote/longscratch are EMPTY for PMS. longnoteMargin=200000µs; JudgeProperty.java:32-35"
    },
    {
      "judge": "KEYBOARD (24K) NOTE — PG/GR/GD/BD/MS",
      "window": "PG[-30000,+30000] GR[-90000,+90000] GD[-200000,+200000] BD[-320000,+240000] MS[-200000,+650000] µs. LN-end PG[-160000,+25000] GR[-200000,+75000] GD[-260000,+140000] BD[-320000,+240000] (asymmetric, easier-late). scratch empty; JudgeProperty.java:43-45"
    },
    {
      "judge": "mjudgestart / mjudgeend (scan bounds)",
      "window": "Computed as min of all judge[i][0] and max of all judge[i][1] across NOTE+SCRATCH tables; for 7K NOTE+SCRATCH => mjudgestart=-290000, mjudgeend=+500000 µs; JudgeManager.java:198-206"
    }
  ],
  "ranks": [
    "Judge-rank scaling lives in JudgeProperty.JudgeWindowRule.create() (JudgeProperty.java:168-214). judgerank is a percentage multiplier: judge[i][j] = fixjudge[i] ? org[i][j] : org[i][j] * judgerank / 100. judgerank=100 => default windows above. Lower judgerank => tighter windows (harder).",
    "The 5 named ranks are NOT applied as windows directly; they are judgerank PERCENT presets selected by BMSPlayerRule.validate() from the #RANK header. JudgeWindowRule.NORMAL.judgerank = {25, 50, 75, 100, 125} for {VERYHARD, HARD, NORMAL, EASY, VERYEASY}. JudgeWindowRule.PMS.judgerank = {33, 50, 70, 100, 133}. JudgeProperty.java:156-157.",
    "Mapping of #RANK to judgerank% (BMSPlayerRule.validate, BMSPlayerRule.java:57-71): BMS_RANK type: rank index 0..4 -> windowrule.judgerank[index]; out-of-range -> judgerank[2] (NORMAL=75 for 7K). BMS_DEFEXRANK: judgerank>0 -> defexrank * judgerank[2] / 100, else judgerank[2]. BMSON_JUDGERANK: judgerank>0 -> used as-is (raw %), else 100. So a bmson with judgerank 100 = exactly the µs tables above; a BMS #RANK 2 (NORMAL) yields 75% i.e. PG ±15 ms, GR ±45 ms, GD ±112.5 ms for 7K.",
    "fixjudge array (JudgeProperty.java:156-157, per-window NOT scaled by judgerank): NORMAL fixjudge={false,false,false,false,true} => only the MS(index4) window is fixed; PG/GR/GD/BD all scale. PMS fixjudge={true,false,false,true,true} => PG(index0) and BD(index3) and MS(index4) are FIXED, only GR/GD scale.",
    "Monotonicity clamping after scaling (JudgeProperty.java:176-198): for each non-fixed window i (<4), it is clamped so |window| is not smaller than the nearest lower fixed window (fixmin) and not larger than the nearest higher fixed window (fixmax). Ensures scaled windows stay nested between fixed neighbors.",
    "Custom judge-window-rate (PlayerConfig.isCustomJudge, JudgeManager.java:176-181): per-judge % rates {PG, GR, GD} applied AFTER judgerank scaling (JudgeProperty.java:201-211): judge[i][j] = judge[i][j]*rate[i]/100, then clamped so |window| <= |BD window (index3)| and |window| >= |previous judge window|. Defaults {100,100,100} when not custom; PlayerConfig defaults are PG=GR=400, GD=100 but only used if customJudge=true. Separate key vs scratch rates.",
    "Course constraints (JudgeManager.java:182-190): NO_GREAT sets GR&GD rate to 0 (only PG counts); NO_GOOD sets GD rate to 0."
  ],
  "algorithm": "NOTE-MATCHING (JudgeManager.update, key-DOWN branch, JudgeManager.java:393-506). Per lane the model holds a time-sorted Note[] with a moving cursor (Lane.getNote() returns next note and advances cursor; Lane.reset() rewinds to base; Lane.mark(t) GCs notes older than t — confirmed via javap of bms.model.Lane). On a key press at pmtime: pick mjudge = smjudge if scratch lane else nmjudge. Iterate notes from the lane base cursor: dmtime = note.getMicroTime() - pmtime. If dmtime >= mjudgeend break (too far future); if dmtime < mjudgestart skip (already-past, continue); skip MineNote and LN-END notes. CANDIDATE SELECTION: a note replaces the current target tnote when (tnote==null OR tnote.state!=0 OR algorithm.compare(tnote, judgenote, pmtime, mjudge)). JudgeAlgorithm (JudgeAlgorithm.java): Combo => true if t2 unjudged AND t1.microTime < ptime+table[2][0](GOOD-late) AND t2.microTime <= ptime+table[2][1](GOOD-early) (prefer note that keeps combo); Duration => true if |t1.micro-ptime| > |t2.micro-ptime| AND t2 unjudged (nearest in time wins); Lowest => always false (keep first/lowest note); Score => like Combo but uses table[1] (GREAT window). defaultAlgorithm = {Combo, Duration, Lowest} (JudgeAlgorithm.java:50). Selected via PlayConfig.getJudgetype() name (JudgeManager.java:153).\\nJUDGE COMPUTATION (JudgeManager.java:411-434): if candidate already judged (state!=0) it becomes a POOR-or-MISS: judge = (dmtime within table[4] empty-poor window) ? 5(MS) : 6(skip). If unjudged: linear scan judge index 0..len over windows where dmtime in [table[j][0], table[j][1]]; then judge = (judge>=4 ? judge+1 : judge) — i.e. index0=PG,1=GR,2=GD,3=BD, index4(MS-window match)->5(empty MISS); no-match => judge stays = mjudge.length (>=6) meaning unmatched/ignore. A candidate with judge<4 is accepted as tnote only if it is the first OR closer in |time| than prior tnote (JudgeManager.java:424-431); judge>=6 clears tnote.\\nEMPTY-POOR / POOR (空POOR): if NO target note found (tnote==null) the press produces NO judge counter increment under the default 7K/5K/KB rule (only key-sound of the nearest passed note plays; laser color reset to 0) — JudgeManager.java:478-505. An actual empty-POOR that breaks combo only happens via miss==MissCondition.ONE (PMS, see lnJudge field) and via the 'judged note re-hit' path producing judge=5(MS). MISS-by-passing (見逃しPOOR) is in the LN/poor-sweep block (JudgeManager.java:612-649): any NormalNote/LN-head still unjudged onc
```

### mechanics[2]

```json
{
  "noteModel": "A chart is bms.model.BMSModel holding metadata + `TimeLine[] timelines` (sorted ascending by time), plus `String[] wavmap` (#WAVxx) and `String[] bgamap` (#BMPxx), `Mode mode`, `int lnobj`, `int lnmode`, `double bpm` (initial #BPM). Decoded: BMSDecoder.decode -> Section.makeTimeLines builds the TimeLine[] (BMSModel.setAllTimeLine).\n\nbms.model.TimeLine fields: `long time` (ABSOLUTE microseconds — getMicroTime() returns it raw, getTime()=time/1000 ms, getMilliTime()=time/1000), `double section` (cumulative measure position, 1.0 per 4/4 measure), `Note[] notes` (one slot per lane, length=mode.key), `Note[] hiddennotes` (invisible-channel notes, same indexing), `Note[] bgnotes` (BGM/autoplay, channel 01), `boolean sectionLine` (true on a measure barline timeline), `double bpm` (effective BPM at this line), `long stop` (STOP duration in microseconds, getMicroStop() raw / getMilliStop()=stop/1000), `double scroll` (#SCROLL factor, default 1.0), `int bga` (#BGA layer index, channel 04), `int layer` (channel 07), `Layer[] eventlayer` (POOR/MISS bga sequences, channel 06). ctor TimeLine(double section, long microtime, int lanecount).\n\nbms.model.Note (abstract) fields: `double section`, `long time` (absolute microseconds, copied from owning TimeLine in setNote), `int wav` (index into wavmap, -1 = no sound, -2 = LN-tail with no sound), `long start` (getMicroStarttime — keysound slice start us, 0 for plain BMS), `long duration` (getMicroDuration — slice length us, 0 for plain BMS), `int state` (judge result, runtime), `long playtime` (getMicroPlayTime, runtime judge delta), `Note[] layerednotes` (extra simultaneous keysounds via addLayeredNote/getLayeredNotes). Subclasses: NormalNote(int wav) / NormalNote(int wav,long start,long duration); LongNote(int type) with fields `boolean end`, `LongNote pair`, `int type` (TYPE_UNDEFINED=0, TYPE_LONGNOTE=1, TYPE_CHARGENOTE=2, TYPE_HELLCHARGENOTE=3); MineNote(int wav,double damage) field `double damage`.\n\nRust mirror: Model{ mode, wavmap:Vec<String>, bgamap, lnobj, lnmode, init_bpm, timelines:Vec<TimeLine> }; TimeLine{ time_us:i64, section:f64, notes:Vec<Option<Note>>(len=mode.key), hidden:Vec<Option<Note>>, bgnotes:Vec<Note>, section_line:bool, bpm:f64, stop_us:i64, scroll:f64, bga:i32, layer:i32 }; Note{ kind: Normal|Long{type,is_end,pair_idx}|Mine{damage}, wav:i32, start_us:i64, duration_us:i64, time_us:i64, section:f64, layered:Vec<Note> }.",
  "channelLaneMapping": "BMS channels are read as the 2 chars after \"#xxxCC:\" parsed base-36 (ChartDecoder.parseInt36(charAt(4),charAt(5))). Section static constants (base-36 values): channel 01 (autoplay/BGM)=1 -> bgnotes (TimeLine.addBackGroundNote); 02=2 SECTION_RATE (measure length, see bpmStopScroll); 03=3 BPM_CHANGE (hex); 04=4 BGA_PLAY; 06=6 POOR_PLAY (MISS layer); 07=7 LAYER_PLAY; 08=8 BPM_CHANGE_EXTEND (#BPMxx); 09=9 STOP. P1_KEY_BASE=37 (channel '11'), P2_KEY_BASE=73 ('21'), P1_INVISIBLE_KEY_BASE=109 ('31'), P2_INVISIBLE_KEY_BASE=145 ('41'), P1_LONG_KEY_BASE=181 ('51'), P2_LONG_KEY_BASE=217 ('61'), P1_MINE_KEY_BASE=469 ('D1'), P2_MINE_KEY_BASE=505 ('E1'), SCROLL=1020 ('SC').\n\nWithin a group the raw channel index = (parsedChannel - base): for visible P1 ch '11'..'19' -> index 0..8, P2 '21'..'29' -> index 0..8 (added to a +9 offset for the 18-wide assign array: P2 uses index = (ch-73)+9). This 0..17 raw lane index is remapped to the actual play lane via per-mode arrays:\nCHANNELASSIGN_BEAT7 (used for BEAT_7K & BEAT_14K)= [0,1,2,3,4,7,-1,5,6,8,9,10,11,12,15,-1,13,14] meaning raw 0..4 = keys1-5, raw5 -> lane7 (P1 scratch is lane7? -> it maps key6('16')->scratch idx7, '17'? ) i.e. lane index where 7 is P1 scratch; raw '18'(idx7)->5, '19'(idx8)->6 are keys 6&7; P2 block (idx9..17) similarly -> 8..15.\nCHANNELASSIGN_BEAT5 (BEAT_5K/10K)= [0,1,2,3,4,5,-1,-1,-1,6,7,8,9,10,11,-1,-1,-1] (key1-5 + scratch=5, '18'/'19' unused).\nCHANNELASSIGN_POPN (POPN_9K)= [0,1,2,3,4,-1,-1,-1,-1,-1,5,6,7,8,-1,-1,-1,-1] (channels 11-15 ->0-4, 22-25 ->5-8). A value of -1 means the channel is ignored for that mode. Mode selection: POPN_9K->POPN, BEAT_7K/BEAT_14K->BEAT7, else BEAT5.\n\nMode definitions (from Mode enum): BEAT_5K(key=6,player=1,scratch={5}), BEAT_7K(key=8,player=1,scratch={7}), BEAT_10K(key=12,player=2,scratch={5,11}), BEAT_14K(key=16,player=2,scratch={7,15}), POPN_5K(key=5,player=1,scratch={}), POPN_9K(key=9,player=1,scratch={}), KEYBOARD_24K(key=26,player=1,scratch={24,25}), KEYBOARD_24K_DOUBLE(key=52,player=2,scratch={24,25,50,51}). The scratch lane indices are exactly mode.scratchKey; isScratchKey(i) checks membership. So in 7K lane7 is scratch (channel '16'), lanes0-6 the 7 keys. NOTE_CHANNELS[]= {37,73,109,145,181,217,469,505} are the 8 group bases iterated during decoding. Invisible channels (31-39/41-49) -> hiddennotes[lane]; mine channels (D1.. / E1..) -> MineNote in notes[lane].",
  "timingAssignment": "Section positions accumulate: each measure's base `sectionnum` is the running sum of previous measure lengths; a note token at fraction f in a channel sits at section = sectionnum + rate*f where `rate` is the measure's length (#xxx02, default 1.0) and f = (slotIndex / slotCount). Section.getTimeLine(double section) lazily creates/caches a TimeLine in a TreeMap keyed by section and computes its absolute micro time from the nearest lower cached entry (prev):\n\nscroll = prev.timeline.getScroll(); bpm = prev.timeline.getBPM();\ntime_us = prev.time + prev.timeline.getMicroStop() + 240000000.0 * (section - prev.key) / bpm;\nnew TimeLine(section, (long)time_us, mode.key); setBPM(bpm); setScroll(scroll).\n\nSo 240000000 microsec = duration of one full section (a 4/4 measure) at the segment's BPM; multiplying by the section delta and dividing by BPM gives elapsed us, and any STOP held at the prior line is added. The first/zero section seeds bpm = model initial #BPM, scroll=1.0. This matches the reference implementation's own RhythmTimerProcessor: `timelines[i].getMicroTime() + getMicroStop() + (deltaSection)*240000000/getBPM()`.\n\nIntegration order in makeTimeLines per measure: (1) ensure a sectionLine TimeLine at the measure start (setSectionLine(true)); (2) merge BPM-change, STOP, SCROLL TreeMaps (keyed by in-measure fraction 0..1, ascending) — walking them in position order so each event's TimeLine is built on top of preceding ones (events strictly ordered so cumulative time/stop chain stays monotonic); (3) then place notes per channel via getTimeLine(sectionnum + rate*f). Because every TimeLine is created through getTimeLine, all share the same prev-based chain, guaranteeing monotonic absolute times. TimeLine.setNote(lane,note) copies the line's section and microtime into the Note (Note.time = TimeLine.time), so a note's absolute us == its TimeLine's us. The final TimeLine[] is sorted by time and stored via BMSModel.setAllTimeLine; BMSModel.getAllTimes()/getLastTime derive from it.",
  "bpmStopScroll": "All three are per-TimeLine doubles/longs resolved during decode into TreeMaps then applied in section order.\nBPM: initial from #BPM header (BMSModel.bpm, double). Channel 03 (BPM_CHANGE) values are 2 hex digits: lambda$new$0 stores value = (token/36)*16 + (token%36) i.e. reinterpret the base-36-parsed pair as base-16 (so '03' line tokens are hex BPM). Channel 08 (BPM_CHANGE_EXTEND) token is an index into the #BPMxx table (bpmtable map); resolved to a double (warns 'undefined BPM' if missing). The resolved bpm is set on the TimeLine at that fraction via setBPM and becomes the `prev.bpm` used by getTimeLine for all later lines until the next change.\n\nSTOP: channel 09 token indexes the #STOPxx table (stoptable). The stored stop value S (in units where 192 = one 4/4 measure, per BMS spec; tables hold S directly). Applied: timeline.setStop((long)(240000000.0 * S / timeline.getBPM())). getMicroStop() returns this microsecond value; getTimeLine adds prev.getMicroStop() to the next line's time, so the playhead freezes for that many us. (Units: 240000000 us is one whole measure;
```

### mechanics[3]

```json
I now have the complete timing architecture. I have all the information needed for a faithful Rust reimplementation report.

---

# the reference implementation INPUT + TIMING-SOURCE design (for Rust port)

## 1. The master clock: `System.nanoTime()`, microsecond-quantized, anchored at state entry

The single master clock is `System.nanoTime()` (monotonic). Everything derives from it; `System.currentTimeMillis()` is used only for wall-clock/boot/throttle housekeeping, never for judge.

- `TimerManager` (`<reference>/TimerManager.java`) is the clock owner:
  - `setMainState()` (L101-107) anchors `starttime = System.nanoTime()`.
  - `update()` (L109-111): `nowmicrotime = (System.nanoTime() - starttime) / 1000` — **microsecond** resolution, computed by integer-dividing ns by 1000.
  - All timers are stored in microseconds (`long[] timer`, `getMicroTimer`, `setMicroTimer`); `Long.MIN_VALUE` = "off". Millisecond getters (`getNowTime`, L33) just divide the micro value by 1000.
- `getNowMicroTime(TIMER_PLAY)` (L48-53) = `nowmicrotime - timer[TIMER_PLAY]` is the **song play position in µs** and is the time base everything in play is judged against.

## 2. Input timestamps: polled, not event-driven; timestamped on a dedicated thread

Input is **polled**, not delivered as timestamped OS events. The libGDX `InputProcessor` callbacks in `KeyBoardInputProcesseor.java` (`keyDown`/`keyUp`, L79-90) are essentially inert for gameplay — they only stash `lastPressedKey`. Actual key state for judging is read via `Gdx.input.isKeyPressed(...)` during polling.

**Dedicated polling thread (separate from the vsync render loop)** — `MainController.create()` (`MainController.java` L357-372):
```java
Thread polling = new Thread(() -> {
    long time = 0;
    for (;;) {
        final long now = System.nanoTime() / 1000000;   // ms tick
        if (time != now) { time = now; input.poll(); }
        else { Thread.sleep(0, 500000); }               // 0.5ms nap
    }
});
polling.start();
```
So input is polled at a **~1 kHz cadence (1 ms)** on its own thread, busy-spinning with 0.5 ms sleeps between ms ticks. The `render()` loop's old inline `input.poll()` is commented out (L412) — input is deliberately decoupled from vsync.

**Timestamp assignment** — `BMSPlayerInputProcessor.poll()` (L498-504):
```java
final long now = System.nanoTime() / 1000 - starttime;   // µs since play start
kbinput.poll(now); for (controller) controller.poll(now);
```
The microsecond `now` is captured **once per poll tick** and threaded into every device's `poll(microtime)`. Each device records the edge time with that value:
- Keyboard (`KeyBoardInputProcesseor.poll`, L99-112): on a state change, `keytime[key] = microtime` and calls `bmsPlayerInputProcessor.keyChanged(this, microtime, i, pressed)`. A debounce gate `microtime >= keytime + duration*1000` enforces a minimum re-trigger interval (`duration` ms, default 0 for KB).
- Controller (`BMControllerInputProcessor.poll`, L126-190): `buttontime[button] = microtime` on change, default `duration = 16 ms` debounce; analog scratch resolved to button edges via `AnalogScratchAlgorithm`.

`keyChanged()` (`BMSPlayerInputProcessor.java` L326-338) is the convergence point: it updates `keystate[i]`/`time[i] = presstime` and appends to the keylog `keylog.add(presstime - microMarginTime, i, pressed)`.

**Two distinct time origins** (this matters for the port):
- During `input.poll()` the `starttime` used is `BMSPlayerInputProcessor.starttime`, set by `input.setStartTime(...)` at `STATE_READY → STATE_PLAY` (`BMSPlayer.java` L605): `micronow + timer.getStartMicroTime() - starttimeoffset*1000`. So once play starts, **input timestamps are already in the song's play-time frame (µs)**, the same axis as note times. Before play start, `starttime == 0`, so timestamps are raw `nanoTime/1000` and no keylog is recorded (`if (starttime != 0)`).

## 3. The judge clock: a THIRD thread, reading `TIMER_PLAY` micro-time

Judging runs on yet another dedicated thread — `KeyInputProccessor.JudgeThread` (`play/KeyInputProccessor.java` L140-213):
```java
final long mtime = player.timer.getNowMicroTime(TIMER_PLAY);  // µs play position
if (mtime != prevtime) {
    ... replay injection ...
    judge.update(mtime);
    prevtime = mtime;
} else { sleep(0, 500000); }   // 0.5ms nap, spins ~ when µs ticks
```
`JudgeManager.update(mtime)` (`play/JudgeManager.java` L220, L352-411) reads the input edge time `pmtime = input.getKeyChangedTime(key)` (the per-key µs timestamp set during input polling) and computes the judge delta directly in microseconds: `dmtime = judgenote.getMicroTime() - pmtime`, then bins it against µs judge windows (`nmjudge`/`smjudge`). **Crucially the judge delta is `note_time - input_press_time`, both in µs — it does NOT use the frame/render time of when the judge ran.** The judge thread's own cadence only affects how soon a press is processed, not the recorded accuracy.

## 4. Threading summary (3 clocks/threads off one monotonic source)

| Thread | Cadence | Reads | Writes |
|---|---|---|---|
| Render (libGDX main) | vsync (~60-240 Hz) | `timer.update()` (L413) → advances `nowmicrotime`; draws | timers, skin |
| Input polling | ~1 kHz (1 ms ticks, 0.5 ms naps) | `Gdx.input.isKeyPressed`, `controller.getAxis/Button` | `keystate[]`, `time[]` (µs), keylog |
| Judge | spins on µs change of `TIMER_PLAY` | `getKeyChangedTime` (µs), note times | judges, gauge, score |

`TIMER_PLAY` itself is advanced on the **render thread** via `timer.update()` (it's `nanoTime`-based, so `getNowMicroTime` is live even between render frames because it recomputes from `System.nanoTime()` each call — L109-111 / L44-46). So the judge thread does get sub-frame-fresh `mtime` because `getNowMicroTime()` recalculates from `nanoTime` on every call, **not** from a frame-cached value.

## 5. The jitter problem to FIX in the Rust port

Despite µs timestamps, the reference implementation has **two residual quantization sources**, and they are the thing to fix:

1. **Input poll quantization (~1 ms).** Key state is sampled only when the polling thread ticks (1 ms granularity, gated further by `Thread.sleep(0, 500000)` whose real resolution on Windows/JVM is often 1-15 ms). A press landing between ticks is stamped with the *poll tick's* `nanoTime`, not the true press instant. So judge timestamps carry up to ~1 ms (often worse on Windows timer-resolution) of input-sampling jitter. This is inherent to polling `isKeyPressed` rather than consuming OS event timestamps.
2. **No vsync quantization in the judge itself** — this part is already correct: judging uses `note.getMicroTime() - pmtime`, both µs, independent of the render frame. The classic "judge clock = vsync" bug is **already avoided** here by (a) the separate judge thread and (b) `getNowMicroTime()` recomputing from `nanoTime` live. The earlier inline `input.poll()` in render (now commented out, L412) is exactly the vsync-coupled design they moved away from.
3. **`TIMER_PLAY` is advanced on the render thread** (`timer.update()` in `render()`), and audio/BGM start (`keysound.startBGPlay`) is keyed off the same wall clock — there is **no hardware audio clock**. Song position is `nanoTime`-derived, so any drift between `nanoTime` and the actual audio device playback clock is uncorrected. Notes are judged against the `nanoTime` timeline, while what the player *hears* is the audio device timeline — a slowly accumulating A/V-vs-judge skew.

### Recommended Rust mapping

- **Master clock**: keep one monotonic source, but make it the **audio device clock** for play position, not `Instant::now()`. Drive `play_position_us` from the audio backend's played-frame count (cpal/rodio stream position), so judge time == what the player hears. Use `Instant`/TSC only as a fallback and to interpolate between audio callbacks.
- **Input**: do **not** poll `is_key_pressed` on a software 1 ms loop. Use **event-driven, OS-timestamped input** (raw input / `winit`/`gilrs` device events, or a dedicated input thr
```

## 포맷 스펙

### BMS

I now have comprehensive, authoritative information confirmed from both the hitkey BMS command memo and the reference implementation's actual parser bytecode (`jbms-parser.jar`). I have enough to compile the precise, implementable spec. Let me compile the final answer.

---

# BMS Family Chart Format — Implementable Spec for a Rust Parser

Every claim is cited inline. Two source classes:
- **Format spec** — *BMS command memo* (hitkey wiki) `https://hitkey.nekokan.dyndns.info/cmds.htm` and its sub-pages, plus Wikipedia "Be-Music Source".
- **the reference implementation's actual behavior** — decompiled from `lib/jbms-parser.jar` in this repo (`bms.model.BMSDecoder`, `bms.model.ChartDecoder`), since that's the parser that produces the MD5/SHA-256 keys used for scores/IR/replays. Cited as `[jbms-parser]`.

> CRITICAL implementation note up front: the reference implementation keys everything on a hash computed over the **raw, undecoded file bytes**, and its object-id parser is **strict base-36 (case-insensitive), NOT base-62**. Both verified from bytecode below.

---

## 1. File / line structure

- A BMS file is line-based text. Lines that begin with `#` are **command lines**; everything else (and anything after a non-matching prefix) is ignored as comment. Command lines are **case-insensitive**. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- Two kinds of command line:
  - **Header**: `#NAME value` — delimiter is **one half-width space** between name and value.
  - **Channel (main data)**: `#xxxCC:value` — delimiter is **one half-width colon** `:`. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- When the **same header appears twice, the one nearer EOF wins**. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- **Encoding**: files are historically Shift_JIS; the reference implementation decodes text as **`MS932`** (Microsoft's Shift_JIS superset) for parsing. Verified `[jbms-parser]`: `new InputStreamReader(.., "MS932")`. (It also probes for UTF-8/UTF-16 BOM in the broader codebase, but MS932 is the default code path.) The only universally safe charset is ASCII. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- File extensions: `.bms` (5-key original), `.bme` (adds 7-key channels 18/19/28/29), `.bml` (long-note variant), `.pms` (pop'n / 9-button). the reference implementation decides POPN_9K vs BEAT_5K mode partly by the `.pms` extension. Verified `[jbms-parser]`: `decode(Path)` checks `path.toLowerCase().endsWith(".pms")` and sets `Mode.POPN_9K` else `Mode.BEAT_5K`.

---

## 2. Header commands

Syntax `#NAME value` (space delimiter). All confirmed present in the reference implementation's parser via constant-pool strings unless noted `[hitkey only]`. Source for semantics: `https://hitkey.nekokan.dyndns.info/cmds.htm`.

| Command | Value | Meaning |
|---|---|---|
| `#PLAYER` | `1`-`4` | Play mode: 1=SP, 2=Couple(obsolete), 3=DP, 4=Battle. the reference implementation validates P2 notes vs this value. `[jbms-parser]` |
| `#GENRE` | string | Genre text. |
| `#TITLE` | string | Song title. |
| `#SUBTITLE` | string | Subtitle (appended/shown under title). |
| `#ARTIST` | string | Artist. |
| `#SUBARTIST` | string | Secondary artist/credits. |
| `#BPM` | number (float ok) | **Initial/base BPM** of the chart. |
| `#BPMxx` | number (float ok) | Defines an indexed BPM (`xx` = base-36 id) referenced by channel `08`. `[jbms-parser]` parses it; "#BPMxxに数字が定義されていません" error string present. |
| `#PLAYLEVEL` | integer | Displayed difficulty level (cosmetic). |
| `#RANK` | `0`-`3` (sometimes 4) | Judge window strictness: **0=VERY HARD, 1=HARD, 2=NORMAL, 3=EASY**. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`. the reference implementation stores it as `JudgeRankType.BMS_RANK`. `[jbms-parser]` |
| `#DEFEXRANK` | integer (percent) | Finer judge difficulty; `100` ≈ `#RANK 2 (NORMAL)`. the reference implementation: `JudgeRankType.BMS_DEFEXRANK`. Source: hitkey; `[jbms-parser]`. |
| `#TOTAL` | number | **Total groove-gauge recovery (%) for a perfect play.** On Normal gauge, recovery per note = `TOTAL / noteCount`; `TOTAL=500` ⇒ a perfect play recovers 500% of gauge. Low TOTAL (<~240) reduces recovery. Source: `https://github.com/wcko87/the reference implementation-english-guide/wiki/Scores-and-Clears`. the reference implementation warns "TOTALが未定義です"/"TOTAL値が少なすぎます". `[jbms-parser]` |
| `#STAGEFILE` | filename | Loading/jacket splash image. |
| `#BANNER` / `#BACKBMP` | filename | Banner / play-background images `[hitkey]`. |
| `#WAVxx` | filename | Keysound sample; `xx` = base-36 id (1–1295). the reference implementation: `wavlist[1296]`. `[jbms-parser]` |
| `#BMPxx` | filename | BGA image/video; `xx` = base-36 id. the reference implementation: `bgalist[1296]`. `[jbms-parser]` |
| `#STOPxx` | integer | STOP-sequence definition referenced by channel `09`. **Unit = 1/192 of a 4/4 measure**, so `192` = one whole 4/4 measure of stop. Negative STOP unsupported in the reference implementation ("negative STOPはサポートされていません"). Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`; `[jbms-parser]`. |
| `#SCROLLxx` | number (float) | Scroll-speed multiplier definition, referenced by the `SC` channel (레퍼런스 구현/bemuse extension). the reference implementation keeps a `scrolltable`; warns "#SCROLLxxは不十分な定義です". `[jbms-parser]` |
| `#LNTYPE` | `1` or `2` | Long-note notation mode (see §6). the reference implementation field `lntype`. `[jbms-parser]` |
| `#LNOBJ` | `xx` | Base-36 object id that marks an LN **end** on visible channels (see §6). Source: hitkey + Wikipedia. |
| `#DIFFICULTY` | `1`-`5` | Difficulty slot/label (1=BEGINNER … 5=INSANE) `[hitkey]`. |
| `#VOLWAV` | integer | Master keysound volume % `[hitkey]`. |
| `#COMMENT`, `#MIDIFILE`, `#OGGxx` | — | misc `[hitkey/Wikipedia]`. |

Base BPM/initial-BPM: the `#BPM` header sets the chart's starting tempo; mid-song changes use channels `03`/`08`. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.

---

## 3. Main data field: `#xxxCC:....`

- Form: `#XXXYY:ZZZZZZ`. `XXX` = **measure number** (000–999, decimal). `YY` = **channel** (2-char). `ZZ…` = a sequence of 2-char **object ids**. The `:` is required. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`, Wikipedia "Be-Music Source".
- The object list **divides the measure into equal intervals**: N two-char objects ⇒ N equal subdivisions; e.g. `0011` = two half-measure positions, `00110000` = quarter positions. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- `00` = **empty / rest** (no object at that slot). Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- **Merging**: multiple lines for the same `measure+channel` are **compounded (overlaid)** — EXCEPT channels `01` (BGM stacks as separate layers anyway), `02` (measure length — last wins), and `A6` (options). Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.

### Object-id radix — base-36 vs base-62 (decisive for the reference implementation)

- The spec **historically** allowed three radices for the 2-char id: hexadecimal (256), "limited base-36" first-char-hex (576), and full **base-36** `[0-9A-Za-z]` ⇒ 1296 slots (`00`=empty, `01`–`ZZ` = 1–1295). Some modern tools use **base-62** to reach 3843 slots. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- **the reference implementation is strictly base-36 and case-insensitive — it does NOT support base-62.** Verified from `ChartDecoder.parseInt36(char, char)` bytecode `[jbms-parser]`:
  - `'0'..'9'` → 0–9
  - `'a'..'z'` → 10–35
  - `'A'..'Z'` → 10–35 (same as lowercase!)
  - any other char → returns `-1` (NumberFormatException upstream)
  - value = `first*36 + second`.
  
  Because `'A'` and `'a'` both map to 10, a base-62 file is silently mis-parsed by the reference implementation. A Rust parser targeting the reference implementation-compatible hashes/IDs must replicate this exact base-36, case-insensitive mapping.

---

## 4. Channel list

Channel codes are 2 chars. The numeric channels below are conventionally read as if hex/base-36; the reference implementation runs the channel string through `parseInt36` too. Source for meanings: `https://hitkey.nekokan.dyndns.info/cmds.htm` and Wikipedia.

| Channel | Meaning |
|---|---|
| `01` | **BGM** (auto-play keysound layer; multiple `01` lines stack as parallel BGM tracks) |
| `02` | **Measure length** (time-signature scaler — see §5) |
| `03` | **BPM change (inline)** — value is a **2-digit hexadecimal** `01`–`FF` ⇒ BPM 1–255 (integer only). `00` = no change. Source: `https://hitkey.nekokan.dyndns.info/exbpm-object.htm` |
| `04` | **BGA-BASE** (main background image/animation layer) |
| `05` | Extended/sequence objects (rarely used) |
| `06` | **BGA-POOR** (image shown on a miss) |
| `07` | **BGA-LAYER** (overlay on top of BASE, transparent-keyed) |
| `08` | **Extended BPM** — object id references a `#BPMxx` definition (float BPM). Must be base-36. Source: `https://hitkey.nekokan.dyndns.info/exbpm-object.htm` |
| `09` | **STOP** — object id references a `#STOPxx` definition. Source: hitkey |
| `SC` | **SCROLL** — object id references `#SCROLLxx` (scroll-speed multiplier). 레퍼런스 구현/bemuse extension. Source: search result + `[jbms-parser]` scrolltable |
| `11`–`15` | **P1 visible** keys 1–5 |
| `16` | **P1 visible SCRATCH** (turntable) |
| `17` | **P1 FREE-ZONE / foot pedal** (5-key era) |
| `18`–`19` | **P1 visible** keys 6–7 (BME 7-key extension) |
| `21`–`25` | **P2 visible** keys 1–5 |
| `26` | **P2 visible SCRATCH** |
| `27` | **P2 FREE-ZONE** |
| `28`–`29` | **P2** keys 6–7 |
| `31`–`35`,`36`,`37`,`38`,`39` | **P1 invisible** objects (keysound-only, mirror 11–19 layout; `36`=invisible scratch) |
| `41`–`49` | **P2 invisible** objects (mirror 21–29) |
| `51`–`59` | **P1 long notes** (mirror the 11–19 lane layout; `56`=LN scratch) |
| `61`–`69` | **P2 long notes** (mirror 21–29) |
| `D1`–`D9` | **P1 mines / landmines** (mirror lane layout) |
| `E1`–`E9` | **P2 mines / landmines** |
| `A0` | extended option / judge change `[hitkey]` |
| `A6` | option change (not merged) `[hitkey]` |

Sources: channel table from `https://hitkey.nekokan.dyndns.info/cmds.htm`; LN channels (51–69) and `#LNTYPE`/`#LNOBJ` origin (NvyU's 2001 RDM proposal using `#xxx51-69`) from `https://hitkey.nekokan.dyndns.info/cmds.htm`.

### 7-key lane assignment
For 7-key (`.bme`), P1 visible lanes are: keys 1–7 on channels `11,12,13,14,15,18,19` and **scratch on `16`** (channel `17` free-zone is unused in 7K). P2 analogously on `21–29` with scratch `26`. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.

### Mine damage encoding
Mine channels `D1–D9`/`E1–E9` carry an object id whose **value encodes damage** (the 2-digit base-36/decimal amount), not a `#WAV` reference. The explosion sound uses `#WAV00`. The hitkey memo notes the damage-amount semantics but does not give a single formula; treat the id value as the damage magnitude. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.

---

## 5. Measure length — channel `#xxx02`

- The value is a **decimal multiplier relative to a 4/4 measure**: `1` = 4/4 (full), `0.75` = 3/4, `2` = 8/4, etc. It scales the measure's duration in beats by `4 * value`. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm` ("The value 1 is 4/4 meter", "The value 0.75 is 3/4 meter").
- Unlike note channels, `02`'s value is a **single decimal number**, not a list of 2-char objects. Only one `02` per measure is meaningful (last wins). Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- Measures with no `02` line default to length `1` (4/4). Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.
- Timing/positions of all objects in a measure are computed against this scaled length; BPM (base + `03`/`08` changes) plus STOP (`09`) then convert measure-fractions to real time.

---

## 6. Long notes — `#LNTYPE` / `#LNOBJ`

Three coexisting LN mechanisms. Source for all quotes: `https://hitkey.nekokan.dyndns.info/cmds.htm`.

1. **LN channels `51–69` + `#LNTYPE 1` (RDM / Type 1, the de-facto standard, NvyU 2001):**
   - "It will be a LN start point if the index which is not `00` is found. It will be a LN end point if the index which is not `00` is found next." — i.e. **pairs of non-`00` objects on the LN channel**: 1st = head, 2nd = tail, 3rd = next head, 4th = next tail, …
2. **LN channels `51–69` + `#LNTYPE 2` (MGQ / Type 2):**
   - "LN section will be opened when indexes other than `00` are found. LN section will be filled up with indexes other than `00`. LN section will be closed when the next `00` is found." — i.e. a **run of non-`00` objects** is one LN; the first `00` after it ends the LN.
3. **`#LNOBJ` on visible channels `11–29`:**
   - "The object of the index specified as `#LNOBJ` is defined as an end point symbol of LN. The object which is just before that … is interpreted as a LN start point." — a normal note placed before an `#LNOBJ`-valued object on the same visible lane becomes the LN head; the `#LNOBJ` object is the tail. This lets `.bms`/`.bme` express LNs without the 51–69 channels.

the reference implementation honors `#LNTYPE` (field `lntype`, default settable via constructor) and `#LNOBJ`. `[jbms-parser]`

---

## 7. Control flow — `#RANDOM` / `#IF` blocks

Confirmed parsed by the reference implementation (constant-pool strings `RANDOM`, `ENDIF`, `ENDRANDOM`, plus mismatch warnings). `[jbms-parser]` Semantics from `https://hitkey.nekokan.dyndns.info/cmds.htm`.

**RANDOM block:**
```
#RANDOM n        ; pick a random integer in [1, n]
   ...           ; (or #SETRANDOM n to force the value, for testing)
   #IF m         ; include the following block only if the drawn value == m
   #ELSEIF m     ; (optional)
   #ELSE         ; (optional)
   #ENDIF        ; closes the IF
#ENDRANDOM       ; closes the RANDOM scope (optional in many parsers)
```
- `#RANDOM n` draws once; nested `#IF/#ELSEIF/#ELSE/#ENDIF` gate which commands (headers AND channel lines) are active. RANDOM blocks may be **nested**. Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`. the reference implementation keeps stacks `randoms`/`crandom`/`skip` to implement nesting. `[jbms-parser]`

**SWITCH block** (less common; supported by some clients incl. the reference implementation-adjacent tooling):
```
#SWITCH n        ; (or #SETSWITCH n)
   #CASE m
      ...
      #SKIP      ; break out (like a break)
   #DEF          ; default case
#ENDSW
```
Source: `https://hitkey.nekokan.dyndns.info/cmds.htm`.

Implementation order for a parser: resolve RANDOM/SWITCH **while reading lines** (a line is processed only if its enclosing IF/CASE branch is active), exactly as the reference implementation does — it evaluates control flow inline during the same single pass that feeds the digest. `[jbms-parser]`

---

## 8. Hash computation (MD5 + SHA-256) — what the reference implementation actually keys on

This is the load-bearing part for scores/IR/replays, so it's taken straight from the reference implementation's parser bytecode, not from prose specs.

**the reference implementation hashes the ENTIRE RAW FILE BYTES, before any decoding/parsing.** Verified from `bms.model.BMSDecoder` `[jbms-parser]`:

1. `decode(Path)` calls `Files.readAllBytes(path)` → a `byte[]` of the **complete, untouched file** (no normalization, no charset conversion, no whitespace trimming).
2. Inside `decode(Path, byte[], boolean, int[])`:
   ```
   MessageDigest md5    = MessageDigest.getInstance("MD5");
   MessageDigest sha256 = MessageDigest.getInstance("SHA-256");
   new BufferedReader(
     new InputStreamReader(
       new DigestInputStream(                 // SHA-256 (outer)
         new DigestInputStream(               // MD5 (inner)
           new ByteArrayInputStream(rawBytes), // the raw file bytes
           md5),
         sha256),
       "MS932"))
   ```
   The two `DigestInputStream`s wrap the raw `ByteArrayInputStream`. As the reader pulls bytes to parse, **every raw byte flows through both digests**. The `MS932` charset is applied by `InputStreamReader` *after* the digesting streams, so **the hash is independent of charset decoding** — it's over the bytes on disk.
3. The parse loop is `while ((line = readLine()) != null)`, which reads to **EOF**, so all bytes are consumed.
4. After the loop: `md5.digest()` and `sha256.digest()` finalize, then `convertHexString(byte[])` formats them and `model.setMD5(...)` / `model.setSHA256(...)` store them.
5. `convertHexString` uses `Character.forDigit(nibble, 16)`, which yields **lowercase** hex. So:
   - **MD5 = 32-char lowercase hex** of the raw file bytes.
   - **SHA-256 = 64-char lowercase hex** of the raw file bytes.

the reference implementation then reads these via `model.getMD5()`/`model.getSHA256()` in `SongData.java` (this repo, lines 145–146) and stores them as the song's `md5`/`sha256` keys.

**Rust implementation:** read the file as raw bytes (`std::fs::read`), feed those exact bytes to `md5` and `sha2::Sha256`, and `format!("{:x}")` (lowercase hex). Do **not** trim BOM, re-encode, normalize line endings, or strip whitespace before hashing — the reference implementation does none of that. The MD5 and SHA-256 are computed over **identical** input (the whole file). Source: `[jbms-parser]` `bms.model.BMSDecoder`; consumer `/Users/hyunseokbyun/<reference>/song/SongData.java`.

> Note: the reference implementation keys scores/IR/replays on **both** hashes — MD5 is the legacy/LR2-compatible key, SHA-256 is the modern primary key. (LR2 also uses MD5-over-raw-bytes, so MD5 matches across clients.) Both come from the same raw-byte stream above.

---

## 9. Practical parser pipeline (recommended order)

1. `bytes = read(file)`. Compute `md5(bytes)` and `sha256(bytes)` (lowercase hex) **first**, from raw bytes.
2. Decode `bytes` as Shift_JIS/MS932 (fallback UTF-8 if BOM) → lines. Hashing must NOT depend on this step.
3. Single pass over lines, maintaining RANDOM/SWITCH branch stacks; only process lines in active branches.
4. For each `#NAME value` header: store (last-wins). For `#WAVxx/#BMPxx/#BPMxx/#STOPxx/#SCROLLxx`: index by `parseInt36(xx)`.
5. For each `#xxxCC:list`: parse `xxx` decimal measure, `CC` channel, split `list` into 2-char ids, each `parseInt36` (base-36, case-insensitive, `00`=empty). Place at fraction `i/N` of the measure.
6. Apply channel `02` to scale each measure; apply base `#BPM` + channels `03`/`08` for tempo, `09` for stops, `SC` for scroll; resolve LNs per `#LNTYPE`/`#LNOBJ`.

---

## Sources

- BMS command memo (hitkey wiki) — primary format spec: [`https://hitkey.nekokan.dyndns.info/cmds.htm`](https://hitkey.nekokan.dyndns.info/cmds.htm)
- Extended BPM (channels 03/08, `#BPMxx`): [`https://hitkey.nekokan.dyndns.info/exbpm-object.htm`](https://hitkey.nekokan.dyndns.info/exbpm-object.htm)
- STOP sequence (`#STOPxx`, channel 09): [`https://hitkey.nekokan.dyndns.info/exstop.htm`](https://hitkey.nekokan.dyndns.info/exstop.htm)
- Wikipedia "Be-Music Source" (`#xxxYY:ZZ` form, base-36, header list): [`https://en.wikipedia.org/wiki/Be-Music_Source`](https://en.wikipedia.org/wiki/Be-Music_Source)
- `#TOTAL` gauge semantics: [`https://github.com/wcko87/the reference implementation-english-guide/wiki/Scores-and-Clears`](https://github.com/wcko87/the reference implementation-english-guide/wiki/Scores-and-Clears)
- the reference implementation parser behavior (base-36 `parseInt36`, raw-byte MD5+SHA-256 via DigestInputStream, MS932, lowercase hex, control-flow strings) — decompiled `bms.model.BMSDecoder` / `bms.model.ChartDecoder` from `/Users/hyunseokbyun/<reference>/lib/jbms-parser.jar` (jbms-parser upstream: [`https://github.com/exch-bms2/the reference implementation`](https://github.com/exch-bms2/the reference implementation))
- Hash consumer in this repo: `/Users/hyunseokbyun/<reference>/song/SongData.java` (lines 145–146, 209)

**Two findings a generic-spec-only implementer would get wrong, both verified against the reference implementation's bytecode:** (1) object ids are **strict base-36, case-insensitive** in the reference implementation — `A`==`a`, no base-62; (2) the MD5/SHA-256 are over the **entire raw file bytes** (lowercase hex), with charset decoding applied only *downstream* of the digest, so hashing must be byte-exact and charset-independent.

### bmson

I now have everything needed, including the reference implementation's exact timing formulas. I have authoritative data from both the official bmson-spec (1.0.0) and the actual 레퍼런스 구현/jbms-parser implementation (which is what the user's parser must be compatible with). Here is the complete spec.

---

# bmson Format — Implementable Spec for a Rust Parser

This covers the **official bmson 1.0.0** spec and the **legacy 0.21** lineage, plus the **the reference implementation extensions** (`scroll_events`, `mine_channels`, `key_channels`) that are *not* in the official spec but are required if you want the reference implementation-compatible parsing. I separate "official" from "the reference implementation-extension" throughout so you know what's portable.

## 1. File format & versioning

- bmson is a single **UTF-8 JSON** document. Recommended extension `.bmson`.
- `version` (string) selects schema. Compare with **Semantic Versioning 2.0.0**.
  - `1.0.0` — current official (released 2015-12-26).
  - `0.21` and earlier — **legacy: the `version` field is absent**. A robust parser must treat a missing `version` as pre-1.0 and apply legacy fallbacks. The structural difference that matters most: pre-1.0 files (BMSON 0.21, from early BmsONE) may use the field name **`info`** the same way, but BGA event handling and some defaults differed; the safest behavior (what the reference implementation does) is to parse leniently with defaults and not hard-fail on a missing `version`.
- A player decides support via **`version` + `info.mode_hint`**. Unknown `mode_hint` → fall back (the reference implementation falls back to `beat-7k` with a warning).

## 2. The pulse / y / resolution timing model

This is the heart of the format. **Everything is positioned by an integer pulse number `y`, independent of time signature.**

- Three clocks: **metric time `t`** (seconds), **musical time `b`** (beats; 1 beat = 1 quarter note), **pulse time `y`** (integer pulses).
- `info.resolution` = **pulses per quarter note**. Default **240**, must be `> 0`.
- A 4/4 measure = 4 quarter notes = `4 × resolution` pulses (= 960 at default). The default bar line spacing is **960 pulses**.
- At song start the tempo is `info.init_bpm`. A `bpm_event` changes tempo at its `y`. A `stop_event` pauses the scroll for `duration` pulses' worth of time at its `y`.
- **Event ordering when multiple events share the same `y`** (official): `Note`/`BGAEvent` → `BpmEvent` → `StopEvent`.

### Converting pulse → seconds (segment integration)

Between consecutive BPM changes the tempo is constant, so integrate piecewise. For a pulse `y` falling in a segment that starts at pulse `y₀` with tempo `bpm`:

```
seconds_per_pulse = 60 / (bpm * resolution)          // resolution = pulses per quarter note
time(y) = time(y₀) + (y - y₀) * 60 / (bpm * resolution)
```

Add each `stop_event`'s pause to the running time at its `y`. A stop of `duration` pulses at tempo `bpm`:

```
stop_seconds = duration * 60 / (bpm * resolution)
```

**the reference implementation-specific note (important for byte-exact compatibility):** the reference implementation internally uses `RESOLUTION = info.resolution * 4` (pulses per **whole note / measure**, default 960), and computes (microseconds):

```
time(y) = time(y₀) + 240000 * 1000 * (y - y₀) / (bpm * RESOLUTION)      // 240000 = 60s * 4 quarters * 1000ms
stop_us = 1000 * 1000 * 60 * 4 * duration / (bpm * RESOLUTION)
section(y) = y / RESOLUTION                                              // measure index, fractional
```

Both formulations are algebraically identical (`RESOLUTION = resolution*4`, so `60*4/RESOLUTION = 60/resolution`). Use the first form with `resolution = pulses-per-quarter` in your own engine; mirror the `*4` only if you must match the reference implementation's measure/section numbers exactly.

the reference implementation **rejects negative BPM and negative STOP** with a warning (does not apply them). Stop time uses the **BPM in effect at that timeline** (`tl.getBPM()`), so order BPM-then-STOP per pulse.

## 3. Top-level object

| Field | Type | Req | Default | Notes |
|---|---|---|---|---|
| `version` | string | yes (1.0.0) | — | absent in 0.21 |
| `info` | Info | yes | — | header |
| `lines` | BarLine[] | no | null/[] | bar lines |
| `bpm_events` | BpmEvent[] | no | null/[] | tempo changes |
| `stop_events` | StopEvent[] | no | null/[] | pauses |
| `sound_channels` | SoundChannel[] | yes | — | keysounded notes |
| `bga` | BGA | yes (official) | — | background |
| **`scroll_events`** | ScrollEvent[] | no | [] | **the reference implementation ext** (Hi-Speed scroll-rate changes) |
| **`mine_channels`** | MineChannel[] | no | [] | **the reference implementation ext** (damage/mine notes) |
| **`key_channels`** | MineChannel[] | no | [] | **the reference implementation ext** (invisible keysound-only notes; same shape as mine_channels in jbms-parser) |

Treat `null` arrays as empty. In the reference implementation's model `key_channels` reuses the **`MineChannel`** Java type (`name` + `MineNote[]`), but its notes are invisible keysound triggers, not damage.

## 4. Info object

Official names; the reference implementation's class is `BMSInfo` with one extra field `ln_type`.

| Field | Type | Req | Default | Notes |
|---|---|---|---|---|
| `title` | string | yes | "" | display title |
| `subtitle` | string | no | "" | may contain newlines |
| `artist` | string | yes | "" | primary creator |
| `subartists` | string[] | no | [] | `"key:value"` entries (e.g. `"obj:foo"`) |
| `genre` | string | yes | "" | |
| `mode_hint` | string | no | `"beat-7k"` | lane layout selector (see §10) |
| `chart_name` | string | no | "" | e.g. "HYPER", "ANOTHER" |
| `level` | unsigned long | yes | 0 | difficulty rating ≥ 0 |
| `init_bpm` | number | yes | — | start tempo; **fatal if unspecified** per spec |
| `judge_rank` | number | no | 100 | judge-window % of player default; >100 = wider/easier |
| `total` | number | no | 100 | lifebar gain multiplier ≥ 0; 0 = lifebar never increases |
| `back_image` | string | no | null/"" | background image |
| `eyecatch_image` | string | no | null/"" | loading image |
| `banner_image` | string | no | null/"" | banner (≈15:4) |
| `preview_music` | string | no | null/"" | preview audio |
| `resolution` | unsigned long | no | 240 | pulses per quarter note, > 0 |
| **`ln_type`** | int | no | 0 | **the reference implementation ext**: LN mode (used only when 1–3) |

the reference implementation interpretation quirks worth replicating: `judge_rank < 0` → warn; `0 ≤ judge_rank < 5` → used verbatim but warned ("not spec-conformant" — i.e. it accepts both the BMS-style small rank and the bmson percentage). `total > 0` → used with `TotalType.BMSON`; `total ≤ 0` → warn and ignore.

## 5. BarLine / lines

| Field | Type | Req | Notes |
|---|---|---|---|
| `y` | unsigned long | yes | pulse of the bar line |
| `k` | int | no | **the reference implementation ext** "kind" (line type) |

The first bar line at `y: 0` may be omitted. If `lines` is empty, assume 4/4 with a bar line every `4*resolution` (960) pulses.

## 6. BpmEvent / StopEvent / ScrollEvent

All three extend a base carrying **`y`** (the pulse position).

**BpmEvent**
| `y` | unsigned long | pulse |
| `bpm` | number | new tempo (must be > 0; the reference implementation rejects ≤ 0) |

**StopEvent**
| `y` | unsigned long | pulse where pause starts |
| `duration` | unsigned long | pause length **in pulses** (the reference implementation rejects < 0) |

**ScrollEvent** (the reference implementation extension — visual scroll-rate / Hi-Speed multiplier, does **not** affect judging time)
| `y` | unsigned long | pulse |
| `rate` | number | scroll multiplier, default `1.0` |

## 7. SoundChannel & Note

```
SoundChannel { name: string, notes: Note[] }
Note { x, y, l?, c? }
```

| Field | Type | Req | Default | Meaning |
|---|---|---|---|---|
| `name` | string | yes | — | audio filename (extension may be omitted; resolve by trying common audio extensions) |
| `x` | int / null | yes | — | **lane**. `0` or `null` = **BGM/auto note** (plays, not hit). `≥1` = playable; mapped per `mode_hint` (§10). |
| `y` | unsigned long | yes | — | pulse when the note fires |
| `l` | unsigned long | no | 0 | `0` = normal (short) note; `> 0` = **long note**, length in pulses (LN ends at `y + l`) |
| `c` | boolean | no | false | **continuation**: `true` = do **not** restart the sample (continue the slice already playing); `false` = (re)start the sample from its head at this note |

The `c` flag enables "sound slicing": one audio file (`name`) is cut into pieces by successive notes; `c:true` means this note is the seam of an already-playing slice and the audio should keep flowing rather than retrigger.

the reference implementation's `Note` also carries internal fields `t` (type) and `up` used during LN reconstruction — not part of the JSON you parse; ignore on input.

**Long-note duration handling (the reference implementation):** the audible duration of an LN's keysound is bounded either by `l` (to `y+l`) or by the next note in the same channel, whichever the engine computes; the playable end timeline is at `y + l`.

## 8. MineChannel / MineNote (the reference implementation ext) and key_channels

```
MineChannel { name: string, notes: MineNote[] }
MineNote { x, y, damage }
```

| Field | Type | Notes |
|---|---|---|
| `name` | string | keysound file for the mine |
| `x` | int | lane (same mapping as Note; `0`/null = non-playable) |
| `y` | unsigned long | pulse |
| `damage` | number (double) | life damage dealt on hit |

`key_channels` uses the **same `MineChannel`/`MineNote` shape** in jbms-parser, but represents **invisible notes** that only trigger a keysound (no judgement, no damage). When parsing for the reference implementation compatibility, model both arrays with one struct and distinguish by which array they came from. Neither is in the official 1.0.0 spec — emit them only as an extension.

## 9. BGA

```
BGA {
  bga_header:  BGAHeader[]   // { id: int, name: string }
  bga_events:  BGAEvent[]    // base layer
  layer_events:BGAEvent[]    // overlay
  poor_events: BGAEvent[]    // shown on miss
}
BGAEvent (a.k.a. BNote) { y: unsigned long, id: int, ... }
```

- `BGAHeader`: `id` (picture handle) + `name` (image/video path). PNG for stills, WebM for video; recommended/typical frame 1280×720.
- Each `*_events` entry has `y` (pulse to show) + `id` (references a `BGAHeader.id`).
- **the reference implementation extensions on the event (`BNote`)**: `id_set: int[]`, `condition: string`, `interval: int`, plus a `bga_sequence: BGASequence[]` (animation sequences, `BGASequence { id, sequence: Sequence[] }`). These are not in official 1.0.0; gate behind the extension flag. (Known the reference implementation quirk: `layer_events` historically had playback bugs — issue #616.)

## 10. mode_hint → lane (x) mapping

`x ≥ 1` is the 1-based lane index; `x = 0`/`null` = BGM. Official spec lane tables:

| mode_hint | playable x values → lane | scratch (x) |
|---|---|---|
| `beat-5k` | 1–5 | **8** = SC |
| `beat-7k` (default) | 1–7 | **8** = SC |
| `beat-10k` | P1: 1–5, P2: 9–13 | P1 **8**, P2 **16** |
| `beat-14k` | P1: 1–7, P2: 9–15 | P1 **8**, P2 **16** |
| `popn-5k` | 1–5 | none |
| `popn-9k` | 1–9 | none |

Note the **gap**: scratch lives at `x=8` (and `x=16` for P2), so `x=6,7` are unused in 5K and `x=14,15` unused in 10K. This is exactly how the reference implementation builds its `keyassign` arrays (`-1` = unmapped lane):

```
BEAT_5K  : keyassign = [0,1,2,3,4,-1,-1,5]                       // x1..x5 → 0..4, x8 → 5(SC)
BEAT_10K : keyassign = [0,1,2,3,4,-1,-1,5, 6,7,8,9,10,-1,-1,11]  // P2 x9..x13 → 6..10, x16 → 11(SC)
default  : keyassign[i] = i                                       // beat-7k, beat-14k, popn-*: x_n → lane n-1
```

In code: `lane = (x>0 && x<=keyassign.length) ? keyassign[x-1] : -1` (i.e. convert the 1-based `x` to 0-based, look it up; `-1` = drop/auto). For the `default` branch the reference implementation just maps `x_n → n-1` over the mode's `key` count, so beat-7k/14k/popn are identity-mapped with `x=8`/`x=16` naturally landing on the scratch lane index. Custom/extension modes should use a **distinct `mode_hint`** so players can route or reject them.

## 11. Minimal Rust struct sketch

```rust
#[derive(Deserialize)]
struct Bmson {
    version: Option<String>,                 // absent => 0.21-era
    info: Info,
    #[serde(default)] lines: Vec<BarLine>,
    #[serde(default)] bpm_events: Vec<BpmEvent>,
    #[serde(default)] stop_events: Vec<StopEvent>,
    #[serde(default)] scroll_events: Vec<ScrollEvent>,   // ext
    #[serde(default)] sound_channels: Vec<SoundChannel>,
    #[serde(default)] mine_channels: Vec<MineChannel>,    // ext
    #[serde(default)] key_channels: Vec<MineChannel>,     // ext (invisible)
    #[serde(default)] bga: Bga,
}
#[derive(Deserialize)]
struct Info {
    #[serde(default)] title: String, #[serde(default)] subtitle: String,
    #[serde(default)] artist: String, #[serde(default)] subartists: Vec<String>,
    #[serde(default)] genre: String,
    #[serde(default = "default_mode")] mode_hint: String,   // "beat-7k"
    #[serde(default)] chart_name: String,
    #[serde(default)] level: u64,
    init_bpm: f64,
    #[serde(default = "h100")] judge_rank: f64,
    #[serde(default = "h100")] total: f64,
    #[serde(default)] back_image: Option<String>,
    #[serde(default)] eyecatch_image: Option<String>,
    #[serde(default)] banner_image: Option<String>,
    #[serde(default)] preview_music: Option<String>,
    #[serde(default = "r240")] resolution: u64,             // pulses per quarter note
    #[serde(default)] ln_type: i32,                          // ext
}
struct BarLine { y: u64, #[serde(default)] k: i32 }
struct BpmEvent  { y: u64, bpm: f64 }
struct StopEvent { y: u64, duration: u64 }
struct ScrollEvent { y: u64, #[serde(default = "one")] rate: f64 }   // ext
struct SoundChannel { name: String, #[serde(default)] notes: Vec<Note> }
struct Note { x: Option<i32>, y: u64, #[serde(default)] l: u64, #[serde(default)] c: bool }
struct MineChannel { name: String, #[serde(default)] notes: Vec<MineNote> }   // ext (also key_channels)
struct MineNote { x: Option<i32>, y: u64, #[serde(default)] damage: f64 }
struct Bga { bga_header: Vec<BgaHeader>, bga_events: Vec<BgaEvent>, layer_events: Vec<BgaEvent>, poor_events: Vec<BgaEvent> }
struct BgaHeader { id: i32, name: String }
struct BgaEvent  { y: u64, id: i32 }
```

Use `pulses_per_quarter = resolution`; one 4/4 measure = `4*resolution` pulses; integrate time piecewise across `bpm_events` and add `stop_events`. For the reference implementation byte-compat, multiply `resolution` by 4 and use the microsecond formulas in §2.

## Sources

- [bmson format specification — bmson-spec 1.0.0 (Read the Docs, rendered)](https://bmson-spec.readthedocs.io/en/master/doc/)
- [bmson-spec/doc/index.rst (canonical source, GitHub raw)](https://raw.githubusercontent.com/bemusic/bmson-spec/master/doc/index.rst)
- [bemusic/bmson-spec repository](https://github.com/bemusic/bmson-spec)
- [How to play bmson — bmson-spec Wiki](https://github.com/bemusic/bmson-spec/wiki/How-to-play-bmson)
- [bmson-spec-fork 2.0.0-rc1 (later/forked spec, for contrast)](https://bmson-spec-fork.readthedocs.io/en/latest/doc/index.html)
- 레퍼런스 구현/jbms-parser implementation (extension fields + exact timing/lane logic): `exch-bms2/jbms-parser` — `src/bms/model/bmson/Bmson.java`, `BMSInfo.java`, `Note.java`, `MineChannel.java`, `MineNote.java`, `ScrollEvent.java`, `BMSONObject.java`, `BGA.java`, and `src/bms/model/BMSONDecoder.java` (keyassign arrays + `resolution*4`, time/stop formulas).
- [exch-bms2/the reference implementation repository](https://github.com/exch-bms2/the reference implementation)

## Rust 스택

### audio

I have all the information needed. One important nuance to confirm: in cpal 0.16+, `SampleRate` became a tuple struct (`SampleRate(pub u32)`) accessed via `.0`, and `device.supported_output_configs()` filtering changed. The fetched docs confirm cpal 0.17.3 with `sample_rate: SampleRate`. I have enough to write the report accurately. Let me compile the findings.

---

# Rust low-latency audio engine for a BMS keysound mixer — current best practices & versions (May 2026)

## TL;DR crate versions (all current on crates.io as of May 2026)

| Crate | Version | Role |
|---|---|---|
| `cpal` | **0.17.3** | Cross-platform output stream (macOS CoreAudio, Windows WASAPI). |
| `symphonia` | **0.5.5** | Decode WAV/OGG-Vorbis/FLAC/MP3 → interleaved `f32`/`i16` PCM. |
| `rtrb` | **0.3.3** | Wait-free SPSC ring buffer — game-thread → audio-callback command channel. |
| `triple_buffer` | **8.x** (latest) | SPSC "latest snapshot" sharing (e.g. master-clock readback, mixer params). |
| `rubato` | **3.0.0** (2026-05-20) | Sample-rate conversion / async sinc resampling. Pre-resample at load. |
| `signalsmith-stretch` (or `ssstretch`) | latest | Polyphonic pitch-shift / time-stretch when you need true pitch independent of rate. |
| `basedrop` | latest | RT-safe deferred drop (`Shared`/`Owned`/`Collector`) — free decoded buffers off the audio thread. |
| `fundsp` | 0.23.0 | Optional, only if you want a DSP-graph effects bus. Not needed for a bare mixer. |

> ⚠️ **cpal API note (0.16 → 0.17):** `SampleRate` is now a **tuple struct** `SampleRate(pub u32)` — read it as `config.sample_rate.0`. Several older tutorials use `config.sample_rate.0 as f32` against an older alias; the field type is `SampleRate` in 0.17.3. `StreamConfig { channels: ChannelCount, sample_rate: SampleRate, buffer_size: BufferSize }`.

---

## 1. cpal — opening the output stream, callback model, RT-safe pattern

### Callback model
`device.build_output_stream::<T, _, _>(config, data_cb, err_cb, timeout)` returns a `Stream`. cpal calls `data_cb(&mut [T], &OutputCallbackInfo)` repeatedly from a **high-priority OS audio thread** (CoreAudio render thread / WASAPI). The buffer is **interleaved** by channel: iterate `data.chunks_mut(channels)` to get per-frame slices. The stream does **not** auto-start on all platforms — you must call `stream.play()`. (Source: cpal docs, `DeviceTrait::build_output_stream`.)

### Choosing device / format / buffer size

```rust
use cpal::{traits::{DeviceTrait, HostTrait, StreamTrait}, BufferSize, SampleFormat, StreamConfig, SupportedBufferSize};

let host = cpal::default_host();                       // CoreAudio on macOS, WASAPI on Windows
let device = host.default_output_device().expect("no output device");
let supported = device.default_output_config().expect("no default config");

let sample_format = supported.sample_format();         // often F32 on both CoreAudio & WASAPI shared
let mut config: StreamConfig = supported.config();

// Low-latency fixed buffer where the backend allows it.
// CoreAudio honors Fixed well; WASAPI *shared* mode often ignores Fixed (use Default + small period).
if let SupportedBufferSize::Range { min, max } = supported.buffer_size() {
    let target = 256u32.clamp(*min, *max);             // 256 frames @48k ≈ 5.3 ms
    config.buffer_size = BufferSize::Fixed(target);
} else {
    config.buffer_size = BufferSize::Default;
}

let channels = config.channels as usize;
let sample_rate = config.sample_rate.0;                // u32 — your master clock rate
```

**Latency guidance:**
- **macOS CoreAudio:** `BufferSize::Fixed` is respected; 128–256 frames is realistic and stable. Round-trip is dominated by buffer size.
- **Windows WASAPI:** In **shared mode**, cpal's WASAPI backend typically does *not* honor `Fixed` and runs at the engine period (~10 ms / 480 frames @48k). For sub-5 ms you need **exclusive mode** (cpal exposes WASAPI exclusive via host config) — but exclusive seizes the device and disables the system mixer, usually undesirable for a game. **Recommended:** ship `BufferSize::Default` on Windows shared mode and accept ~10 ms; offer an opt-in exclusive low-latency mode. Always set your master clock from samples actually rendered, not from wall-clock.
- Prefer the device's native rate (44.1k or 48k). Don't force a rate the device must resample — resample your *samples* instead (§4).

### The RT-safe pattern (no alloc, no lock, no syscall in the callback)
The audio callback must never `malloc`, lock a `Mutex`, do file I/O, or `println!`. Everything it touches is pre-allocated. Communication is via lock-free SPSC primitives:

- **Game thread → callback (events):** `rtrb` (wait-free SPSC ring) carrying small POD commands (`PlaySample { sample_id, gain, pan, pitch }`, `StopChannel`, `SetMasterGain`).
- **Callback → game thread (clock/telemetry):** `triple_buffer` or an `AtomicU64` for `samples_played`.
- **Decoded PCM ownership:** keep an immutable, pre-loaded `Arc<SampleData>` table. To free without blocking the RT thread, hand buffers to `basedrop`'s `Collector` (deferred drop) rather than dropping `Arc` on the audio thread.

```rust
use std::sync::Arc;

// Pre-built, immutable. Cloned cheaply (Arc bump) when a voice grabs it.
struct SampleData { pcm: Arc<[f32]>, channels: u16, frames: usize } // interleaved, already at device rate

enum AudioCmd {                 // POD only — no heap inside
    Play { sample: u32, gain: f32, pan: f32 },
    Stop { sample: u32 },
    SetMasterGain(f32),
}
```

---

## 2. The command channel (rtrb 0.3.3)

`rtrb` allocates its fixed capacity once at construction; `push`/`pop` are wait-free and return immediately. Producer = game/chart-scheduler thread, consumer = audio callback.

```rust
use rtrb::{RingBuffer, Producer, Consumer};

let (mut tx, mut rx): (Producer<AudioCmd>, Consumer<AudioCmd>) =
    RingBuffer::new(4096).split();   // capacity ≫ worst-case burst of simultaneous notes

// game thread (scheduler): non-blocking
let _ = tx.push(AudioCmd::Play { sample: 42, gain: 0.8, pan: 0.0 });

// inside the audio callback: drain everything, allocate nothing
while let Ok(cmd) = rx.pop() {
    match cmd { /* spawn voice / stop / set gain */ }
}
```

> For BMS specifically: a single key can retrigger the same keysound rapidly (hundreds of overlapping voices). Size the ring (4096+) and the voice pool generously, and **drain the whole ring at the top of every callback** before mixing.

**Master clock readback** (callback → game) — use an atomic, it's RT-safe and lock-free:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
static SAMPLES_PLAYED: AtomicU64 = AtomicU64::new(0);
// end of callback:
SAMPLES_PLAYED.fetch_add(frames_rendered as u64, Ordering::Release);
// game thread: song_time_seconds = SAMPLES_PLAYED.load(Acquire) as f64 / sample_rate as f64
```

`triple_buffer` is the better choice when you need to publish a *struct* of telemetry (current voice count, peak meter, clock) atomically without an atomic-per-field.

---

## 3. symphonia 0.5.5 — decode to interleaved PCM (load time, off the RT thread)

Decode each keysound **once at load** into an interleaved `Vec<f32>` (or `i16`), resample to the device rate, then freeze into `Arc<[f32]>`. The decode loop (probe → format reader → make decoder → packet loop) is exactly the canonical Symphonia flow:

```rust
use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::{probe::Hint, FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

fn decode_to_interleaved_f32(path: &std::path::Path) -> (Vec<f32>, u32, u16) {
    let src = std::fs::File::open(path).expect("open");
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) { hint.with_extension(ext); }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .expect("unsupported format");                 // auto-detects wav/ogg/flac/mp3

    let track = format.default_track(TrackType::Audio).expect("audio track");
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(
            track.codec_params.as_ref().unwrap().audio().unwrap(),
            &AudioDecoderOptions::default(),
        )
        .expect("unsupported codec");

    let mut out: Vec<f32> = Vec::new();
    let mut rate = 0u32;
    let mut channels = 0u16;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(Error::IoError(_)) => break,
            Err(e) => panic!("{e}"),
        };
        if packet.track_id() != track_id { continue; }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = decoded.spec();
                rate = spec.rate();
                channels = spec.channels().count() as u16;
                // 0.5.x convenience: copy any sample format → interleaved f32, appended
                decoded.copy_to_vec_interleaved(&mut out);
            }
            Err(Error::DecodeError(_)) | Err(Error::IoError(_)) => continue,
            Err(e) => panic!("{e}"),
        }
    }
    (out, rate, channels)
}
```

Key symphonia 0.5.5 facts:
- `symphonia::default::get_probe()` auto-detects the container; `get_codecs().make_audio_decoder(...)` builds the decoder.
- `GenericAudioBufferRef::copy_to_vec_interleaved(&mut Vec<f32>)` is the one-call path to interleaved f32 regardless of the codec's native format (handles i16/i24/f32/etc). Use `copy_to_vec_interleaved::<i16>` style if you want i16 output.
- MP3 / Vorbis / FLAC / WAV / AAC are enabled via default features; MP3 is non-experimental in 0.5.x. Enable only what you ship to cut binary size: e.g. `features = ["mp3", "ogg", "vorbis", "flac", "wav", "pcm", "isomp4"]`.

---

## 4. Resampling & pitch (rubato 3.0.0 / signalsmith-stretch)

Two distinct needs — keep them separate:

**(a) Rate-match at load (mandatory, do once, off RT thread).** Every keysound must be stored at the **device sample rate** so the mixer never resamples per-frame. Use `rubato` 3.0.0 — its `SincFixedIn` / FFT resamplers are allocation-free *during* `process` (allocate up front), and the crate is explicitly designed for RT safety. For offline batch resampling at load you don't even need the RT guarantees, but reuse the same code path.

```rust
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

let params = SincInterpolationParameters {
    sinc_len: 256, f_cutoff: 0.95, oversampling_factor: 256,
    interpolation: SincInterpolationType::Linear, window: WindowFunction::BlackmanHarris2,
};
let mut rs = SincFixedIn::<f32>::new(
    device_rate as f64 / src_rate as f64, // resample ratio
    2.0, params, chunk_frames, num_channels,
).unwrap();
// feed planar (per-channel Vec<f32>) chunks: rs.process(&input_planar, None)
```

> rubato consumes/produces **planar (non-interleaved)** channel vectors — de-interleave before, re-interleave after for your mixer's interleaved store.

**(b) Per-note pitch / pitch-bend (optional).** BMS rarely needs pitch shifting; when it does (e.g. `#WAV` pitched variations), two options:
- **Cheap:** per-voice playback-rate change = read the source cursor with a fractional stride and linearly/cubically interpolate in the voice. This changes pitch *and* duration (classic sampler behavior) — usually exactly what a keysound wants. No extra crate, fully RT-safe.
- **Pitch-independent of duration:** `signalsmith-stretch` (or the `ssstretch` Rust binding) for polyphonic pitch shift / time-stretch — heavier, prefer to pre-render offline rather than run live per voice.

For a BMS keysound mixer, **(a) load-time resample + (b) cheap fractional-stride pitch in the voice** is the recommended, RT-safe combination. Reserve signalsmith for offline pre-rendering.

---

## 5. Recommended architecture — custom keysound mixer

```
 chart scheduler thread            audio callback (CoreAudio/WASAPI render thread)
 ─────────────────────             ───────────────────────────────────────────────
 read SAMPLES_PLAYED (atomic)      drain rtrb commands  → activate voices in pool
 derive song time                  for each output frame:
 push Play{...} into rtrb            sum active voices (cursor++ with stride+interp)
                                     apply per-voice gain/pan, then master gain
                                    write interleaved into &mut [f32]
                                    SAMPLES_PLAYED += frames  (Release)
 load thread (symphonia+rubato) → builds Arc<[f32]> sample table, swapped in via rtrb/triple_buffer
 basedrop Collector::collect() on game thread frees retired buffers
```

### Voice pool (fixed-size, pre-allocated)

```rust
struct Voice {
    pcm: Arc<[f32]>,   // interleaved, device rate
    src_channels: u16,
    pos: f64,          // fractional read cursor (frames) — supports pitch via stride
    stride: f64,       // 1.0 = normal; 2^(semitones/12) for pitch
    gain: f32,
    pan: f32,          // -1..1
    active: bool,
}

const MAX_VOICES: usize = 512;   // BMS overlapping keysounds need a big pool

struct Mixer {
    voices: [Voice; MAX_VOICES],   // pre-allocated, never grows in callback
    samples: Arc<[Option<Arc<[f32]>>]>, // sample table indexed by id (immutable snapshot)
    master_gain: f32,
    out_channels: usize,
}
```

### Mix routine (the heart of the callback)

```rust
fn render(mixer: &mut Mixer, out: &mut [f32], samples_played: &AtomicU64) {
    let ch = mixer.out_channels;
    for s in out.iter_mut() { *s = 0.0; }                 // clear (no alloc)

    for v in mixer.voices.iter_mut().filter(|v| v.active) {
        let frames = v.pcm.len() / v.src_channels as usize;
        for frame in out.chunks_mut(ch) {
            let i = v.pos.floor() as usize;
            if i + 1 >= frames { v.active = false; break; } // voice done → reusable slot

            let frac = (v.pos - i as f64) as f32;
            // linear interpolation on the source frame (mono example; stereo: per-channel)
            let base = i * v.src_channels as usize;
            let s0 = v.pcm[base];
            let s1 = v.pcm[base + v.src_channels as usize];
            let sample = (s0 + (s1 - s0) * frac) * v.gain;

            // equal-power-ish pan into stereo out
            let l = sample * (0.5 - 0.5 * v.pan).sqrt();
            let r = sample * (0.5 + 0.5 * v.pan).sqrt();
            if ch >= 2 { frame[0] += l; frame[1] += r; }
            else { frame[0] += sample; }

            v.pos += v.stride;                            // pitch = stride
        }
    }

    let mg = mixer.master_gain;
    for s in out.iter_mut() { *s *= mg; }                 // master, then clamp/limit if desired
    samples_played.fetch_add((out.len() / ch) as u64, std::sync::atomic::Ordering::Release);
}
```

### Sample-accurate master clock & scheduling
- The **only** source of truth for song time is `SAMPLES_PLAYED / sample_rate`. Never use `Instant::now()` for note timing — it drifts against the audio device.
- For *sub-buffer* (truly sample-accurate) note onsets, carry an `at_sample: u64` (absolute device-sample timestamp) in `AudioCmd::Play`. In the callback, compute this buffer's start sample = `SAMPLES_PLAYED` snapshot; if a queued note's `at_sample` falls inside `[start, start+frames)`, set the new voice's initial `pos` so its first sample lands on the exact sub-buffer offset. This gives you sample-accurate triggering even though the callback runs in blocks of 128–480 frames.
- Apply a tiny attack/release ramp (a few samples) per voice to avoid clicks when retriggering the same keysound — important for BMS where the same `#WAV` is spammed.

### Memory safety on the RT thread
- Sample table is an **immutable snapshot** (`Arc`). To add/replace samples mid-game, build a new snapshot on the load thread and publish the pointer through `triple_buffer` (or swap an `Arc` via an `rtrb` "here's the new table" command). The callback only ever reads.
- Drop retired `Arc<[f32]>` via **`basedrop::Collector`** so the actual `free` happens on the game thread, not the audio thread.

---

## Practical defaults for a BMS engine

- **Output:** `f32`, native device rate (44.1k or 48k), stereo. macOS: `BufferSize::Fixed(256)`. Windows shared WASAPI: `BufferSize::Default` (≈10 ms), exclusive mode as opt-in for low latency.
- **All keysounds pre-decoded (symphonia 0.5.5) + pre-resampled (rubato 3.0.0) to device rate, stored interleaved `Arc<[f32]>`.**
- **Game→callback:** `rtrb` 0.3.3 ring of POD commands. **Callback→game:** `AtomicU64` samples-played (+ `triple_buffer` for richer telemetry).
- **Voice pool** ≥512, pitch via fractional stride + linear interp, no allocation/locking in the callback. Free via `basedrop`.
- **Clock** derived solely from samples rendered → sample-accurate, drift-free timing.

---

## Sources

- [cpal — crates.io (v0.17.3)](https://crates.io/crates/cpal) · [cpal 0.17.3 docs.rs](https://docs.rs/crate/cpal/latest) · [cpal StreamConfig docs](https://docs.rs/cpal/latest/cpal/struct.StreamConfig.html)
- [symphonia — crates.io (v0.5.5)](https://crates.io/crates/symphonia) · [symphonia 0.5.5 docs.rs](https://docs.rs/crate/symphonia/latest) · [symphonia GETTING_STARTED.md (decode loop, copy_to_vec_interleaved)](https://github.com/pdeljanov/symphonia/blob/main/GETTING_STARTED.md)
- [rubato — crates.io (v3.0.0)](https://crates.io/crates/rubato) · [rubato Resampler trait docs](https://docs.rs/rubato/latest/rubato/trait.Resampler.html) · [HEnquist/rubato GitHub](https://github.com/HEnquist/rubato)
- [rtrb — crates.io (v0.3.3)](https://crates.io/crates/rtrb) · [mgeier/rtrb GitHub (wait-free SPSC)](https://github.com/mgeier/rtrb)
- [triple_buffer — crates.io](https://crates.io/crates/triple_buffer) · [triple_buffer docs.rs](https://docs.rs/triple_buffer/)
- [signalsmith-stretch — lib.rs](https://lib.rs/crates/signalsmith-stretch) · [ssstretch Rust bindings (docs.rs)](https://docs.rs/ssstretch/latest/ssstretch/)
- [basedrop docs.rs (RT-safe deferred drop)](https://docs.rs/basedrop/latest/basedrop/) · [Basedrop: A garbage collector for real-time audio in Rust](https://micahrj.github.io/posts/basedrop/)
- [fundsp — crates.io (v0.23.0)](https://crates.io/crates/fundsp)

### graphics

That's fine — the relevant point (macroquad has no built-in FPS cap; you control vsync via `platform.swap_interval` and otherwise sleep/use delta-time manually) is already established from the earlier search and the swap_interval issue. I have everything needed. Compiling the final report.

---

# 2D Graphics + Windowing in Rust for a Rhythm Game — Stack Research (current as of May 2026)

## TL;DR recommendation

**Build on `wgpu` + `winit` directly.** A rhythm game's defining requirement is *deterministic timing under an audio clock with low, consistent input-to-photon latency* — exactly the area where macroquad's convenience abstractions get in your way. macroquad is the right call **only** for a throwaway prototype to validate gameplay feel in a weekend. Use it as a scaffolding sandbox, not the engine you ship.

The whole modern stack is now version-aligned on the **wgpu 29.x** major, which removes the biggest historical friction (text renderer lagging wgpu's breaking releases).

---

## 1. Confirmed current crate versions (crates.io, May 2026)

| Crate | Latest stable | Released | Notes |
|---|---|---|---|
| **wgpu** | **29.0.3** | 2026-05-02 | wgpu uses a single-integer major now (26→27→28→29). |
| **winit** | **0.30.13** | 2026-03-02 | 0.31 is still **beta** (`0.31.0-beta.2`, Nov 2025) — stay on 0.30.x. `ApplicationHandler` is the current non-deprecated API. |
| **glyphon** | **0.11.0** | 2026-04-13 | Depends on **`wgpu ^29`**, `cosmic-text ^0.18`, `etagere ^0.3`. |
| **wgpu_text** | **29.0.3** | 2026-05-16 | Version-matched to `wgpu ^29`. Simpler/bitmap-font oriented. |
| **image** | **0.25.10** | 2026-03-10 | PNG/JPEG/etc decoding for textures. |
| **macroquad** | **0.4.15** | 2026-05-20 | Self-contained (own `miniquad` GL/Metal backend — does *not* use wgpu/winit). |

Key compatibility fact to design around: **glyphon 0.11 and wgpu_text 29.0.3 both target `wgpu ^29`**, so if you pin `wgpu = "29"` everything lines up. Historically the text renderer was the crate that pinned you to an old wgpu — that's resolved right now, so 29.x is the version to standardize on. ([glyphon deps](https://crates.io/api/v1/crates/glyphon/dependencies), [wgpu_text](https://lib.rs/crates/wgpu_text))

```toml
[dependencies]
wgpu    = "29"
winit   = "0.30"
glyphon = "0.11"     # or: wgpu_text = "29"
image   = { version = "0.25", default-features = false, features = ["png"] }
pollster = "0.4"     # block_on for async wgpu init inside resumed()
bytemuck = { version = "1", features = ["derive"] }  # POD vertex/instance structs
```

---

## 2. Stack (1): wgpu + winit

### 2.1 winit 0.30 `ApplicationHandler` — the current pattern + the migration gotcha

winit 0.30 replaced the old closure-based `event_loop.run(|event, elwt| …)` with the **`ApplicationHandler` trait**. You implement a struct and the loop calls back into trait methods. ([ApplicationHandler docs](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html))

Methods you care about:
- **`resumed()`** — the *only* correct place to create the window and graphics context. (Mandatory on Android; treat as mandatory everywhere.)
- **`window_event()`** — receives `WindowEvent`, including **`RedrawRequested`** (where you render) and input.
- **`new_events()`** — "before processing events"; the doc-recommended spot to update frame timing.
- **`about_to_wait()`** — explicitly documented as **not** the place to drive rendering. Use it only to call `window.request_redraw()` when you want a new frame in `Poll` mode.

**The #1 0.30 migration gotcha — the self-referential `Window`/`Surface` lifetime.** winit 0.30 forces window creation *inside the event loop* (in `resumed`), but wgpu's `Surface` borrows the window. The clean, community-blessed fix is to wrap the window in **`Arc<Window>`** and hand the surface a clone, giving the surface a `'static` lifetime — instead of fighting borrow lifetimes. The secondary gotcha: wgpu init is `async` but `resumed` is sync, so wrap it with **`pollster::block_on`** (winit "was never designed to be async and won't be"). Both are confirmed across the wgpu/winit community threads and the updated *Learn Wgpu* tutorial. ([gfx-rs discussion #6005](https://github.com/gfx-rs/wgpu/discussions/6005), [winit discussion #3667](https://github.com/rust-windowing/winit/discussions/3667), [Learn Wgpu tutorial1](https://sotrh.github.io/learn-wgpu/beginner/tutorial1-window/))

Minimal shape:

```rust
use std::sync::Arc;
use winit::{application::ApplicationHandler, event::WindowEvent,
            event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
            window::{Window, WindowId}};

struct App { state: Option<State> }      // State holds the renderer

struct State {
    window: Arc<Window>,                 // Arc -> gives Surface a 'static lifetime
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let window = Arc::new(el.create_window(Window::default_attributes()).unwrap());
        self.state = Some(pollster::block_on(State::new(window)));  // async init, blocked
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, ev: WindowEvent) {
        let Some(state) = self.state.as_mut() else { return };
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::RedrawRequested => {
                state.render();
                state.window.request_redraw();   // keep the frames coming
            }
            _ => {}
        }
    }
}

fn main() {
    let el = EventLoop::new().unwrap();
    el.set_control_flow(ControlFlow::Poll);     // continuous render; NOT Wait
    el.run_app(&mut App { state: None }).unwrap();
}
```

> **`ControlFlow::Poll` vs `Wait`:** a rhythm game renders every frame, so use **`Poll`** (loop never blocks) and drive frames via `request_redraw()` in `RedrawRequested`. `Wait`/`WaitUntil` is for event-driven UIs that idle.

### 2.2 wgpu Surface/Device/Queue init

Inside `State::new(window)`:
```rust
let instance = wgpu::Instance::default();
let surface  = instance.create_surface(window.clone())?;             // Arc clone -> 'static
let adapter  = instance.request_adapter(&wgpu::RequestAdapterOptions {
    compatible_surface: Some(&surface), ..Default::default() }).await?;
let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor::default()).await?;

let caps = surface.get_capabilities(&adapter);
let config = wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: caps.formats[0],
    width: size.width, height: size.height,
    present_mode: wgpu::PresentMode::Fifo,                            // see §2.4
    alpha_mode: caps.alpha_modes[0],
    view_formats: vec![], desired_maximum_frame_latency: 2,
};
surface.configure(&device, &config);
```

### 2.3 Sprite batch renderer (instanced quads + texture atlas) — the right architecture for a rhythm game

A rhythm game draws thousands of near-identical sprites (notes, receptors, hold bodies) from **one atlas**. The correct pattern is **instanced rendering**:

- **One static unit-quad vertex buffer** (4 verts, or 6 for two triangles) shared by all sprites.
- **One per-frame instance buffer**: each instance = `{ pos: vec2, size: vec2, uv_rect: vec4, color: vec4 }` (a `#[repr(C)] #[derive(bytemuck::Pod)]` struct). You rewrite this buffer each frame (`queue.write_buffer`) and issue **a single `draw_indexed(0..6, 0, 0..N)`**.
- **One `bind_group`** holding the atlas `Texture` + `Sampler` + an orthographic projection uniform.
- The vertex shader reads `@builtin(instance_index)`, applies the instance transform, and samples the atlas sub-rect.

This collapses the entire note field into ~1 draw call → trivial CPU cost, leaving headroom for the timing work that actually matters. (This is the standard wgpu 2D-batching approach; the *Learn Wgpu* "instancing" chapter covers the buffer/shader wiring. [Learn Wgpu](https://sotrh.github.io/learn-wgpu/))

### 2.4 Present modes + frame pacing (the rhythm-game-critical part)

From the official [`PresentMode` docs](https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html):

| Mode | Behavior | Tearing | Latency | Support |
|---|---|---|---|---|
| **Fifo** ("Vsync On") | ~3-frame queue, 1/vblank; blocks `get_current_texture()` | No | Higher (queue depth) | **All platforms (guaranteed)** |
| **FifoRelaxed** ("Adaptive") | Fifo, but late frame shows immediately | Possible if late | Medium | AMD/Vulkan only |
| **Mailbox** ("Fast Vsync") | 1-deep queue, newest frame *replaces* old | No | **Low** | DX12(Win10), NVIDIA Vulkan, Wayland Vulkan |
| **Immediate** ("Vsync Off") | No queue, swap instantly | **Yes** | **Lowest** | Not on old DX12 / Wayland |
| **AutoVsync** | falls back FifoRelaxed→Fifo | — | — | universal |
| **AutoNoVsync** | falls back Immediate→Mailbox→Fifo | — | — | universal |

**Recommendation for a rhythm game:**
- **Always query `surface.get_capabilities(&adapter).present_modes`** and pick from what's actually supported — don't hardcode `Mailbox`/`Immediate`.
- **Preferred:** `Mailbox` if present (low latency, *no tearing*) → else `Fifo` as the guaranteed-safe baseline.
- **Offer the player an "input latency" toggle** exposing `Immediate` (lowest input-to-photon, tearing acceptable to many rhythm players) vs `Fifo` (clean). This is a real, common rhythm-game setting.
- Set **`desired_maximum_frame_latency: 1`** (instead of the default 2) to cut a frame of queued latency — directly relevant to input-to-photon.

**Audio clock vs frame clock — the single most important design rule:** Do **not** advance gameplay by frame `dt`. The **audio playback position is the master clock**; the frame clock only decides *when you draw a snapshot of it*. Each frame: read the audio backend's current playback sample/time (e.g. from `cpal`/`kira`/`rodio`), convert to song time, and compute note positions from *that*. Frame jitter then only affects visual smoothness, never judgment timing. Maintain a small smoothing/extrapolation on the audio clock (it updates in callback chunks) so on-screen scroll stays smooth between audio updates, but **judge hits against the raw audio clock + measured input timestamp**, never against accumulated frame time. This separation is what makes the game feel "tight," and it's the architectural reason to own your loop rather than inherit macroquad's.

### 2.5 Text (scores, combo, judgment, menus)
- **`glyphon` 0.11** is the recommended choice: built on `cosmic-text` (full shaping/layout, Unicode, fallback) + `etagere` glyph atlas, integrates as middleware **inside your existing render pass** (no extra passes). Targets `wgpu ^29`. ([glyphon](https://github.com/grovesNL/glyphon))
- **`wgpu_text` 29.0.3** is the lighter alternative — simpler API, good for bitmap/`ab_glyph` fonts and basic HUD text; version-matched to wgpu 29. Use it if you don't need cosmic-text's shaping. ([wgpu_text](https://github.com/Blatko1/wgpu-text))
- Avoid `wgpu_glyph` (older, glyph_brush-based, lagging).

### 2.6 PNG loading
`image` 0.25.10: decode atlas PNG → upload to a `wgpu::Texture`.
```rust
let img = image::load_from_memory(bytes)?.to_rgba8();      // image 0.25
let (w, h) = img.dimensions();
queue.write_texture(/* dst */, &img, /* layout w*4 */, wgpu::Extent3d{width:w,height:h,depth_or_array_layers:1});
```
Use `default-features = false, features = ["png"]` to keep build lean.

---

## 3. Stack (2): macroquad 0.4.15

### What you get for free
One crate (`macroquad = "0.4"`), no winit/wgpu wiring. Self-contained on its own **miniquad** backend (GL/Metal/WebGL — *not* wgpu). Provides: window/loop (`#[macroquad::main]` + `Conf`), 2D draw (`draw_texture_ex`, shapes), `Texture2D` (loads PNG itself), input (`is_key_down`, mouse, touch), built-in **text** (`draw_text`/`draw_text_ex` + `load_ttf_font`), immediate-mode UI, audio, and **custom materials/shaders** (`load_material`, `gl_use_material`) for effects. ([macroquad.rs](https://macroquad.rs/), [docs.rs/macroquad](https://docs.rs/macroquad/latest/macroquad/)). `Conf` exposes `high_dpi`, `sample_count` (MSAA), `default_filter_mode`, and a `platform` escape hatch. ([Conf](https://docs.rs/macroquad/latest/macroquad/window/struct.Conf.html))

### Frame/present model and its **limitations for a rhythm game**
- **No built-in FPS cap and no first-class present-mode control.** macroquad "draws frames as quickly as possible," which is why every macroquad tutorial hammers `get_frame_time()` delta-time movement. ([agical book](https://mq.agical.se/ch3-smooth-movement.html)) FPS limiting is a long-standing open pain point ([#749](https://github.com/not-fl3/macroquad/issues/749), [#170](https://github.com/not-fl3/macroquad/issues/170)).
- **Vsync control is indirect** — only via the underlying miniquad `platform.swap_interval` (`0` = vsync off), not a clean per-mode API like wgpu's `PresentMode`. ([swap_interval #52](https://github.com/not-fl3/macroquad/issues/52))
- **You don't own the loop body's pacing.** Limiting/pacing means manual `std::thread::sleep` + spin, which fights the framework rather than configuring it. There's no `Mailbox`/`desired_maximum_frame_latency` knob, so you can't tune the input-to-photon queue depth.
- **Timing is frame-clock-centric by default.** `macroquad::time` gives `get_frame_time`/`get_fps` ([time docs](https://docs.rs/macroquad/latest/macroquad/time/index.html)) — fine for visuals, but you must still drive judgment off the audio clock yourself, and macroquad gives you less control over how render timing relates to that clock.
- **Custom render control is shallow.** Materials/shaders exist, but you don't get raw command-encoder / render-pass / instance-buffer control. A true single-draw-call instanced note batch is awkward; you lean on `draw_texture_ex` per sprite.

These aren't bugs — macroquad optimizes for *speed-to-prototype*, which is the opposite axis from *frame-pacing precision*.

---

## 4. Recommendation matrix

| Axis | wgpu + winit | macroquad |
|---|---|---|
| Time-to-first-pixel | Days (boilerplate: `ApplicationHandler`, Arc-window, surface config) | **Minutes** (`#[macroquad::main]`) |
| Present-mode control (Fifo/Mailbox/Immediate) | **Full**, with capability query | Indirect (`swap_interval` only) |
| Input-to-photon latency tuning | **Full** (`PresentMode` + `desired_maximum_frame_latency`) | Limited |
| Frame-pacing / FPS control | **Full** (own the loop) | Manual sleep; no built-in cap |
| Instanced sprite batch (1 draw call) | **Native** | Awkward (per-sprite draws) |
| Audio-clock-driven design | **You own the loop → clean** | Possible but you fight the loop |
| Text | glyphon 0.11 / wgpu_text 29 | Built-in `draw_text` |
| PNG | `image` 0.25 | Built-in |
| Cross-platform / WASM | Yes (more setup) | Yes (very easy) |
| Maintenance/version churn | Higher (wgpu majors break ~quarterly) | Lower (one crate) |

**Verdict:**
- **Ship on wgpu + winit** — the rhythm-game-critical features (present-mode choice, frame-latency depth, single-draw-call note batch, loop-owned audio-clock judgment) are exactly its strengths and macroquad's weaknesses.
- **Prototype on macroquad** if you want to validate gameplay feel *today*; migrate before timing/latency tuning begins. Note macroquad is *not* a wgpu wrapper (it's miniquad), so the prototype is throwaway, not a foundation.

**Concrete version set to pin:** `wgpu 29.0.3`, `winit 0.30.13`, `glyphon 0.11.0` (or `wgpu_text 29.0.3`), `image 0.25.10`, plus `pollster 0.4`, `bytemuck 1`. macroquad branch: `macroquad 0.4.15` alone.

---

### Sources
- [crates.io: wgpu](https://crates.io/crates/wgpu) · [winit](https://crates.io/crates/winit) · [macroquad](https://crates.io/crates/macroquad) · [glyphon deps](https://crates.io/api/v1/crates/glyphon/dependencies) · [image](https://crates.io/crates/image) · [wgpu_text on lib.rs](https://lib.rs/crates/wgpu_text)
- [winit `ApplicationHandler` docs](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html) · [winit 0.30 + wgpu discussion #3667](https://github.com/rust-windowing/winit/discussions/3667) · [wgpu self-referential Surface discussion #6005](https://github.com/gfx-rs/wgpu/discussions/6005)
- [Learn Wgpu (window/instancing tutorial)](https://sotrh.github.io/learn-wgpu/beginner/tutorial1-window/)
- [wgpu `PresentMode` docs](https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html)
- [glyphon](https://github.com/grovesNL/glyphon) · [wgpu_text](https://github.com/Blatko1/wgpu-text)
- [macroquad.rs](https://macroquad.rs/) · [macroquad docs](https://docs.rs/macroquad/latest/macroquad/) · [Conf](https://docs.rs/macroquad/latest/macroquad/window/struct.Conf.html) · [macroquad::time](https://docs.rs/macroquad/latest/macroquad/time/index.html) · [FPS limit issue #749](https://github.com/not-fl3/macroquad/issues/749) · [#170](https://github.com/not-fl3/macroquad/issues/170) · [swap_interval #52](https://github.com/not-fl3/macroquad/issues/52) · [smooth movement / delta-time](https://mq.agical.se/ch3-smooth-movement.html)
