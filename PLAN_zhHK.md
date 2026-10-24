# openMapper 繁體中文（香港）翻譯計劃

對象：`openMapper-main`（v0.1.5，commit `f9e2aa4`）
目標語言：繁體中文・香港（`zh-HK`，對應 OMSI 語言代碼 `ZHT`）

---

## 一、現況核實（已讀碼確認）

### 1.1 翻譯機制：兩語並排函數

`src/i18n.rs` 只有兩個語言，用 `tr(en, ru)` 把兩種文字寫在同一個呼叫點：

```1:53:openMapper-main/src/i18n.rs
//! Two languages, written side by side where the text is used: `tr("Save", "Сохранить")`.

static LANG: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
}

pub fn tr(en: &'static str, ru: &'static str) -> &'static str {
    match get() {
        Lang::En => en,
        Lang::Ru => ru,
    }
}
```

規模（以 regex 掃 `src/**/*.rs` 實測）：

| 項目 | 數量 |
|---|---|
| `tr(` 呼叫點 | 2,877 |
| 可配對嘅字面量 `tr("..","..")` | 2,464 |
| 唯一英文句 | 1,974 |
| 英文總字元 | 約 66,649（平均 27 字/句） |
| 涉獵檔案 | 134 個 `.rs` |

分佈最密嘅檔案（`tr()` 數）：`ui.rs` 413、`ui_mesh.rs` 341、`ui_edit.rs` 182、`editor.rs` 180、`ui_geo.rs` 122、`check.rs` 117、`ui_timetable.rs` 107、`ui_chrono.rs` 93、`app.rs` 89。

### 1.2 指令面板另一套機制

`src/ui_palette.rs` 有 `cmd!` 宏，除中英文名稱外，仲有**搜尋關鍵字**欄：

```25:28:openMapper-main/src/ui_palette.rs
macro_rules! cmd {
    ($icon:expr_2021, $en:literal, $ru:literal, $words:literal, $key:literal, $run:expr_2021) => {
        Command { icon: $icon, name: tr($en, $ru), words: concat!($en, " ", $ru, " ", $words), key: $key, run: $run }
    };
}
```

共 77 條命令。搜尋係「兩種語言名 + 關鍵字」都食，所以中文介面要 **中文名 + 中文搜尋詞**（例如「儲存」「另存」），否則用戶用中文打字搵唔到嘢。

### 1.3 語言選擇與偵測

- 選單：`src/ui.rs` `lang_menu()` 只有 `(Lang::En, "English")`、`(Lang::Ru, "Русский")` 兩個 radio。
- 偏好：`src/install.rs` `Prefs.language` 存 `"en"` / `"ru"` 字串（空 = 跟系統）。
- 偵測：`src/i18n.rs` `from_code()` 讀 `LC_ALL` / `LC_MESSAGES` / `LANG`（macOS 另讀 `AppleLanguages`），目前只認 `ru` / `uk` / `be` → 俄文，其餘全部英文。**`zh-HK` 會掉返英文。**
- 環境變數：`OPENMAPPER_LANG` 可強制指定（`src/app.rs`）。
- 啟動時套用：`src/app.rs:157`。

### 1.4 字型：最關鍵的缺口

`src/icons.rs` `fonts()` 只掛兩個字型：

```156:166:openMapper-main/src/icons.rs
pub fn fonts() -> FontDefinitions {
    let mut f = FontDefinitions::default();
    f.font_data.insert(FAMILY.into(), std::sync::Arc::new(FontData::from_static(include_bytes!("../assets/fonts/MaterialSymbolsSubset.ttf"))));
    // (a text font behind the icons: egui wants its replacement character in every family)
    let text = f.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    f.families.insert(FontFamily::Name(FAMILY.into()), std::iter::once(FAMILY.to_string()).chain(text).collect());
    for fam in [FontFamily::Proportional, FontFamily::Monospace] {
        f.families.entry(fam).or_default().push(FAMILY.into());
    }
    f
}
```

`assets/fonts/` 只有 `InterVariable.ttf`（Inter 無中文字）同 Material Symbols（圖示）。egui 內建嘅 Ubuntu-Light / Hack / emoji 亦無 CJK。

**結論：就算加咗中文翻譯，畫面只會出現「豆腐格」□□□。字型係第一優先，唔係收尾工序。**

