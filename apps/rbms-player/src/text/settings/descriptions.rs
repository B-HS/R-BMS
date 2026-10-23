//! What each row of the settings screen is for, said on the panel beside the list, by language.
//!
//! The panel has six lines of about thirty full-width characters, so the Korean and the Japanese
//! are written to fit them: they say what the English says, a little more shortly where the English
//! runs on. A row that is another row's neighbour is named by that row's own name in the language
//! of the sentence ([`super::row_label`]), and the terms the glossary keeps (the names of options,
//! gauges, lamps and judgements) are written as they are.

rbms_model::translations! {
    /// The sentences of the settings screen's rows. An entry is named after the row it describes;
    /// the sentence every JUDGE WIDTH row shares is one entry with the tier and the lanes put in.
    pub(crate) enum RowDescriptionText {
        JudgeAlgorithm => (
            "Which note a press is given to when several notes of its lane are in range. COMBO passes over a note that can no longer be GOOD or better, \
             DURATION takes the one nearest in time, LOWEST always takes the earliest, and SCORE passes over a note that can no longer be GREAT or better.",
            "판정 범위에 노트가 여럿일 때 입력을 어느 노트에 줄지 정합니다. COMBO는 GOOD 이상이 불가능한 노트를 건너뛰고, DURATION은 시간상 가장 가까운 노트, \
             LOWEST는 가장 이른 노트를 고르며, SCORE는 GREAT 이상이 불가능한 노트를 건너뜁니다.",
            "同じレーンの判定範囲に複数のノーツがあるとき、入力をどのノーツに割り当てるかを決めます。COMBOはGOOD以上になり得ないノーツを飛ばし、\
             DURATIONは時間的に最も近いノーツ、LOWESTは常に最も早いノーツを選び、SCOREはGREAT以上になり得ないノーツを飛ばします。"
        ),
        JudgeWidth => (
            "Scales the {0} window of the {1} lanes, as a share of the width the chart asks for. 100% is the chart's own. \
             Narrowing is free; over 100% makes a run a custom judge: no best score, the ASSIST lamp at best; the run is still kept, with its replay, and sent.",
            "{1} 레인의 {0} 판정 폭을 차트 기준 비율로 조절합니다. 100%가 차트 기본값이며 좁히는 것은 자유입니다. \
             100%를 넘기면 커스텀 판정이 되어 최고 기록이 남지 않고 램프는 ASSIST가 한계입니다. 플레이는 리플레이와 함께 저장·전송됩니다.",
            "{1}レーンの{0}判定幅を、譜面が求める幅に対する割合で調整します。100%が譜面どおりで、狭めるのは自由です。\
             100%を超えるとカスタム判定となり、ベストスコアは残らずランプはASSISTまでです。プレーはリプレイとともに保存・送信されます。"
        ),
        LanesOfKeys => ("key", "건반", "鍵盤"),
        LanesOfScratch => ("scratch", "스크래치", "スクラッチ"),
        LongNoteMargin => (
            "Scales the window the end of a long note is released in, as a share of the mode's own. 100% is the mode's own. \
             Over 100% makes a run a custom judge: no best score, the ASSIST lamp at best; the run is still kept, with its replay, and sent.",
            "롱노트 끝을 뗄 수 있는 판정 폭을 모드 기본값에 대한 비율로 조절합니다. 100%가 모드 기본값입니다. \
             100%를 넘기면 커스텀 판정이 되어 최고 기록이 남지 않고 램프는 ASSIST가 한계입니다. 플레이는 리플레이와 함께 저장·전송됩니다.",
            "ロングノートの終端を離せる判定幅を、モード標準に対する割合で調整します。100%がモード標準です。\
             100%を超えるとカスタム判定となり、ベストスコアは残らずランプはASSISTまでです。プレーはリプレイとともに保存・送信されます。"
        ),
        LnMode => (
            "What a long note is played as when its chart does not say: LN (long note), CN (charge note) or HCN (hell charge note). \
             A chart that states the kind is played as it states.",
            "차트에 지정이 없는 롱노트를 어떤 방식으로 플레이할지 정합니다: LN(롱노트), CN(차지 노트), HCN(헬 차지 노트). 종류가 지정된 차트는 지정대로 플레이됩니다.",
            "譜面に指定のないロングノートをどう扱うかを選びます: LN（ロングノート）、CN（チャージノート）、HCN（ヘルチャージノート）。\
             種類が指定された譜面はその指定どおりにプレーします。"
        ),
        GaugeSet => (
            "Which table the gauges of a run are built from. AUTO takes the table of the chart's mode; LR2 takes the LR2 table for every chart.",
            "플레이의 게이지를 어느 테이블로 구성할지 정합니다. AUTO는 차트 모드의 테이블을, LR2는 모든 차트에 LR2 테이블을 씁니다.",
            "プレーのゲージをどのテーブルで構成するかを選びます。AUTOは譜面のモードのテーブル、LR2はすべての譜面でLR2のテーブルを使います。"
        ),
        CourseGauge => (
            "Which table the gauges of a course are built from, whatever gauge the course itself asks for. LR2 takes the LR2 table for every chart. \
             BEATORAJA takes the table of each chart's mode. IIDX takes the DAN gauge of the arcade game, which takes half the damage at or below 30%.",
            "코스가 요구하는 게이지와 관계없이 코스의 게이지를 어느 테이블로 구성할지 정합니다. LR2는 모든 차트에 LR2 테이블, BEATORAJA는 차트 모드별 테이블, \
             IIDX는 아케이드의 단위 게이지를 쓰며 30% 이하에서는 피해가 절반입니다.",
            "コース自体が求めるゲージにかかわらず、コースのゲージをどのテーブルで構成するかを選びます。LR2は全譜面でLR2のテーブル、\
             BEATORAJAは譜面のモードごとのテーブル、IIDXはアーケード版の段位ゲージを使い、30%以下ではダメージが半分になります。"
        ),
        GaugeAutoShift => (
            "How the gauge may change during a run. NONE fails the run when the gauge empties. CONTINUE plays on with an empty gauge. \
             SURVIVAL TO GROOVE drops an emptied survival gauge to NORMAL. BEST CLEAR keeps the strongest gauge still clearing, \
             and SELECT TO UNDER the strongest at or below the one selected.",
            "플레이 중 게이지 변경 방식입니다. NONE은 게이지가 비면 실패하고, CONTINUE는 빈 채로 계속합니다. SURVIVAL TO GROOVE는 빈 서바이벌 게이지를 NORMAL로 내립니다. \
             BEST CLEAR는 클리어 가능한 가장 강한 게이지를, SELECT TO UNDER는 선택 게이지 이하 중 가장 강한 게이지를 유지합니다.",
            "ゲージの切り替え方式です。NONEは空になると失敗、CONTINUEは空のまま続行します。\
             SURVIVAL TO GROOVEは空のサバイバルゲージをNORMALに下げ、BEST CLEARはクリア可能な最も強いゲージを、\
             SELECT TO UNDERは選択ゲージ以下で最も強いものを維持します。"
        ),
        BottomShiftableGauge => (
            "The weakest gauge GAUGE AUTO SHIFT may drop a run to: ASSIST EASY, EASY or NORMAL.",
            "게이지 자동 전환이 내려갈 수 있는 가장 약한 게이지입니다: ASSIST EASY, EASY, NORMAL 중 하나.",
            "ゲージオートシフトで下げてよい最も弱いゲージです: ASSIST EASY、EASY、NORMALのいずれか。"
        ),
        Total => (
            "The TOTAL every chart is played with, which sets how much the gauge gains from a note. AUTO, the lowest step, keeps each chart's own TOTAL; \
             a number from 1 to 5000 is used for every chart instead.",
            "모든 차트를 플레이할 때 쓰는 TOTAL로, 노트 하나로 게이지가 얼마나 차는지를 정합니다. 가장 낮은 단계인 AUTO는 차트 고유의 TOTAL을 그대로 쓰고, \
             1~5000의 숫자는 모든 차트에 그 값을 씁니다.",
            "すべての譜面のプレーに使うTOTALで、ノーツ1つでゲージがどれだけ増えるかを決めます。最も低い段階のAUTOは譜面固有のTOTALをそのまま使い、\
             1〜5000の数値を選ぶと全譜面にその値を使います。"
        ),
        Language => (
            "The language the screens are written in, from the next frame on. AUTO follows the first language the system prefers, \
             and English when the screens are not written in it. Each language is named in itself. Words a skin or a chart supplies stay as they are.",
            "화면에 쓰는 언어이며 다음 프레임부터 바로 적용됩니다. AUTO는 시스템이 가장 우선하는 언어를 따르고, 화면이 그 언어로 쓰여 있지 않으면 영어를 씁니다. \
             각 언어는 그 언어로 표기합니다. 스킨이나 차트가 제공하는 글자는 그대로 둡니다.",
            "画面に使う言語で、次のフレームから反映されます。AUTOはシステムが最優先する言語に従い、画面がその言語に対応していなければ英語を使います。\
             各言語はその言語自身の表記で示します。スキンや譜面が提供する文字はそのままです。"
        ),
        WindowMode => (
            "WINDOWED is an ordinary window of the size RESOLUTION names. BORDERLESS covers the whole monitor the window is on.",
            "WINDOWED는 해상도 행이 정한 크기의 일반 창입니다. BORDERLESS는 창이 있는 모니터 전체를 덮습니다.",
            "WINDOWEDは解像度で指定した大きさの通常のウィンドウです。BORDERLESSはウィンドウのあるモニター全体を覆います。"
        ),
        WindowResolution => (
            "The size of the window in WINDOWED mode. A BORDERLESS window takes the size of its monitor instead.",
            "WINDOWED 모드에서 창의 크기입니다. BORDERLESS 창은 대신 모니터 크기를 따릅니다.",
            "WINDOWEDモードでのウィンドウの大きさです。BORDERLESSのウィンドウは代わりにモニターの大きさになります。"
        ),
        Letterbox => (
            "ON fits the screen inside the window at its own proportions. OFF stretches it to the window. A BORDERLESS window is always fitted.",
            "ON이면 화면을 원래 비율로 창 안에 맞추고, OFF이면 창에 맞춰 늘립니다. BORDERLESS 창은 항상 비율을 유지합니다.",
            "ONにすると画面を元の比率のままウィンドウ内に収め、OFFにするとウィンドウに合わせて引き伸ばします。BORDERLESSのウィンドウは常に比率を保ちます。"
        ),
        Font => (
            "The font the player writes its text with. Choose a TTF, OTF or TTC file; it is loaded at once and again at every start. \
             DEFAULT is the font built into the player.",
            "플레이어가 글자를 쓸 글꼴입니다. TTF, OTF, TTC 파일을 고르면 바로 불러오고 시작할 때마다 다시 불러옵니다. DEFAULT는 플레이어에 내장된 글꼴입니다.",
            "プレーヤーが文字の表示に使うフォントです。TTF、OTF、TTCファイルを選ぶとすぐに読み込まれ、起動のたびに再度読み込まれます。\
             DEFAULTはプレーヤー内蔵のフォントです。"
        ),
        Bga => (
            "Shows the background animation of a chart while it is played.",
            "플레이 중 차트의 배경 애니메이션(BGA)을 표시합니다.",
            "プレー中、譜面の背景アニメーション（BGA）を表示します。"
        ),
        ScoreGraph => (
            "Shows score ranks and score differences on the built-in screens and in the lists of records.",
            "내장 화면과 기록 목록에 스코어 등급과 스코어 차이를 표시합니다.",
            "内蔵画面と記録の一覧に、スコアのランクとスコア差を表示します。"
        ),
        ResultGraphs => (
            "Draws the gauge, timing and judge distribution graphs on the built-in result screen.",
            "내장 결과 화면에 게이지, 타이밍, 판정 분포 그래프를 그립니다.",
            "内蔵のリザルト画面に、ゲージ・タイミング・判定分布のグラフを描画します。"
        ),
        ReplayAnalysis => (
            "Lets a replay be stepped through and slowed down while it is played back.",
            "리플레이를 재생하는 동안 한 단계씩 넘기거나 느리게 볼 수 있게 합니다.",
            "リプレイの再生中に、コマ送りやスロー再生ができるようにします。"
        ),
        Preview => (
            "Plays a preview of the song under the cursor on the song select. Its volume and fade are in the SELECT group.",
            "선곡 화면에서 커서가 있는 곡의 미리듣기를 재생합니다. 볼륨과 페이드는 선곡 그룹에 있습니다.",
            "選曲画面でカーソル上の曲のプレビューを再生します。音量とフェードは選曲グループにあります。"
        ),
        Skin => (
            "The layout of the built-in note field: NORMAL or WIDE. It reads CUSTOM while a layout file named on the command line is in use, \
             and choosing a value here lets go of that file.",
            "내장 노트 필드의 레이아웃입니다: NORMAL 또는 WIDE. 명령줄로 지정한 레이아웃 파일을 쓰는 동안은 CUSTOM으로 표시되며, 여기서 값을 고르면 그 파일을 놓습니다.",
            "内蔵ノーツフィールドのレイアウトです: NORMALまたはWIDE。コマンドラインで指定したレイアウトファイルを使用中はCUSTOMと表示され、\
             ここで値を選ぶとそのファイルの使用をやめます。"
        ),
        FiveKeyLayout => (
            "Lays a five-key chart out on lane widths of its own on the built-in note field, rather than on the seven-key ones.",
            "내장 노트 필드에서 5키 차트를 7키용이 아닌 전용 레인 폭으로 배치합니다.",
            "内蔵ノーツフィールドで、5鍵の譜面を7鍵用ではなく専用のレーン幅で配置します。"
        ),
        WhiteNumber => (
            "Shows the white number on the built-in note field: how long a note takes to reach the bottom of the lane cover.",
            "내장 노트 필드의 WHITE NUMBER 표시입니다. 노트가 레인 커버 아래까지 내려오는 데 걸리는 시간입니다.",
            "内蔵ノーツフィールドのWHITE NUMBER表示です。ノーツがレーンカバーの下端に届くまでにかかる時間です。"
        ),
        JudgeTextY => (
            "Where the judgement is written on the built-in note field, as a share of the screen's height. 0% leaves it where the layout puts it.",
            "내장 노트 필드에서 판정 글자를 쓰는 위치를 화면 높이에 대한 비율로 정합니다. 0%이면 레이아웃이 정한 위치 그대로 둡니다.",
            "内蔵ノーツフィールドで判定文字を表示する位置を、画面の高さに対する割合で指定します。0%ならレイアウトが定めた位置のままです。"
        ),
        DebugMode => (
            "Draws the frame and audio counters on the screen.",
            "프레임과 오디오 카운터를 화면에 표시합니다.",
            "フレームとオーディオのカウンターを画面に表示します。"
        ),
        ScratchSide => (
            "The side of the built-in note field the scratch lane is on. A screen drawn by a skin follows the skin's own SCRATCH SIDE option in the PLAY menu.",
            "내장 노트 필드에서 스크래치 레인이 놓이는 쪽입니다. 스킨이 그리는 화면은 플레이 메뉴에 있는 스킨 자체의 SCRATCH SIDE 옵션을 따릅니다.",
            "内蔵ノーツフィールドでスクラッチレーンを置く側です。スキンが描画する画面は、プレーメニューにあるスキン自身のSCRATCH SIDEオプションに従います。"
        ),
        KeyConfig => (
            "Opens the key editor, where every key and controller button the player reads is listed and can be changed.",
            "플레이어가 읽는 모든 키와 컨트롤러 버튼이 나열되고 바꿀 수 있는 키 편집기를 엽니다.",
            "プレーヤーが読み取るすべてのキーとコントローラーのボタンを一覧し、変更できるキーエディターを開きます。"
        ),
        KeyDebounce => (
            "How long a lane key of the keyboard is held after it goes down while a chart is played, so a switch that chatters plays as one press, the way LR2 does. \
             A press is always played the instant it comes; a release inside the window is played when the window ends, if the key is still up then. \
             0 MS plays every change. A controller keeps its own window.",
            "플레이 중 키보드 레인 키를 누른 뒤 그 상태를 유지하는 시간으로, 채터링하는 스위치도 LR2처럼 한 번의 입력이 됩니다. \
             누름은 즉시 처리하고, 구간 안의 뗌은 구간이 끝날 때 키가 아직 떼어져 있으면 처리합니다. 0 MS는 모든 변화를 처리하며, 컨트롤러는 자체 구간을 씁니다.",
            "キーボードのレーンキーを押した後その状態を保持する時間で、チャタリングするスイッチもLR2のように1回の入力になります。\
             押下は即座に処理され、保持時間内の離しは終了時にまだ離れていれば処理されます。0 MSなら全変化を処理します。\
             コントローラーは独自の時間を持ちます。"
        ),
        PadDebounce => (
            "How long a controller button is held after it goes down, on every screen, so a switch that chatters reads as one press, the way LR2 does. \
             A press counts at once; a release inside the window counts when it ends, if still up. A turntable is deaf that long after each change. \
             0 MS reads every change. Kept in the key config.",
            "모든 화면에서 컨트롤러 버튼을 누른 뒤 그 상태를 유지하는 시간으로, 채터링하는 스위치도 LR2처럼 한 번의 입력이 됩니다. \
             누름은 즉시 반영하고, 구간 안의 뗌은 구간이 끝날 때 아직 떼어져 있으면 반영합니다. 턴테이블은 변화마다 그 시간 동안 입력을 받지 않습니다. \
             0 MS는 모든 변화를 읽습니다. 키 설정에 저장됩니다.",
            "全画面でコントローラーのボタンを押した後その状態を保持する時間です。チャタリングするスイッチもLR2のように1回の入力になります。\
             押下は即座に反映され、保持時間内の離しは終了時にまだ離れていれば反映されます。ターンテーブルは変化後その間無反応です。\
             0 MSなら全変化を読み取ります。キー設定に保存されます。"
        ),
        ServerUrl => (
            "The address of the score server. Left empty the player stays offline and nothing is sent.",
            "스코어 서버의 주소입니다. 비워 두면 플레이어는 오프라인으로 동작하며 아무것도 전송하지 않습니다.",
            "スコアサーバーのアドレスです。空欄のままだとプレーヤーはオフラインのままで、何も送信しません。"
        ),
        PlayerId => (
            "The id LOGIN and REGISTER sign in with. While signed in, scores and settings go to the account the token belongs to. \
             Empty is guest, and LOGOUT puts it back to guest.",
            "로그인과 회원가입에 쓰는 ID입니다. 로그인한 동안 스코어와 설정은 토큰이 속한 계정으로 전송됩니다. 비어 있으면 게스트이며, 로그아웃하면 게스트로 돌아갑니다.",
            "ログインと新規登録に使うIDです。ログイン中は、スコアと設定がトークンの属するアカウントへ送られます。空欄はゲストで、ログアウトするとゲストに戻ります。"
        ),
        Account => (
            "The account this player is signed in to. ENTER writes its state on the status line.",
            "이 플레이어가 로그인한 계정입니다. ENTER를 누르면 상태 줄에 그 상태를 표시합니다.",
            "このプレーヤーがログインしているアカウントです。ENTERを押すと、その状態をステータス行に表示します。"
        ),
        Email => (
            "The address REGISTER creates an account with. LOGIN does not need it.",
            "회원가입 때 계정을 만드는 데 쓰는 주소입니다. 로그인에는 필요하지 않습니다.",
            "新規登録でアカウントを作成するときに使うアドレスです。ログインには不要です。"
        ),
        Password => (
            "The password for the next LOGIN or REGISTER. It is kept in memory only, is never written to the settings file, and is forgotten once it is used.",
            "다음 로그인 또는 회원가입에 쓰는 비밀번호입니다. 메모리에만 보관되고 설정 파일에는 저장되지 않으며, 사용하면 바로 잊습니다.",
            "次のログインまたは新規登録に使うパスワードです。メモリ上にのみ保持され、設定ファイルには書き込まれず、使用後は破棄されます。"
        ),
        Login => (
            "Signs in to the score server with PLAYER ID and PASSWORD.",
            "플레이어 ID와 비밀번호로 스코어 서버에 로그인합니다.",
            "プレーヤーIDとパスワードでスコアサーバーにログインします。"
        ),
        Register => (
            "Creates an account on the score server from PLAYER ID, EMAIL and PASSWORD.",
            "플레이어 ID, 이메일, 비밀번호로 스코어 서버에 계정을 만듭니다.",
            "プレーヤーID、メールアドレス、パスワードでスコアサーバーにアカウントを作成します。"
        ),
        Logout => (
            "Forgets the signed-in account on this machine and puts PLAYER ID back to guest.",
            "이 기기에서 로그인한 계정을 잊고 플레이어 ID를 게스트로 되돌립니다.",
            "この端末でログイン中のアカウントを破棄し、プレーヤーIDをゲストに戻します。"
        ),
        SyncSettings => (
            "Mirrors the settings and the key configuration of this machine to the account.",
            "이 기기의 설정과 키 설정을 계정에 그대로 동기화합니다.",
            "この端末の設定とキー設定をアカウントに同期します。"
        ),
        UploadSettings => (
            "Sends the settings and the key configuration of this machine to the account now.",
            "이 기기의 설정과 키 설정을 지금 계정으로 전송합니다.",
            "この端末の設定とキー設定を今すぐアカウントへ送信します。"
        ),
        DownloadSettings => (
            "Takes the settings and the key configuration the account holds and applies them to this machine now.",
            "계정에 저장된 설정과 키 설정을 지금 이 기기에 적용합니다.",
            "アカウントに保存されている設定とキー設定を今すぐこの端末に適用します。"
        ),
        SyncRecords => (
            "Sends every run kept on this machine that the score server has not taken yet, with its replay. It also runs at start and on sign-in while you are signed in.",
            "이 기기에 있는 플레이 기록 중 스코어 서버가 아직 받지 않은 것을 리플레이와 함께 모두 전송합니다. 로그인한 상태에서는 시작할 때와 로그인할 때도 실행됩니다.",
            "この端末に保存されている、スコアサーバーがまだ受け取っていないプレー記録をリプレイとともにすべて送信します。ログイン中は、起動時とログイン時にも実行されます。"
        ),
        AutoUploadReplay => (
            "Sends the replay of a run to the score server after a score that was ranked.",
            "랭크된 스코어가 나온 뒤 그 플레이의 리플레이를 스코어 서버로 전송합니다.",
            "ランク対象となったスコアのあと、そのプレーのリプレイをスコアサーバーに送信します。"
        ),
        AutoUploadTrace => (
            "Sends each run's note-by-note record (judgments, gauges, scroll changes) once the server took its score. Only you read it; \
             with sharing on in the web settings, a rival who has you as one too sees its summary by section. \
             Every run keeps it here; an earlier run gets one from its replay when that plays back exactly.",
            "서버가 스코어를 받은 뒤 각 플레이의 노트별 기록(판정, 게이지, 스크롤 변화)을 전송합니다. 본인만 볼 수 있으며, 웹 설정에서 공유를 켜면 서로 라이벌인 상대가 \
             구간별 요약을 볼 수 있습니다. 모든 플레이가 이 기기에 기록을 보관하고, 이전 플레이는 리플레이가 정확히 재생될 때 거기서 기록을 만듭니다.",
            "サーバーがスコアを受け取った後、各プレーのノーツごとの記録（判定、ゲージ、スクロール変化）を送信します。閲覧できるのは本人だけで、\
             ウェブ設定で共有をオンにすると、お互いをライバルにしている相手が区間ごとの要約を見られます。\
             以前のプレーはリプレイが正確に再生できればそこから記録を作ります。"
        ),
        Rivals => (
            "The players shown beside you on a ranking. ENTER opens the list to add and remove them.",
            "랭킹에서 내 옆에 표시되는 플레이어입니다. ENTER로 목록을 열어 추가하거나 삭제할 수 있습니다.",
            "ランキングで自分の隣に表示されるプレーヤーです。ENTERでリストを開き、追加や削除ができます。"
        ),
        IrProfiles => (
            "More score servers every run is also sent to. ENTER opens the list to add, switch off and remove them.",
            "모든 플레이를 추가로 전송할 다른 스코어 서버입니다. ENTER로 목록을 열어 추가, 끄기, 삭제를 할 수 있습니다.",
            "すべてのプレーを追加で送信する別のスコアサーバーです。ENTERでリストを開き、追加・無効化・削除ができます。"
        ),
        DiscordStatus => (
            "Shows R-BMS as the game you are playing on your Discord profile, with the song, its mode and difficulty, the course and how long you have played. \
             Anyone who can see your profile sees it. Needs the Discord desktop app on this computer.",
            "Discord 프로필에 R-BMS를 플레이 중인 게임으로 표시하며, 곡, 모드와 난이도, 코스, 플레이한 시간이 함께 보입니다. \
             프로필을 볼 수 있는 모든 사람에게 보이며, 이 컴퓨터에 Discord 데스크톱 앱이 필요합니다.",
            "Discordのプロフィールに、プレー中のゲームとしてR-BMSを表示し、曲、モードと難易度、コース、プレー時間も表示されます。\
             プロフィールを見られる全員に見えます。このコンピューターにDiscordデスクトップアプリが必要です。"
        ),
        MasterVolume => (
            "The volume of everything the player sounds, applied after the three below are mixed.",
            "플레이어가 내는 모든 소리의 볼륨으로, 아래 세 가지를 섞은 뒤에 적용됩니다.",
            "プレーヤーが鳴らすすべての音の音量で、下の3つをミックスした後に適用されます。"
        ),
        KeyVolume => (
            "The volume of the key sounds: what a note sounds when it is hit.",
            "키음의 볼륨입니다. 노트를 쳤을 때 나는 소리입니다.",
            "キー音の音量です。ノーツを叩いたときに鳴る音です。"
        ),
        BgmVolume => (
            "The volume of the background sounds of a chart.",
            "차트의 배경음 볼륨입니다.",
            "譜面のバックグラウンド音の音量です。"
        ),
        SystemVolume => (
            "The volume of the system sounds, which are read from SOUND FOLDER.",
            "사운드 폴더에서 읽어 오는 시스템 사운드의 볼륨입니다.",
            "サウンドフォルダから読み込むシステム音の音量です。"
        ),
        AudioDevice => (
            "The device sound is played on. DEFAULT follows the system's own choice. The line at the foot of this panel says what was opened.",
            "소리를 재생할 장치입니다. DEFAULT는 시스템의 선택을 따르며, 이 패널 아래쪽 줄에 실제로 연 장치가 표시됩니다.",
            "音を再生するデバイスです。DEFAULTはシステムの選択に従い、このパネル下部の行に実際に開いたデバイスが表示されます。"
        ),
        AudioBuffer => (
            "How many frames the output is asked for at a time. AUTO leaves it to the audio backend. Changing it opens the output again.",
            "출력이 한 번에 요청하는 프레임 수입니다. AUTO는 오디오 백엔드에 맡기며, 바꾸면 출력을 다시 엽니다.",
            "出力が一度に要求するフレーム数です。AUTOはオーディオバックエンドに任せ、変更すると出力を開き直します。"
        ),
        AudioSampleRate => (
            "The rate the output is asked to run at, in Hz. AUTO takes the device's own. When the device cannot run at the rate asked for, \
             the line at the foot of this panel says what it was opened at.",
            "출력이 동작하도록 요청하는 샘플레이트(Hz)입니다. AUTO는 장치 고유의 값을 씁니다. 장치가 요청한 값으로 동작하지 못하면 이 패널 아래쪽 줄에 실제 값이 표시됩니다.",
            "出力に要求するサンプルレート（Hz）です。AUTOはデバイス固有の値を使います。デバイスが要求どおりに動作できない場合は、このパネル下部の行に実際の値が表示されます。"
        ),
        AudioPolyphony => (
            "How many sounds the mixer may play at once. Changing it opens the output again.",
            "믹서가 동시에 재생할 수 있는 소리의 수입니다. 바꾸면 출력을 다시 엽니다.",
            "ミキサーが同時に再生できる音の数です。変更すると出力を開き直します。"
        ),
        SoundFolder => (
            "The folder the system sounds are read from. With none chosen every system sound is silent.",
            "시스템 사운드를 읽어 오는 폴더입니다. 고르지 않으면 모든 시스템 사운드가 무음입니다.",
            "システム音を読み込むフォルダです。未選択の場合、すべてのシステム音が無音になります。"
        ),
        GuideSe => (
            "Sounds a guide cue at every judgement of a run. The cues are system sounds, read from SOUND FOLDER.",
            "플레이 중 판정마다 가이드 소리를 냅니다. 이 소리는 사운드 폴더에서 읽어 오는 시스템 사운드입니다.",
            "プレー中、判定のたびにガイド音を鳴らします。このガイド音は、サウンドフォルダから読み込むシステム音です。"
        ),
        Sort => (
            "The order of the song list. The sort key of the song select steps the same value.",
            "곡 목록의 정렬 순서입니다. 선곡 화면의 정렬 키도 같은 값을 순서대로 바꿉니다.",
            "曲リストの並び順です。選曲画面のソートキーも同じ値を順に切り替えます。"
        ),
        FavoriteOnly => (
            "Lists only the charts marked as favourites.",
            "즐겨찾기로 표시한 차트만 목록에 표시합니다.",
            "お気に入りに登録した譜面だけを一覧に表示します。"
        ),
        ShowMissingCharts => (
            "Lists the charts a difficulty table names that the library does not have, dimmed. Choosing one offers its download.",
            "난이도표에 있지만 라이브러리에 없는 차트를 흐리게 표시합니다. 고르면 다운로드를 제안합니다.",
            "難易度表に載っているがライブラリにない譜面を、薄く表示します。選ぶとダウンロードを提案します。"
        ),
        RankingKeys => (
            "Holding a blue key on the chart under the cursor opens the IR RANKING of that chart. A short press does what the key always did, \
             once it is let go. On the keyboard keys 2 and 4 open it, and 9KEYS uses buttons 2 and 8. With OFF the keys work as they always have.",
            "커서가 있는 차트에서 파란 건반을 누르고 있으면 그 차트의 IR RANKING 화면이 열립니다. 짧게 누르면 건반을 뗄 때 원래 동작을 합니다. \
             키보드에서는 2번·4번 키로 열고, 9KEYS 모드에서는 2번·8번 버튼을 씁니다. OFF 상태에서는 건반이 원래대로 동작합니다.",
            "カーソルの譜面で青い鍵盤を押し続けると、その譜面のIR RANKING画面が開きます。短く押すと、離したときに元の動作をします。\
             キーボードでは2番・4番キーで開き、9KEYSモードではボタン2・8を使います。OFFでは鍵盤は元のとおりに動作します。"
        ),
        DownloadFolder => (
            "The folder the song list unpacks a chart it downloads into. It is scanned with the song folders. With none chosen, the first download asks for one.",
            "곡 목록에서 받은 차트를 압축 해제할 폴더입니다. 곡 폴더와 함께 스캔됩니다. 고르지 않으면 첫 다운로드 때 폴더를 묻습니다.",
            "曲リストでダウンロードした譜面を展開するフォルダです。曲フォルダとあわせてスキャンされます。未選択の場合、最初のダウンロード時に尋ねます。"
        ),
        PreviewVolume => (
            "How loud the preview of the song under the cursor is played.",
            "커서가 있는 곡의 미리듣기를 재생하는 볼륨입니다.",
            "カーソル上の曲のプレビューを再生する音量です。"
        ),
        PreviewFade => (
            "How long the preview takes to fade in and to fade out. 0 MS starts and stops it at once.",
            "미리듣기가 페이드 인·아웃하는 데 걸리는 시간입니다. 0 MS는 즉시 시작하고 즉시 멈춥니다.",
            "プレビューがフェードイン・フェードアウトにかける時間です。0 MSなら即座に開始・停止します。"
        ),
    }
}

#[cfg(test)]
mod tests {
    use rbms_model::translations::catalog_problems;

    use super::*;

    #[test]
    fn every_sentence_is_written_in_every_language_with_the_same_placeholders() {
        assert_eq!(catalog_problems(RowDescriptionText::ALL), Vec::<String>::new());
    }
}