### 1.5 其他受影響位置

- `src/descriptions.rs` `LANGS`：地圖名稱/描述嘅多語言檔（`global_ZHT.dsc`）語言代碼表，現時無中文。
- openOMSI 側已有對應：`omsi-cfg/src/lang.rs` 有 `("ZHT", "繁體中文", "zh-tw", &["zh-tw","zh-hant","zh-hk","zh-mo","cht",…], "zho_Hant")` → 我哋直接用 **`ZHT`**，與 openOMSI 一致。
- `src/geo/fetch.rs:303`：Nominatim 查地名用 `i18n::code()` 傳 `accept-language`，中文時應傳 `zh-HK`。
- `tools/package-windows.ps1` NSIS 安裝程式語言（可選加入正體中文）。
- `docs/TUTORIAL.md`、`README.md`、`CHANGELOG.md`（可選）。

---

## 二、目標

1. 介面 100% 有繁體中文（香港）翻譯，唔會出現英文殘留或豆腐格。
2. 用香港慣用語（見 §6 詞彙表），唔用大陸用語。
3. 指令面板可以用中文搜尋到每一條命令。
4. 系統為 `zh-HK` 時首次啟動自動切中文。
5. 地圖描述可以出 `global_ZHT.dsc`。
6. **上游 openMapper 日後出新版時，盡量唔使大量重譯**（設計重點，見 §3）。

非目標（本次唔做）：簡體中文、日文、韓文；不重寫英文原文。

---

## 三、方案比較

### 方案 A：三參數 `tr(en, ru, zh)`

把 2,464 個字面量呼叫點全部改成 `tr("Save", "Сохранить", "儲存")`。

- 好處：翻譯同原文同一行，IDE 內一眼睇到，唔會漏。
- 壞處：diff 巨大（每個檔案都要改），日後每次 `git pull` 上游都必爆衝突；翻譯工作同 code 混在同一檔，review 難；改一個譯要重新 compile 全樹。

### 方案 B：翻譯表 + 零呼叫點改動（**建議採用**）

保留所有 `tr("Save", "Сохранить")` 原封不動，另開一個獨立嘅翻譯表檔 `src/lang/zh_hk.rs`，用英文原文做 key。`tr()` 喺中文模式下改成查表。

```rust
pub fn tr(en: &'static str, ru: &'static str) -> &'static str {
    match get() {
        Lang::En => en,
        Lang::Ru => ru,
        Lang::ZhHk => crate::lang::zh_hk::lookup(en).unwrap_or(en),
    }
}
```

- 好處：
  - **2,464 個呼叫點一行都唔使改**，上游合併零衝突（只有 `i18n.rs` 一個檔會撞）。
  - 翻譯獨立成檔，唔使識 Rust 嘅翻譯員都能改，code review 乾淨。
  - 英文原文係 key，英文日後改字只要 build check 提醒補翻譯。
- 壞處：翻譯同原文唔同檔，開發新功能時容易「寫完英文漏翻譯」→ 用 build.rs 檢查解決（§4.3）。
- 額外注意：少數 `tr()` 嘅第一個參數唔係英文（例如 `cmd!(Icon::Settings, "Русский язык", "English language", …)` 呢個語言切換命令），要特別處理（§4.4）。

### 方案 C：外部 JSON/TOML 翻譯檔 + runtime 載入

- 好處：翻譯檔可以熱改、唔使 recompile。
- 壞處：`tr()` 回傳 `&'static str`，改成 runtime 載入要改成 `String`／`Cow`，會推翻 2,464 個呼叫點嘅型別（`format!` 內更麻煩）；多一層 runtime 錯誤來源。**唔建議。**

**決定：方案 B。** 理由：上游相容性（openMapper 仲有新功能每星期出）＋ 翻譯可獨立維護。

---

## 四、架構設計

### 4.1 新增檔案

| 檔案 | 內容 |
|---|---|
| `src/lang/mod.rs` | 模組入口，暴露 `lookup(en) -> Option<&'static str>`、`search_words(en) -> &'static str` |
| `src/lang/zh_hk.rs` | 繁中翻譯表（`&[(&str, &str)]` 配對陣列），分區塊排列 |
| `src/lang/zh_hk_search.rs` | 中文搜尋關鍵字表（對應 `cmd!` 嘅 77 條命令） |
| `assets/fonts/NotoSansTC-Regular.ttf` | 中文字型（OFL 授權） |
| `assets/fonts/LICENSE-NotoSansTC.txt` | 字型授權 |
| `tools/extract-i18n.py` | 抽所有 `tr()` 英文 key → 產生翻譯模板／檢查缺漏 |
| `tools/subset-cjk.py` | 字型子集化（可選，見 §5.3） |

### 4.2 `src/i18n.rs` 改動

```rust
pub enum Lang { En, Ru, ZhHk }

pub fn code(l: Lang) -> &'static str {
    match l {
        Lang::En => "en",
        Lang::Ru => "ru",
        Lang::ZhHk => "zh-HK",
    }
}

pub fn from_code(c: &str) -> Lang {
    match c {
        "ru" => Lang::Ru,
        "en" => Lang::En,
        "zh-hk" | "zh-hant-hk" | "zh-tw" | "zh-hant" | "cht" => Lang::ZhHk,
        _ => {
            // 系統偵測：LC_ALL / LC_MESSAGES / LANG，macOS 另讀 AppleLanguages
            // "zh-HK" / "zh_TW" / "zh-Hant" → 繁中
            // "zh-CN" / "zh-Hans" → 暫時回落英文（本次唔做簡體）
        }
    }
}
```

`Lang` 現時用 `AtomicU8` 存，`Ru = 1`；新增 `ZhHk = 2` 即可，`get()` 改為三路 match。**注意** `from_code` 現時會把 `uk` / `be` 當俄文，`zh-BE` 之類邊界要小心只匹配 `zh-` 前綴。

### 4.3 缺翻譯檢查（build.rs 或獨立測試）

`build.rs` 已經存在，可以加一個步驟：

1. 掃 `src/**/*.rs`，用一個穩健嘅 Rust 字串常值 parser（處理 `\"`、`\\`、`{...}`）抽出所有 `tr()` 第一個參數嘅英文原文。
2. 對照 `src/lang/zh_hk.rs` 入面嘅 key。
3. 有缺漏 → **編譯期報錯**，列出缺嘅句子（首 20 條 + 總數），並提示跑 `tools/extract-i18n.py --missing`。
4. 同時檢查「譯文含 `{placeholder}` 但英文冇」或反之 → 防止 `format!` 位置錯位（`ui_geo.rs:1488` 就有 `{z} {x} {y} {bbox} {width} {height} {key}`）。

> 解析器唔好用 regex 硬寫：`tr("a \"b\" c", "…")` 呢類會爆。建議用 Python 工具 + Rust `syn`（工作區已有 `syn` 依賴？需確認；否則手寫狀態機解析字串常值）。

### 4.4 指令面板（`src/ui_palette.rs`）

`cmd!` 宏要改成 runtime 計算，唔能用 `concat!`：

```rust
macro_rules! cmd {
    ($icon:expr_2021, $en:literal, $ru:literal, $words:literal, $key:literal, $run:expr_1) => {
        Command {
            icon: $icon,
            name: crate::i18n::tr($en, $ru),
            words: crate::i18n::tr_words($en, $ru, $words),
            key: $key,
            run: $run,
        }
    };
}
```

- `tr_words()` 回傳英文 + 俄文 + 中文名 + 中文搜尋詞 + 原有 `$words`，中文模式下用戶打「儲存」「地圖」都搵到。
- 語言切換命令（`"Русский язык" / "English language"`）要改寫成三語都合理嘅形式，例如名稱用 `tr("Language", …)`，而 `words` 加「語言」「切换」「切換」。
- `Command.words` 由 `&'static str` 改 `String`（或 `Cow<'static, str>`）。
- `src/ui_palette.rs` 已有測試 `commands_are_found_by_their_names_and_words_in_either_language` → 擴充成三語都驗。

### 4.5 語言選單（`src/ui.rs` `lang_menu()`）

```rust
for (l, n) in [(Lang::En, "English"), (Lang::Ru, "Русский"), (Lang::ZhHk, "繁體中文（香港）")] {
```

### 4.6 偏好與環境變數

- `Prefs.language` 保持 `String`，中文存 `"zh-HK"`（大小寫要同 `from_code` 一致）。
- `OPENMAPPER_LANG=zh-HK` 支援。

### 4.7 地圖描述（`src/descriptions.rs`）

`LANGS` 加 `("ZHT", "繁體中文")`，位置放喺 `("ENG","English")` 附近或按字母。寫出嘅檔名為 `global_ZHT.dsc`，UTF-16 BOM（該模組已有處理）。

### 4.8 地名查詢（`src/geo/fetch.rs`）

`accept-language` 已經用 `i18n::code()` → 中文時自動變 `zh-HK`，Nominatim 支援。**確認咪改過。**

---

## 五、字型方案（關鍵路徑）

### 5.1 選字型

- **Noto Sans TC**（Google / SIL OFL 1.1）：繁中正體，覆蓋港台常用字 + 香港補充字符，設計上照顧繁體。
- 替代：**Source Han Sans TC**（同一字型嘅 Adobe 版本，OFL）。兩者擇一即可。
- 唔建議用微軟正黑體（Noto Sans CJK TC）：授權係 OFL 但要留意，體積較大；且系統字型唔適合打包（授權問題）。

### 5.2 掛入 egui

`src/icons.rs` `fonts()` 要做兩件事：

1. `f.font_data.insert("NotoSansTC", FontData::from_static(include_bytes!("../assets/fonts/NotoSansTC-Regular.ttf")))`。
2. 把 `"NotoSansTC"` 加做 **Proportional 同 Monospace** 嘅 fallback（排在 Material Symbols 之後、egui 內建字型之前）：

```rust
for fam in [FontFamily::Proportional, FontFamily::Monospace] {
    let mut list = f.families.entry(fam).or_default();
    list.insert(0, "NotoSansTC".to_string()); // 文字 fallback
    list.push(FAMILY.into());                 // 圖示
}
```

（實際插入位置要試：Material Symbols 係圖示字型，必須喺 CJK 之後，否則部分漢字會被 icon 字型嘅 fallback 吃掉。）

### 5.3 體積

- 完整 Noto Sans TC Regular ≈ 16–18 MB → 安裝包由 ~10.8 MB 變 ~28 MB。
- **可選**用 `fonttools` 做子集化：只保留 (a) 翻譯表用到嘅字符，(b) 常見 3500 繁體字 + 香港常用字。估計可壓到 2–4 MB。
- 建議：**先用完整字型驗證功能**，功能通咗先做子集化；子集化後要加一個「翻譯表新增字符不在字型內」的檢查（build.rs 已有資訊可以做）。
- 用 `pyftsubset`：
  ```
  pyftsubset assets/fonts/NotoSansTC-Regular.ttf --output-file=assets/fonts/NotoSansTC-Subset.ttf \
    --unicodes="U+0000-00FF,U+2000-206F,U+3000-303F,U+4E00-9FFF,U+FF00-FFEF" --layout-features='*'
  ```

### 5.4 中文斷行

egui 用 `unicode-linebreak`，中文可以逐字斷行，唔使特殊處理；但**標點擺放**（避頭尾）egui 唔做，長句可能出現「，」喺行首嘅情況。可接受，或喺翻譯時避免句子太長。

---

## 六、香港用語詞彙表（翻譯必須遵守）

| 英文 / 俄文 | 繁中（香港） | 唔好用（大陸用語） |
|---|---|---|
| Menu | 選單 | 菜单 |
| Save | 儲存 | 保存 |
| Save as | 另存為 | 另存为 |
| File | 檔案 | 文件 |
| Folder | 資料夾 | 文件夹 |
| Software | 軟件／程式 | 软件 |
| Mouse | 滑鼠 | 鼠标 |
| Screen | 螢幕 | 屏幕 |
| Scroll | 捲動 | 滚动 |
| Drag | 拖曳 | 拖拽 |
| Checkbox | 核取方塊 | 复选框 |
| Dialog | 對話方塊 | 对话框 |
| Quality | 品質 | 质量 |
| Information | 資訊 | 信息 |
| Project | 專案 | 项目 |
| Network | 網絡 | 网络 |
| Server | 伺服器 | 服务器 |
| Internet | 互聯網 | 互联网 |
| Memory | 記憶體 | 内存 |
| Graphics card | 顯示卡 | 显卡 |
| Performance | 效能 | 性能 |
| Default | 預設 | 默认 |
| Error | 錯誤 | 错误 |
| Crash | 當機／程式崩潰 | 崩溃 |
| Lane | 行車線 | 车道 |
| Kerb | 路緣石 | 路缘 |
| Footpath | 行人路 | 人行道 |
| Traffic light | 交通燈 | 红绿灯 |
| Signal | 訊號 | 信号 |
| Junction | 交叉路口 | 路口／交叉口 |
| Tile | 圖塊（OMSI 社群常用 tile） | 图块 |
| Texture | 貼圖 | 纹理 |
| Mesh | 網格 | 网格 |
| Spline | 樣條線 | 样条 |
| Heightmap | 高度圖 | 高度图 |
| Catalogue | 資源目錄 | 目录 |
| Preview | 預覽 | 预览 |
| Shortcut | 快捷鍵 | 快捷键 |
| Keyboard | 鍵盤 | 键盘 |
| Coordinates | 座標 | 坐标 |
| Latitude / Longitude | 緯度／經度 | 纬度／经度 |
| Undo / Redo | 撤銷／重做 | 撤销／重做 |
| Delete | 刪除 | 删除 |
| Copy / Paste | 複製／貼上 | 复制／粘贴 |
| Customise | 自訂 | 自定义 |
| Layout | 佈局 | 布局 |
| Layer | 圖層 | 层 |
| Depot | 車廠 | 车场 |
| Timetable | 時刻表 | 时刻表 |
| Video | 影片 | 视频 |
| Import / Export | 匯入／匯出 | 导入／导出 |
| Tooltip | 工具提示 | 工具提示 |
| Zoom | 縮放 | 缩放 |
| Build | 編譯 | 编译 |

**句式**：用「您」定「你」要統一（建議全程式用「你」，較輕鬆，與原文英文語氣一致）；唔好翻成書面公文体。

---

## 七、分階段執行

### Phase 0 — 準備（0.5 天）
- [x] 開 branch `feat/i18n-zh-hk`（基於 v0.1.5）。
- [x] `git tag backup-v0.1.5-pre-zh`。
- [x] 記錄基線：`cargo build` 通過、`cargo test` 通過。

### Phase 1 — 基礎設施（1 天）
- [x] `src/lang/mod.rs` + `src/lang/zh_hk.rs`（空表 + 5 條示範）。
- [x] `src/i18n.rs`：加 `Lang::ZhHk`、`code()`、`from_code()` 系統偵測、`tr()` 查表。
- [x] `src/ui.rs` `lang_menu()` 加 radio；`src/app.rs` 冇需改（已用 `from_code`）。
- [x] 驗證：`OPENMAPPER_LANG=zh-HK` 啟動，選單變中文（未翻譯者顯示英文 fallback）。

### Phase 2 — 字型（1 天，可與 Phase 1 平行）
- [x] 下載 Noto Sans CJK TC（OFL 授權檔 `assets/fonts/LICENSE-NotoSansCJK.txt`）。
- [x] `src/icons.rs` `fonts()` 掛 fallback（喺 Material Symbols 之前、egui 內建字型之後）。
- [x] 驗證：中文文字正常顯示、圖示正常、無豆腐格、Monospace 面板正常。
- [x] **字型子集化**（`tools/subset-cjk.py`）：完整 Noto Sans CJK TC 係 16 MB，入唔到 repository（CI 擋 ≥ 1 MB 嘅檔）。
  - [x] `tools/fetch-chars.py` 拉常用繁中字符表（粵語常用字 + HKSCS 香港補充字符）→ `tools/i18n/chars.txt`。
  - [x] 子集 = 全部譯文用嘅字符（1,048）＋ 常用字前 800 個 ＋ Latin／標點／符號 → **735 kB**（`assets/fonts/NotoSansCJKtc-Subset.otf`）。
  - [x] 完整字型唔入 repository（`.gitignore`），要時 `tools/subset-cjk.py` 會自動下載。
  - [x] `src/lang/mod.rs` 測試：讀字型嘅 cmap，確認**每個譯文字符都有字**（唔會出現豆腐格）。

### Phase 3 — 抽字串工具 + 檢查（1 天）
- [x] `tools/extract-i18n.py`：抽出所有 `tr()` 嘅英文 key → `--missing` / `--template` / `--check` / `--by-file`。
- [x] `tools/i18n/*.tsv`：譯文以「英文<TAB>繁中」存放（翻譯員用純文字編輯器改，唔使識 Rust）。
- [x] `tools/generate-lang.py`：由 TSV 生成 `src/lang/zh_hk.rs`（`--check` 檢查檔案係咪最新）。
- [x] `tools/check-lang.py`：檢查亂碼（U+FFFD）、placeholder 對唔上、譯文空白、同句譯得唔同。
- [x] `tools/check-i18n.py`：一次過跑齊所有檢查（`--fix` 會先生成 Rust 表）。
- [x] `tools/fetch-chars.py` + `tools/subset-cjk.py`：字型子集化（見 Phase 2）。
- [x] 缺譯檢查：`tools/extract-i18n.py --check`（2,109 句全部已譯），已入 CI（`.github/workflows/ci.yml` 嘅 `i18n` job）。

### Phase 4 — 翻譯內容（主體工作量，估 6–10 天）

按「價值／風險」排序批做，每批做完即 build + 肉眼試：

| 批次 | 範圍 | 句數（估） | 重點 |
|---|---|---|---|
| 4a | 選單列、工具列、常用按鈕（`ui.rs` 前段） | ~120 | 「儲存」「開啟」「工具」等高頻詞，先定術語 |
| 4b | 錯誤／提示訊息（`app.rs`、`editor.rs`、各 `check.rs`） | ~300 | 句子短、要用「你」 |
| 4c | 面板標題與欄位（`ui_mesh.rs`、`ui_edit.rs`、`ui_roads.rs`、`ui_mapsettings.rs` 等） | ~700 | 術語一致性最重要 |
| 4d | 檢查器與報告（`check.rs` 117、`gamecheck.rs` 12） | ~150 | 專業詞（碰撞、Z-fighting、貼圖重疊） |
| 4e | 時刻表／路線／交通（`ui_timetable.rs`、`ui_traffic.rs`、`ui_hof.rs`） | ~250 | OMSI 專有名詞（停站、發車、班次） |
| 4f | 教學長文（`ui_guide.rs`） | ~60 | 最長句、最需要文筆；建議最後做、最好逐句校對 |
| 4g | 地理／WMS／衛星（`ui_geo.rs`） | ~140 | 技術詞、含 `{z}{x}{y}{bbox}` placeholder |
| 4h | 雜項（`ui_pbr.rs`、`ui_uv.rs`、`ui_merge.rs`、`ui_find.rs`、updater 等） | ~300 | 補漏 |

**每批規則**：譯文放喺 `tools/i18n/zh_hk*.tsv`（唔係直接改 Rust 檔），key 必須係**完整英文原文**（一字不差）；改完跑 `python tools/check-i18n.py --fix` 生成 Rust 表。新功能先加英文 key，再補譯（檢查會逼住你補）。

**實際進度**：2,109 句全部譯好（`extract-i18n.py --check` 通過），分 9 個 TSV 檔：`zh_hk.tsv`、`zh_hk_app.tsv`、`zh_hk_edit.tsv`、`zh_hk_last.tsv`、`zh_hk_lines.tsv`、`zh_hk_mesh.tsv`、`zh_hk_panels.tsv`、`zh_hk_rest.tsv`、`zh_hk_ui.tsv`（另外 `zh_hk_search.tsv` 係 77 條命令嘅搜尋詞）。

### Phase 5 — 指令面板中文搜尋（0.5–1 天）
- [x] `cmd!` 宏改 runtime、`tr_words()`。
- [x] 77 條命令寫中文搜尋詞（例：「開啟地圖」「匯出」「匯入」「合併圖塊」「路線圖」）。
- [x] 擴充 `ui_palette.rs` 既有測試到三語。

### Phase 6 — 地圖描述 + 雜項（0.5 天）
- [x] `src/descriptions.rs` `LANGS` 加 `ZHT`。
- [x] 驗證：`global_ZHT.dsc` 寫出後 openOMSI launcher 讀到。（未做：要裝 openOMSI 實測，見 §13）

### Phase 7 — 收尾驗證（1 天）
- [x] `cargo build --release`、全套 `cargo test`。
- [x] 中文模式逐個面板行一遍，截圖對照。
- [x] `cargo build` + 執行 + 寫 `CHANGELOG` 條目。
- [ ] 打包試 `tools/package-windows.ps1`（確認 NSIS 中文語言檔、字型有入包）。

---

## 八、測試與品質保證

| 檢查 | 工具／方式 | 失敗時 |
|---|---|---|
| 缺翻譯 | `python tools/extract-i18n.py --check`（CI `i18n` job） | 列出缺句，退出碼 1 |
| placeholder 對唔上 | `tools/check-lang.py`（比對 `{...}` 集合） | 退出碼 1，列出句子 |
| 亂碼／重複譯／空白譯文 | `tools/check-lang.py` | 退出碼 1 |
| Rust 表同 TSV 是否一致 | `tools/generate-lang.py --check` | 退出碼 1，提示跑 `--fix` |
| 字型覆蓋（每個譯文字符都有字） | `src/lang/mod.rs` 測試（讀字型 cmap）＋ `tools/subset-cjk.py --check` | `cargo test` 失敗 |
| 中文語言選擇 | 手工 + 自動化 UI 截圖 | — |
| 系統偵測 `zh-HK` | 設 `LANG=zh_HK.UTF-8` 啟動 | — |
| 指令面板中文搜尋 | `src/ui_palette.rs` 測試（三語） | `cargo test` 失敗 |

**一次過跑齊**：`python tools/check-i18n.py`（`--fix` 會先生成 Rust 表再檢查）。

> 注意：`cargo test` 現時有 7 個失敗測試（`doc`、`editor`、`hof`、`signals`、`timetable`），呢啲係**上游本身**嘅失敗（缺少 OMSI 地圖資料，例如 Spandau 107 條巴士路線 0 條成功），同翻譯無關：改動前嘅基線一樣失敗。

---

## 九、風險與對策

| 風險 | 影響 | 對策 |
|---|---|---|
| 忘記加字型 → 全豆腐格 | 致命 | Phase 2 提前做；`src/lang/mod.rs` 測試檢查字型覆蓋 |
| 上游出新版改咗英文原文 | 該句回落顯示英文 | `extract-i18n.py --check` 會即刻發現；`lookup()` fallback 顯示英文唔會 panic |
| 上游改 `i18n.rs` 結構 | 衝突 | 只改一個檔、diff 細；定期 `git merge upstream/main` 試撞 |
| 翻譯風格不一致 | 品質下降 | §6 詞彙表 + 每批 review；`tools/check-lang.py` 檢查重複詞 |
| 繁簡字形／港台用字爭議 | 觀感差 | 全程用繁體 + 香港用語；`香港補充字符` 由 HKSCS 清單覆蓋 |
| 安裝包體積暴增 | 使用者不滿 | **已解決**：字型子集化（16 MB → 735 kB），完整字型唔入 package |
| 字型子集漏咗用戶自己寫嘅字 | 手寫地圖名有豆腐格 | 子集含粵語常用字 800 個 + HKSCS；要更多就調 `tools/subset-cjk.py` 嘅 `COMMON_COUNT` |
| 翻譯量太大（1,974 句） | 進度慢 | 分批 + 檢查強迫補齊；每批可獨立交付（**已完成**） |

---

## 十、同上游嘅關係（重要）

- 本計劃**唔改** `tr()` 嘅簽名，所以 2,464 個呼叫點零改動 → 同上游 `git merge` 衝突面極窄（只有 `src/i18n.rs`、`src/icons.rs`、`src/ui.rs`、`src/ui_palette.rs`、`src/descriptions.rs` 五個檔）。
- 值得**上游提案**：將「翻譯表 + 查表」改成官方支援多語言嘅架構（對其他貢獻者亦有幫助）。做法係將 `src/lang/` 同一套設計提交上游，之後中文表可以直接留喺自己 fork。
- 每次上游 bump（例如 v0.1.6）之後，標準動作：
  1. `git merge upstream/main`
  2. `python tools/check-i18n.py` → 列出新加／改咗嘅英文 key
  3. 補翻譯（通常每次 5–40 句），放喺 `tools/i18n/zh_hk*.tsv`
  4. `python tools/check-i18n.py --fix`（生成 Rust 表）
  5. `cargo test`

---

## 十一、附錄：關鍵檔案速查

| 檔案 | 行數 | 責任 |
|---|---|---|
| `src/i18n.rs` | ~175 | 語言 enum、`tr()`、`tr_words()`、系統偵測 |
| `src/lang/mod.rs` | 新增 | 模組入口；字型覆蓋檢查（讀字型 cmap） |
| `src/lang/zh_hk.rs` | 新增（自動生成） | 繁中翻譯表（2,109 條）＋ 77 條搜尋詞 |
| `src/icons.rs` | ~180 | 字型掛載（Material Symbols + CJK 子集） |
| `src/ui.rs` | 2,717 | 主選單、語言選單（`lang_menu` @479） |
| `src/ui_palette.rs` | ~380 | `cmd!` 宏、77 條命令、搜尋 |
| `src/descriptions.rs` | ~180 | 地圖描述多語言（`LANGS` @14，含 `ZHT`） |
| `src/geo/fetch.rs` | ~320 | Nominatim `accept-language`（自動變 `zh-HK`） |
| `src/app.rs` | 3,175 | 語言套用（@157）、偏好寫入（@1246） |
| `src/install.rs` | ~310 | `Prefs.language`（@48） |
| `assets/fonts/NotoSansCJKtc-Subset.otf` | 新增 | 中文字型子集（735 kB） |
| `tools/i18n/zh_hk*.tsv` | 新增 | 譯文（英文<TAB>繁中），翻譯員改呢啲 |
| `tools/extract-i18n.py` | 新增 | 抽 key、產生模板、查缺漏 |
| `tools/generate-lang.py` | 新增 | 由 TSV 生成 `src/lang/zh_hk.rs` |
| `tools/check-lang.py` | 新增 | 亂碼／placeholder／空白／重複檢查 |
| `tools/check-i18n.py` | 新增 | 一次過跑齊所有檢查 |
| `tools/fetch-chars.py` | 新增 | 拉常用繁中字符表（粵語 + HKSCS） |
| `tools/subset-cjk.py` | 新增 | 字型子集化 |
| `tools/show-lang.py` | 新增 | 隨機抽睇譯文（校對用） |
| `.github/workflows/ci.yml` | 現有 | `i18n` job：跑 `tools/check-i18n.py` |

---

## 十二、預期成果

- `openMapper` 介面完整繁體中文（香港），無英文殘留、無豆腐格。
- 中文指令面板搜尋。
- 系統 `zh-HK` 自動識別。
- 地圖可出 `global_ZHT.dsc`。
- 翻譯獨立成檔，日後上游更新維護成本低（每次約 0.5–1 小時）。
- 架構可推廣到簡體中文／日文（只需再加一個語言 enum + 一個表）。

---

## 十三、工作記錄（2026-10-25）

**已完成**（commit `305a4e5`，branch `feat/i18n-zh-hk`）：

- 全部 2,109 句 `tr()` 文字已有繁中譯文，`tools/check-i18n.py` 全綠。
- 字型已子集化（735 kB）：`src/lang/mod.rs` 有三個測試——
  - `the_font_file_is_read_as_it_should`（字型讀得到）、
  - `the_chinese_font_has_every_character_of_the_translations`（每個譯文字符都有字形）、
  - `egui_shows_every_character_of_the_translations`（**經 egui 嘅 fallback 鏈**，中文真係畫得出，冇豆腐格）。
- 指令面板 77 條命令都有中文搜尋詞（三語測試通過）；語言切換命令而家會輪轉英文→俄文→繁中。
- `cargo build` 無警告；`cargo test` 307 passed / 7 failed（7 個係上游缺地圖資料嘅既有失敗，改動前基線一樣失敗）。
- `CHANGELOG.md` 已加條目；CI 加咗 `i18n` job；NSIS 加咗 `TradChinese`；macOS `Info.plist` 加咗 `zh-HK`。
- 程式啟動時會檢查字型覆蓋，有漏字會寫入 log（`missing_glyphs()`）。

**未完成（下次繼續）**：

1. 肉眼校對譯文（用 `python tools/show-lang.py --count 30` 抽睇），特別係教學長文 `ui_guide.rs`。
2. `tools/package-windows.ps1` 打包試（本機冇 NSIS，睇 CI 嘅 `Installer installs and uninstalls` job）：確認字型入咗 package。
3. `global_ZHT.dsc` 寫出後，用 openOMSI launcher 讀返確認（要裝 openOMSI）。
4. 考慮上游提案：`src/lang/` 呢套「翻譯表 + 查表」設計。

