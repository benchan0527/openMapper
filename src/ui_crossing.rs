//! The crossing path editor: the `[path]`s of a crossing object of the map, drawn from above
//! and edited - where they start and end, how wide, how steep, which way, which light they
//! wait at, which paths they block - then written back into a copy of the object.
//!
//! It does not build a crossing (the complex junction editor does): it changes one that is
//! already on the map, as OMSI 2's own `OmsiObjEditP` does, without touching the original
//! `.sco` in the installation.

use egui::{Color32, Pos2, RichText, Sense, Stroke, Vec2};
use glam::{DVec2, DVec3};

use crate::assets::Assets;
use crate::crossing_edit::{self, Crossing, Helper, Path, Sheet, TURN_LEFT, TURN_NONE, TURN_RIGHT};
use crate::doc::Sel;
use crate::editor::Editor;
use crate::i18n::tr;
use crate::icons::{self, Icon};
use crate::install::Roots;
use crate::library::Library;

const STREET: Color32 = Color32::from_rgb(232, 190, 96);
const FOOTWAY: Color32 = Color32::from_rgb(104, 184, 224);
const RAIL: Color32 = Color32::from_rgb(196, 148, 208);
const PICKED: Color32 = Color32::from_rgb(255, 148, 40);
const HANDLE: Color32 = Color32::from_rgb(238, 238, 238);
const SNAPPED: Color32 = Color32::from_rgb(96, 208, 128);
const DIM: Color32 = Color32::from_gray(150);
const BAD: Color32 = Color32::from_rgb(240, 110, 96);
const GOOD: Color32 = Color32::from_rgb(120, 200, 130);

/// How near a click must be to an end to take it (pixels).
const GRAB: f32 = 9.0;

/// The crossing path editor's window and its state.
pub struct CrossingPaths {
    pub open: bool,
    /// The object being edited.
    target: Option<Sel>,
    /// The map it was opened on, and the `.sco` the object named then.
    map_dir: std::path::PathBuf,
    from_rel: String,
    /// The `.sco` the map names, and where it is.
    rel: String,
    abs: std::path::PathBuf,
    /// The bytes it was read from: the code page it is written back in.
    bytes: Vec<u8>,
    sheet: Option<Sheet>,
    /// The crossing as it was when opened (Revert).
    seen: Crossing,
    /// The path being edited, and the end being dragged of it.
    selected: Option<usize>,
    drag: Option<(usize, bool)>,
    /// The canvas: pixels per metre, and the point of the object at its centre.
    scale: f32,
    centre: DVec2,
    fit: bool,
    /// The file this window writes is already a copy the editor made (so it writes in place).
    copied: bool,
    message: Option<(String, bool)>,
    /// The page shown: the paths, or the traffic light program (as OMSI's two editors).
    tab: Tab,
    /// The light and the phase picked in the light program, and the block edge being dragged.
    light: Option<usize>,
    phase: Option<usize>,
    edge: Option<(usize, usize)>,
    /// Pixels a second takes on the timeline.
    sec_px: f32,
    /// Whether the timeline follows the window's width (the whole cycle in view).
    sec_fit: bool,
    /// Crossing objects of the map, to pick from when nothing is selected.
    candidates: Vec<((i32, i32), i64, String)>,
    /// The room the drawing side was given, so a test can say it is not squeezed out.
    #[cfg(test)]
    last_canvas: Vec2,
    /// Where the timeline was drawn, so a test can click a block in it.
    #[cfg(test)]
    last_timeline: Option<egui::Rect>,
    /// Where the window itself was drawn, so a test can say its body is not collapsed.
    #[cfg(test)]
    last_window: Option<egui::Rect>,
}

impl Default for CrossingPaths {
    fn default() -> Self {
        CrossingPaths {
            open: false,
            target: None,
            rel: String::new(),
            abs: Default::default(),
            bytes: Vec::new(),
            sheet: None,
            seen: Crossing::default(),
            map_dir: Default::default(),
            from_rel: String::new(),
            selected: None,
            drag: None,
            scale: 14.0,
            centre: DVec2::ZERO,
            fit: true,
            copied: false,
            message: None,
            tab: Tab::Paths,
            light: None,
            phase: None,
            edge: None,
            sec_px: 14.0,
            sec_fit: true,
            candidates: Vec::new(),
            #[cfg(test)]
            last_canvas: Vec2::ZERO,
            #[cfg(test)]
            last_timeline: None,
            #[cfg(test)]
            last_window: None,
        }
    }
}

/// Where the mark that a `.sco` is this editor's own copy sits.
fn sidecar(abs: &std::path::Path) -> std::path::PathBuf {
    abs.with_extension("crossing.json")
}

fn kind_color(kind: i32) -> Color32 {
    match kind {
        1 => FOOTWAY,
        2 => RAIL,
        _ => STREET,
    }
}

fn kind_name(kind: i32) -> &'static str {
    match kind {
        1 => tr("Footway", "Тротуар"),
        2 => tr("Rail", "Рельсы"),
        _ => tr("Street", "Проезжая часть"),
    }
}

fn direction_name(d: i32) -> &'static str {
    match d {
        1 => tr("Against", "Против"),
        2 => tr("Both ways", "В обе стороны"),
        _ => tr("Along", "По направлению"),
    }
}

fn turn_name(t: i32) -> &'static str {
    match t {
        TURN_LEFT => tr("Left", "Налево"),
        TURN_RIGHT => tr("Right", "Направо"),
        _ => tr("None", "Нет"),
    }
}

/// What a `[phase]`'s state shows (the game's own groups).
fn aspect_name(state: i32) -> &'static str {
    match state {
        0..=2 => tr("Red", "Красный"),
        3..=5 => tr("Red and yellow", "Красный и жёлтый"),
        6 | 7 => tr("Green", "Зелёный"),
        8 => tr("Green and yellow", "Зелёный и жёлтый"),
        9..=11 => tr("Yellow", "Жёлтый"),
        _ => tr("Off", "Выключен"),
    }
}

/// The state's colour, as the crossing editor's timeline shows it.
fn aspect_color(state: i32) -> Color32 {
    match state {
        0..=2 => Color32::from_rgb(198, 56, 56),
        3..=5 => Color32::from_rgb(210, 146, 52),
        6 | 7 => Color32::from_rgb(62, 150, 74),
        8 => Color32::from_rgb(142, 168, 58),
        9..=11 => Color32::from_rgb(206, 192, 62),
        _ => Color32::from_rgb(70, 72, 70),
    }
}

/// The state as the game's own table writes it (`R` red, `Ro` red and yellow, `G` green,
/// `G+Y` green and yellow, `Y` yellow, `A` off, `B` blinking and `+A` with an arrow).
fn aspect_symbol(state: i32) -> &'static str {
    match state {
        0 => "R",
        1 => "R+B",
        2 => "R+P",
        3 => "Ro",
        4 => "Ro+B",
        5 => "Ro+P",
        6 => "G",
        7 => "G+B",
        8 => "G+Y",
        9 => "Y",
        10 => "Y+B",
        11 => "Y+P",
        _ => "A",
    }
}

/// Which page of the window is shown.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Paths,
    Lights,
}

/// The timeline's room: what the row names take on the left, and the second marks on top.
const GUTTER: f32 = 118.0;
const HEAD: f32 = 18.0;
const ROW_H: f32 = 19.0;

fn unsaved_text() -> &'static str {
    tr("There are unsaved changes here: Save or Revert first", "Здесь есть несохранённые правки: сначала сохраните или отмените их")
}

impl CrossingPaths {
    fn unsaved(&self) -> bool {
        self.sheet.as_ref().is_some_and(|s| s.crossing != self.seen)
    }

    /// Open on the selected object, or on a crossing picked from the map's list.
    pub fn request(&mut self, ed: &mut Editor, roots: &Roots, lib: &Library) {
        // (what is being edited is not dropped for another object)
        if self.unsaved() && !matches!((ed.selected, self.target), (Some(a), Some(b)) if a == b) {
            self.open = true;
            self.message = Some((unsaved_text().into(), true));
            return;
        }
        self.sheet = None;
        if let Some(Sel::Object(k, id)) = ed.selected {
            if self.open_on(ed, roots, k, id) {
                self.open = true;
                return;
            }
        }
        self.candidates.clear();
        if let Some(doc) = ed.doc.as_ref() {
            for (k, t) in &doc.tiles {
                for o in &t.tile.objects {
                    if self.candidates.len() >= 300 {
                        break;
                    }
                    let crossing = lib.find(&o.file).map(|a| a.category == crate::library::Category::Junctions).unwrap_or(false) || crate::editor::is_own_junction(&o.file);
                    if crossing {
                        self.candidates.push((*k, o.id, o.file.clone()));
                    }
                }
            }
        }
        self.open = true;
        if self.message.is_none() {
            self.message = Some((tr("Select a crossing object, or pick one of these", "Выберите объект перекрёстка или возьмите один из этих").into(), false));
        }
    }

    /// Which page the window shows when it opens (checks of the light program's page).
    pub fn open_page(&mut self, lights: bool) {
        self.tab = if lights { Tab::Lights } else { Tab::Paths };
    }

    /// Whether a crossing is open (a check run says so before it asks for the map's list).
    pub fn has_sheet(&self) -> bool {
        self.sheet.is_some()
    }

    /// Open the first crossing of the map's own list (a check run, with no one to pick one).
    pub fn open_first_candidate(&mut self, ed: &mut Editor, roots: &Roots) -> bool {
        let Some((k, id)) = self.candidates.first().map(|(k, id, _)| (*k, *id)) else { return false };
        self.open_on(ed, roots, k, id)
    }

    /// Read the object's `.sco` for editing. False (and a message) when it has no paths.
    fn open_on(&mut self, ed: &Editor, roots: &Roots, k: (i32, i32), id: i64) -> bool {
        let Some(doc) = ed.doc.as_ref() else { return false };
        let Some(o) = doc.object(k, id) else { return false };
        let rel = o.file.clone();
        if let Some(why) = crossing_edit::why_not(roots, &rel) {
            log::info!("crossing editor: {rel} cannot be edited: {why}");
            self.message = Some((format!("{}: {}", &rel, why), true));
            self.sheet = None;
            return false;
        }
        match crossing_edit::open(roots, &rel) {
            Ok((abs, bytes, sheet)) => {
                self.copied = sidecar(&abs).is_file();
                self.target = Some(Sel::Object(k, id));
                self.map_dir = doc.dir.clone();
                self.from_rel = rel.clone();
                self.rel = rel;
                self.abs = abs;
                self.bytes = bytes;
                self.seen = sheet.crossing.clone();
                self.selected = (!sheet.crossing.paths.is_empty()).then_some(0);
                self.sheet = Some(sheet);
                self.fit = true;
                self.drag = None;
                self.message = None;
                true
            }
            Err(e) => {
                log::info!("crossing editor: {rel} cannot be read: {e}");
                self.message = Some((e.to_string(), true));
                self.sheet = None;
                false
            }
        }
    }

    /// The window.
    pub fn window(&mut self, ctx: &egui::Context, ed: &mut Editor, lib: &Library, roots: Option<&Roots>, assets: Option<&mut Assets>, actions: &mut Vec<crate::ui::Action>) {
        let _ = (lib, actions);
        if !self.open {
            if ed.crossing_draft.is_some() {
                ed.crossing_draft = None;
            }
            return;
        }
        let mut open = self.open;
        if self.sheet.is_some() {
            // the map shows the paths as they are here, not as the file still has them
            if let (Some(Sel::Object(k, id)), Some(sheet)) = (self.target, self.sheet.as_ref()) {
                ed.crossing_draft = Some((k, id, sheet.crossing.paths.clone()));
            }
        } else {
            ed.crossing_draft = None;
        }
        let screen = ctx.content_rect();
        let size = Vec2::new(1000.0, 660.0);
        // (in the middle of the screen, as the crossing editor's own window is)
        let pos = screen.center() - size * 0.5;
        let mut save = false;
        let mut revert = false;
        let mut assets = assets;
        // (each page named as the crossing editor's own window names it)
        let title = match self.tab {
            Tab::Paths => icons::labelled(Icon::Route, tr("Crossing paths", "Пути перекрёстка")),
            Tab::Lights => icons::labelled(Icon::Traffic, tr("Traffic lights", "Светофоры")),
        };
        let shown = egui::Window::new(title)
            .id(egui::Id::new("crossing paths window"))
            .open(&mut open)
            .default_pos(pos)
            .default_size(size)
            .min_size(Vec2::new(880.0, 540.0))
            .resizable(true)
            .show(ctx, |ui| {
                // (a scroll area takes the layout of the ui it is made in, so each half gets
                // its own top-down layout: put straight into the row below, the left side
                // would lay its labels out sideways and push the view off the window)
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
                let body = Vec2::new(ui.available_width().max(800.0), ui.available_height().max(440.0));
                ui.set_min_size(body);
                let Some(mut sheet) = self.sheet.take() else {
                    self.no_target(ui, ed, roots);
                    return;
                };
                self.head(ui, &sheet, &mut save, &mut revert);                ui.separator();
                let body = Vec2::new(ui.available_width().max(800.0), ui.available_height().max(400.0));
                match self.tab {
                    Tab::Paths => self.paths_body(ui, ed, &mut sheet, body),
                    Tab::Lights => self.lights_body(ui, ed, &mut sheet, body),
                }
                self.sheet = Some(sheet);
            });
        #[cfg(test)]
        if let Some(s) = &shown {
            self.last_window = Some(s.response.rect);
        }
        #[cfg(not(test))]
        let _ = &shown;
        // (closed with edits in it: they are not dropped without a word)
        if !open && self.unsaved() {
            open = true;
            self.message = Some((unsaved_text().into(), true));
        }
        self.open = open;
        if !self.open {
            ed.crossing_draft = None;
        }
        if revert {
            if let Some(sheet) = self.sheet.as_mut() {
                sheet.crossing = self.seen.clone();
            }
            self.drag = None;
            self.edge = None;
            self.message = Some((tr("Back to how it was opened", "Обратно к состоянию при открытии").into(), false));
        }
        if save {
            if let (Some(roots), Some(assets)) = (roots, assets.as_deref_mut()) {
                self.save(ed, roots, assets);
            }
        }
    }

    /// Above both pages: what is open, the two pages, and saving - as the crossing editor's
    /// own title bar carries them.
    fn head(&mut self, ui: &mut egui::Ui, sheet: &Sheet, save: &mut bool, revert: &mut bool) {
        let name = std::path::Path::new(&self.rel.replace('\\', "/")).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| self.rel.clone());
        let copied = self.copied;
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).strong());
            ui.label(RichText::new(if copied { tr("openMapper's copy", "Копия openMapper") } else { tr("in its own file (a copy is made on saving)", "в своём файле (копия делается при сохранении)") }).small().color(DIM));
            ui.separator();
            let paths = format!("{} ({})", tr("Paths", "Пути"), sheet.crossing.paths.len());
            let lights = format!("{} ({})", tr("Traffic lights", "Светофоры"), sheet.crossing.lights.len());
            if ui.selectable_label(self.tab == Tab::Paths, paths).clicked() {
                self.tab = Tab::Paths;
            }
            if ui.selectable_label(self.tab == Tab::Lights, lights).clicked() {
                self.tab = Tab::Lights;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if copied { tr("Save", "Сохранить") } else { tr("Save as a copy", "Сохранить копию") };
                let hint = if copied {
                    tr("Write the paths into openMapper's copy of the object", "Записать пути в копию объекта openMapper")
                } else {
                    tr("The object's folder is copied into openMapper's, the paths written there, and the map's object pointed at it: the original file is not touched", "Папка объекта копируется в openMapper, пути пишутся туда, объект карты начинает ссылаться на неё: исходный файл не трогается")
                };
                *save = ui.button(icons::labelled(Icon::Save, label)).on_hover_text(hint).clicked();
                *revert = ui.button(icons::labelled(Icon::Undo, tr("Revert", "Отменить"))).on_hover_text(tr("Back to how the object was when it was opened (the file is not touched)", "Обратно к тому, каким объект был при открытии (файл не трогается)")).clicked();
            });
        });
        if let Some((msg, bad)) = &self.message {
            ui.label(RichText::new(msg).small().color(if *bad { BAD } else { GOOD }));
        }
    }

    /// The paths page: the list and its numbers on the left, the crossing drawn on the right.
    fn paths_body(&mut self, ui: &mut egui::Ui, ed: &mut Editor, sheet: &mut Sheet, body: Vec2) {
        let (left, rest) = (320.0f32, (body.x - 334.0).max(400.0));
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(left, body.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::ScrollArea::vertical().id_salt("crossing side").auto_shrink([false, false]).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(left - 18.0);
                        self.paths_side(ui, ed, sheet);
                    });
                });
            });
            ui.separator();
            ui.allocate_ui_with_layout(Vec2::new(rest, body.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                self.view_bar(ui);
                self.canvas_ui(ui, sheet);
            });
        });
    }

    /// The light page: the program on the left, its timeline on the right - the crossing
    /// editor's traffic light window.
    fn lights_body(&mut self, ui: &mut egui::Ui, _ed: &mut Editor, sheet: &mut Sheet, body: Vec2) {
        let (left, rest) = (352.0f32, (body.x - 366.0).max(400.0));
        let side = left - 20.0;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(left, body.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_max_width(side);
                // (a scroll area so a long request-loop or light list is never out of reach)
                egui::ScrollArea::vertical().id_salt("crossing lights side").auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_max_width(side);
                    self.lights_side(ui, sheet, side);
                });
            });
            ui.separator();
            ui.allocate_ui_with_layout(Vec2::new(rest, body.y), egui::Layout::top_down(egui::Align::Min), |ui| {
                self.timeline_ui(ui, sheet);
            });
        });
    }

    fn no_target(&mut self, ui: &mut egui::Ui, ed: &mut Editor, roots: Option<&Roots>) {
        ui.add_space(4.0);
        ui.label(RichText::new(tr("Pick a crossing object of the map", "Возьмите объект перекрёстка на карте")).strong());
        ui.label(RichText::new(tr("The paths, the blocking and the traffic light program of an existing crossing are edited here. The object is copied into openMapper's own folder; the original file is left as it is.", "Здесь правят пути, блокировки и программу светофора готового перекрёстка. Объект копируется в папку openMapper, исходный файл остаётся как был.")).small().color(DIM));
        if let Some((msg, bad)) = &self.message {
            ui.colored_label(if *bad { BAD } else { DIM }, msg);
        }
        ui.add_space(6.0);
        let mut pick = None;
        egui::ScrollArea::vertical().max_height((ui.available_height() - 40.0).max(80.0)).show(ui, |ui| {
            for (k, id, rel) in &self.candidates {
                let name = std::path::Path::new(&rel.replace('\\', "/")).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| rel.clone());
                if ui.selectable_label(false, name).on_hover_text(format!("{rel}\n{}, {}", k.0, k.1)).clicked() {
                    pick = Some((*k, *id));
                }
            }
        });
        if let (Some((k, id)), Some(roots)) = (pick, roots) {
            if self.open_on(ed, roots, k, id) {
                self.message = None;
            }
        }
    }

    /// The paths page's left side: the list, and the numbers of the one picked.
    fn paths_side(&mut self, ui: &mut egui::Ui, ed: &mut Editor, sheet: &mut Sheet) {
        let Some(Sel::Object(..)) = self.target else { return };
        let mut changed = false;
        // ---- the paths
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} ({})", tr("Paths", "Пути"), sheet.crossing.paths.len())).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(icons::labelled(Icon::Add, "")).on_hover_text(tr("Add a path", "Добавить путь")).clicked() {
                    let mut p = Path::default();
                    if let Some(i) = self.selected {
                        if let Some(q) = sheet.crossing.paths.get(i) {
                            p = q.shifted(q.width().max(1.0));
                        }
                    }
                    sheet.crossing.paths.push(p);
                    self.selected = Some(sheet.crossing.paths.len() - 1);
                    changed = true;
                }
                if ui.small_button(icons::labelled(Icon::Copy, "")).on_hover_text(tr("Duplicate", "Дублировать")).clicked() {
                    if let Some(q) = self.selected.and_then(|i| sheet.crossing.paths.get(i)).cloned() {
                        sheet.crossing.paths.push(q);
                        self.selected = Some(sheet.crossing.paths.len() - 1);
                        changed = true;
                    }
                }
            });
        });
        let mut remove = None;
        let mut move_to = None;
        egui::ScrollArea::vertical().id_salt("crossing paths").max_height(150.0).auto_shrink([false, true]).show(ui, |ui| {
            for i in 0..sheet.crossing.paths.len() {
                let p = &sheet.crossing.paths[i];
                let mut text = format!("#{i}  {}  {:.1} m", kind_name(p.kind()), p.length());
                if p.turn() != TURN_NONE {
                    text.push_str(&format!("  {}", turn_name(p.turn())));
                }
                if let Some(l) = p.light {
                    text.push_str(&format!("  L{l}"));
                }
                if !p.blocks.is_empty() {
                    text.push_str(&format!("  ⛔{}", p.blocks.len()));
                }
                let on = self.selected == Some(i);
                ui.horizontal(|ui| {
                    if ui.selectable_label(on, RichText::new(text).color(kind_color(p.kind()))).clicked() {
                        self.selected = Some(i);
                    }
                    if i > 0 && ui.small_button("↑").on_hover_text(tr("Move up", "Выше")).clicked() {
                        move_to = Some((i, i - 1));
                    }
                    if i + 1 < sheet.crossing.paths.len() && ui.small_button("↓").on_hover_text(tr("Move down", "Ниже")).clicked() {
                        move_to = Some((i, i + 1));
                    }
                    if ui.small_button(icons::labelled(Icon::Delete, "")).on_hover_text(tr("Remove this path", "Удалить этот путь")).clicked() {
                        remove = Some(i);
                    }
                });
            }
        });
        if let Some((from, to)) = move_to {
            sheet.crossing.move_path(from, to);
            self.selected = Some(to);
            changed = true;
        }
        if let Some(i) = remove {
            sheet.crossing.remove_path(i);
            self.selected = if sheet.crossing.paths.is_empty() { None } else { Some(i.min(sheet.crossing.paths.len() - 1)) };
            changed = true;
        }

        // ---- the numbers of the picked path
        if let Some(i) = self.selected.filter(|i| *i < sheet.crossing.paths.len()) {
            ui.separator();
            let crossing = &mut sheet.crossing;
            let p = &mut crossing.paths[i];
            egui::Grid::new("crossing numbers").num_columns(2).spacing([8.0, 3.0]).show(ui, |ui| {
                ui.label(RichText::new(tr("Start", "Начало")).small().color(DIM));
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut p.v[0]).speed(0.05).prefix("x ")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut p.v[1]).speed(0.05).prefix("y ")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut p.v[2]).speed(0.05).prefix("z ")).changed();
                });
                ui.end_row();
                ui.label(RichText::new(tr("Heading", "Направление")).small().color(DIM));
                let mut h = p.heading();
                if ui.add(egui::DragValue::new(&mut h).speed(0.5).suffix("°")).changed() {
                    p.v[3] = h as f32;
                    changed = true;
                }
                ui.end_row();
                ui.label(RichText::new(tr("Radius", "Радиус")).small().color(DIM));
                let mut r = p.radius();
                if ui.add(egui::DragValue::new(&mut r).speed(0.1).suffix(" m")).on_hover_text(tr("0 is a straight line, above 0 turns right, below 0 left", "0 — прямая, больше 0 — направо, меньше 0 — налево")).changed() {
                    p.v[4] = r as f32;
                    changed = true;
                }
                ui.end_row();
                ui.label(RichText::new(tr("Length", "Длина")).small().color(DIM));
                let mut l = p.length();
                if ui.add(egui::DragValue::new(&mut l).speed(0.1).range(0.0..=2000.0).suffix(" m")).changed() {
                    p.v[5] = l as f32;
                    changed = true;
                }
                ui.end_row();
                ui.label(RichText::new(tr("Slope", "Уклон")).small().color(DIM));
                ui.horizontal(|ui| {
                    let mut a = p.slope_start() * 100.0;
                    let mut b = p.slope_end() * 100.0;
                    if ui.add(egui::DragValue::new(&mut a).speed(0.05).suffix(" %")).changed() {
                        p.v[6] = (a / 100.0) as f32;
                        changed = true;
                    }
                    if ui.add(egui::DragValue::new(&mut b).speed(0.05).suffix(" %")).changed() {
                        p.v[7] = (b / 100.0) as f32;
                        changed = true;
                    }
                });
                ui.end_row();
                ui.label(RichText::new(tr("Kind", "Тип")).small().color(DIM));
                let mut k = p.v[8] as i32;
                let before = k;
                egui::ComboBox::from_id_salt("crossing kind").selected_text(kind_name(k)).width(150.0).show_ui(ui, |ui| {
                    for n in [0, 1, 2] {
                        ui.selectable_value(&mut k, n, kind_name(n));
                    }
                });
                p.v[8] = k as f32;
                changed |= k != before;
                ui.end_row();
                ui.label(RichText::new(tr("Width", "Ширина")).small().color(DIM));
                changed |= ui.add(egui::DragValue::new(&mut p.v[9]).speed(0.05).range(0.0..=30.0).suffix(" m")).changed();
                ui.end_row();
                ui.label(RichText::new(tr("Direction", "Направление движения")).small().color(DIM));
                let mut d = p.v[10] as i32;
                let before = d;
                egui::ComboBox::from_id_salt("crossing direction").selected_text(direction_name(d)).width(150.0).show_ui(ui, |ui| {
                    for n in [0, 1, 2] {
                        ui.selectable_value(&mut d, n, direction_name(n));
                    }
                });
                p.v[10] = d as f32;
                changed |= d != before;
                ui.end_row();
                ui.label(RichText::new(tr("Turn signal", "Поворот")).small().color(DIM));
                let mut t = p.v[11] as i32;
                let before = t;
                egui::ComboBox::from_id_salt("crossing turn").selected_text(turn_name(t)).width(150.0).show_ui(ui, |ui| {
                    for n in [TURN_NONE, TURN_LEFT, TURN_RIGHT] {
                        ui.selectable_value(&mut t, n, turn_name(n));
                    }
                });
                p.v[11] = t as f32;
                changed |= t != before;
                ui.end_row();
                ui.label(RichText::new(tr("Light", "Светофор")).small().color(DIM));
                let names: Vec<(i32, String)> = std::iter::once((-1, tr("None", "Нет").to_string())).chain(crossing.lights.iter().enumerate().map(|(n, l)| (n as i32, format!("#{n} {}", if l.name.trim().is_empty() { "-" } else { l.name.trim() })))).collect();
                let mut cur = p.light.unwrap_or(-1);
                let before = cur;
                egui::ComboBox::from_id_salt("crossing light").selected_text(names.iter().find(|(n, _)| *n == cur).map(|(_, s)| s.clone()).unwrap_or_else(|| tr("None", "Нет").into())).width(150.0).show_ui(ui, |ui| {
                    for (n, s) in &names {
                        ui.selectable_value(&mut cur, *n, s);
                    }
                });
                if cur != before {
                    p.light = (cur >= 0).then_some(cur);
                    changed = true;
                }
                ui.end_row();
            });
            changed |= ui.checkbox(&mut sheet.crossing.paths[i].crossing_problem, tr("Keeps the junction clear", "Держит перекрёсток свободным")).on_hover_text(tr("A vehicle waiting on this path holds the crossing empty for the others (`[crossingproblem]`)", "Машина на этом пути держит перекрёсток свободным для других (`[crossingproblem]`)")).changed();
            // blocking: which other paths wait while this one is taken
            let others: Vec<(usize, String)> = (0..sheet.crossing.paths.len()).filter(|j| *j != i).map(|j| (j, format!("#{j} {}", kind_name(sheet.crossing.paths[j].kind())))).collect();
            if !others.is_empty() {
                let n = sheet.crossing.paths[i].blocks.len();
                egui::CollapsingHeader::new(RichText::new(format!("{} ({n})", tr("Blocks", "Блокирует"))).small()).id_salt("crossing blocks").default_open(n > 0).show(ui, |ui| {
                    for (j, label) in &others {
                        let mut on = sheet.crossing.paths[i].blocks.iter().any(|(a, _)| *a == *j as i32);
                        if ui.checkbox(&mut on, label).changed() {
                            let p = &mut sheet.crossing.paths[i];
                            p.blocks.retain(|(a, _)| *a != *j as i32);
                            if on {
                                p.blocks.push((*j as i32, 0));
                            }
                            changed = true;
                        }
                    }
                });
            }
        }

        // ---- the spline helpers
        ui.separator();
        let mut helper_gone = None;
        egui::CollapsingHeader::new(RichText::new(format!("{} ({})", tr("Spline helpers", "Точки привязки сплайнов"), sheet.crossing.helpers.len())).strong()).id_salt("crossing helpers").default_open(false).show(ui, |ui| {
            ui.label(RichText::new(tr("Where a road of the map snaps onto the crossing. x, height, forward in the object's frame", "Где дорога карты пристыковывается к перекрёстку. x, высота, вперёд в системе объекта")).small().color(DIM));
            for (n, h) in sheet.crossing.helpers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("#{n}")).small().color(DIM));
                    changed |= ui.add(egui::TextEdit::singleline(&mut h.spline).desired_width(120.0)).on_hover_text(tr("The road type it takes (a `.sli`)", "Тип дороги, который он принимает (`.sli`)")).changed();
                    if ui.small_button(icons::labelled(Icon::Delete, "")).clicked() {
                        helper_gone = Some(n);
                    }
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut h.x).speed(0.05).prefix("x ")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut h.fwd).speed(0.05).prefix("f ")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut h.up).speed(0.05).prefix("h ")).changed();
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut h.heading).speed(0.5).suffix("°")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut h.length).speed(0.05).prefix("len ")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut h.radius).speed(0.1).prefix("r ")).changed();
                });
            }
            if let Some(n) = helper_gone {
                sheet.crossing.helpers.remove(n);
                changed = true;
            }
            ui.horizontal(|ui| {
                if ui.button(icons::labelled(Icon::Add, tr("A helper", "Точка"))).clicked() {
                    let spline = ed.road_type.clone().unwrap_or_default();
                    sheet.crossing.helpers.push(Helper { spline, x: 0.0, up: 0.0, fwd: 0.0, heading: 0.0, length: 0.1, radius: 0.0 });
                    changed = true;
                }
                if ui.button(tr("At the picked end", "У выбранного конца")).on_hover_text(tr("A helper where the picked path ends, facing out of the crossing", "Точка там, где кончается выбранный путь, наружу от перекрёстка")).clicked() {
                    if let Some(p) = self.selected.and_then(|i| sheet.crossing.paths.get(i)) {
                        let at = p.end();
                        let c = p.curve();
                        let heading = c.heading_at(c.length) as f32;
                        let spline = ed.road_type.clone().unwrap_or_default();
                        sheet.crossing.helpers.push(Helper { spline, x: at.x as f32, up: at.z as f32, fwd: at.y as f32, heading, length: 0.1, radius: 0.0 });
                        changed = true;
                    }
                }
            });
        });

        if changed {
            // (nothing to do: the view and the map both draw the sheet as it is each frame;
            // the file is only written on Save)
            let _ = changed;
        }
    }

    /// Above the drawing: the zoom.
    fn view_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("−").on_hover_text(tr("Zoom out", "Отдалить")).clicked() {
                self.scale = (self.scale / 1.35).clamp(0.6, 400.0);
            }
            if ui.button("+").on_hover_text(tr("Zoom in", "Приблизить")).clicked() {
                self.scale = (self.scale * 1.35).clamp(0.6, 400.0);
            }
            if ui.button(tr("Fit", "Вписать")).on_hover_text(tr("The whole crossing in view", "Весь перекрёсток в окне")).clicked() {
                self.fit = true;
            }
            ui.label(RichText::new(tr("Drag an end to move it. Ctrl snaps it to another path's end. Wheel zooms, right or middle drag pans", "Тяните конец, чтобы сдвинуть его. Ctrl притягивает к концу другого пути. Колесо — масштаб, правая/средняя кнопка — сдвиг")).small().color(DIM));
        });
    }

    /// The light page's left side: which light, which of its phases, what that phase shows,
    /// the detector and the request loops - as the crossing editor's traffic light window.
    fn lights_side(&mut self, ui: &mut egui::Ui, sheet: &mut Sheet, width: f32) {
        if self.light.is_some_and(|l| l >= sheet.crossing.lights.len()) {
            self.light = None;
            self.phase = None;
        }
        if self.light.is_none() && !sheet.crossing.lights.is_empty() {
            self.light = Some(0);
        }
        let count = sheet.crossing.lights.len();

        // ---- which light, its name and the cycle it runs in (the window's own top box)
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width(width - 12.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Traffic light", "Светофор")).strong());
                let shown = self.light.map(|i| i + 1).unwrap_or(0);
                ui.label(RichText::new(format!("{shown} / {count}")).monospace().small().color(DIM));
                if ui.add_enabled(self.light.is_some_and(|i| i > 0), egui::Button::new("<").small()).clicked() {
                    self.light = self.light.map(|i| i - 1);
                    self.phase = Some(0);
                }
                if ui.add_enabled(self.light.is_some_and(|i| i + 1 < count), egui::Button::new(">").small()).clicked() {
                    self.light = self.light.map(|i| i + 1);
                    self.phase = Some(0);
                }
                if ui.small_button(icons::labelled(Icon::Add, "")).on_hover_text(tr("A light", "Светофор")).clicked() {
                    self.light = Some(sheet.crossing.add_light());
                    self.phase = Some(0);
                }
                if ui.add_enabled(self.light.is_some(), egui::Button::new(icons::labelled(Icon::Delete, "")).small()).on_hover_text(tr("Remove this light", "Удалить этот светофор")).clicked() {
                    if let Some(i) = self.light {
                        sheet.crossing.remove_light(i);
                        self.light = if sheet.crossing.lights.is_empty() { None } else { Some(i.min(sheet.crossing.lights.len() - 1)) };
                        self.phase = Some(0);
                    }
                }
            });
            if let Some(i) = self.light {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Name", "Имя")).small().color(DIM));
                    ui.add(egui::TextEdit::singleline(&mut sheet.crossing.lights[i].name).desired_width(ui.available_width()));
                });
                ui.horizontal(|ui| {
                    let mut on = sheet.crossing.has_cycle();
                    if ui.checkbox(&mut on, tr("Cycle", "Цикл")).on_hover_text(tr("`[traffic_lights_group]`: the seconds of one run. Unset, the longest light's phases add up to it", "`[traffic_lights_group]`: секунды одного круга. Если не задан — сумма фаз самой длинной программы")).changed() {
                        sheet.crossing.cycle = if on { Some(sheet.crossing.phases_sum().max(1.0)) } else { None };
                    }
                    let len = sheet.crossing.cycle_len();
                    if let Some(c) = sheet.crossing.cycle.as_mut() {
                        ui.add(egui::DragValue::new(c).speed(0.5).range(1.0..=600.0).suffix(" s"));
                    } else {
                        ui.label(RichText::new(format!("{len:.0} s")).monospace().small().color(DIM));
                    }
                });
            } else if count == 0 {
                ui.label(RichText::new(tr("A crossing with lights needs the signal heads on the map: the game only runs a program that heads are linked to", "Перекрёстку со светофором нужны головки на карте: игра запускает программу, только если к ней привязаны головки")).small().color(DIM));
            }
        });

        // ---- the phase and what it shows (the window's "Traffic Light Phase" box)
        ui.add_space(3.0);
        if let Some(i) = self.light {
            let n = sheet.crossing.lights[i].phases.len();
            if self.phase.is_some_and(|p| p >= n) {
                self.phase = if n > 0 { Some(n - 1) } else { None };
            }
            if self.phase.is_none() && n > 0 {
                self.phase = Some(0);
            }
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_width(width - 12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("Phase", "Фаза")).strong());
                    ui.label(RichText::new(format!("{} / {n}", self.phase.map(|p| p + 1).unwrap_or(0))).monospace().small().color(DIM));
                    if ui.add_enabled(self.phase.is_some_and(|p| p > 0), egui::Button::new("<").small()).clicked() {
                        self.phase = self.phase.map(|p| p - 1);
                    }
                    if ui.add_enabled(self.phase.is_some_and(|p| p + 1 < n), egui::Button::new(">").small()).clicked() {
                        self.phase = self.phase.map(|p| p + 1);
                    }
                    if ui.small_button(icons::labelled(Icon::Add, "")).on_hover_text(tr("A phase", "Фаза")).clicked() {
                        let at = self.phase.map(|p| p + 1).unwrap_or(n);
                        self.phase = sheet.crossing.insert_phase(i, at);
                    }
                    if ui.add_enabled(self.phase.is_some(), egui::Button::new(icons::labelled(Icon::Delete, "")).small()).on_hover_text(tr("Delete", "Удалить")).clicked() {
                        if let Some(p) = self.phase {
                            sheet.crossing.remove_phase(i, p);
                            let n = sheet.crossing.lights[i].phases.len();
                            self.phase = if n == 0 { None } else { Some(p.min(n - 1)) };
                        }
                    }
                });
                if let Some(p) = self.phase {
                    let mut state = sheet.crossing.lights[i].phases[p].state;
                    // (the states laid out three to a row, as the crossing editor's own list)
                    egui::Grid::new("crossing aspects").num_columns(3).spacing([8.0, 0.0]).max_col_width(width / 3.0 - 12.0).show(ui, |ui| {
                        for code in 0..=12 {
                            let label = RichText::new(format!("{code} {}", aspect_symbol(code))).small().color(aspect_color(code));
                            ui.radio_value(&mut state, code, label).on_hover_text(format!("{} ({})", aspect_name(code), aspect_symbol(code)));
                            if code % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                    if state != sheet.crossing.lights[i].phases[p].state {
                        sheet.crossing.lights[i].phases[p].state = state;
                    }
                    let mut det = sheet.crossing.lights[i].approach_dist;
                    let mut on = det.is_some();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("Duration", "Длительность")).small().color(DIM));
                        ui.add(egui::DragValue::new(&mut sheet.crossing.lights[i].phases[p].duration).speed(0.2).range(0.0..=900.0).suffix(" s"));
                        ui.separator();
                        if ui.checkbox(&mut on, tr("Detector", "Мелдер")).on_hover_text(tr("`[approachdist]`: how far before the stop line a vehicle registers a request", "`[approachdist]`: за сколько метров до стоп-линии машина запрашивает")).changed() {
                            det = if on { Some(det.unwrap_or(10.0).max(0.0)) } else { None };
                        }
                        if let Some(d) = det.as_mut() {
                            ui.add(egui::DragValue::new(d).speed(0.2).range(0.0..=200.0).suffix(" m"));
                        }
                    });
                    sheet.crossing.lights[i].approach_dist = det;
                    let sum = sheet.crossing.phases_sum();
                    let len = sheet.crossing.cycle_len();
                    if (sum - len).abs() > 0.5 {
                        ui.label(RichText::new(tr("The phases do not add up to the cycle: the last one holds to its end", "Фазы не складываются в цикл: последняя длится до его конца")).small().color(DIM));
                    }
                }
            });
        }

        // ---- the lights of the crossing, as the window's own list
        ui.add_space(3.0);
        egui::ScrollArea::vertical().id_salt("crossing light list").max_height(96.0).auto_shrink([false, true]).show(ui, |ui| {
            for (k, l) in sheet.crossing.lights.iter().enumerate() {
                let name = if l.name.trim().is_empty() { tr("(no name)", "(без имени)") } else { l.name.trim() };
                let color = l.phases.first().map(|p| aspect_color(p.state)).unwrap_or(DIM);
                if ui.selectable_label(self.light == Some(k), RichText::new(format!("{k}: {name}")).color(color)).clicked() {
                    self.light = Some(k);
                    self.phase = Some(0);
                }
            }
        });

        // ---- the request loops, as the numbers they are
        egui::CollapsingHeader::new(RichText::new(format!("{} ({})", tr("Request loops", "Петли запроса"), sheet.crossing.stops.len() + sheet.crossing.jumps.len())).small()).id_salt("crossing stops").default_open(false).show(ui, |ui| {
            let mut stop_gone = None;
            for (n, s) in sheet.crossing.stops.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("stop", "стоп")).small().color(DIM));
                    ui.add(egui::DragValue::new(&mut s[0]).speed(0.2).prefix("L"));
                    ui.add(egui::DragValue::new(&mut s[1]).speed(0.2).suffix(" s"));
                    ui.add(egui::DragValue::new(&mut s[2]).range(0.0..=1.0).speed(0.1).prefix("req "));
                    if ui.small_button(icons::labelled(Icon::Delete, "")).clicked() {
                        stop_gone = Some(n);
                    }
                });
            }
            if let Some(n) = stop_gone {
                sheet.crossing.stops.remove(n);
            }
            let mut jump_gone = None;
            for (n, j) in sheet.crossing.jumps.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr("jump", "переход")).small().color(DIM));
                    ui.add(egui::DragValue::new(&mut j[0]).speed(0.2).prefix("L"));
                    ui.add(egui::DragValue::new(&mut j[1]).speed(0.2).suffix(" s"));
                    ui.add(egui::DragValue::new(&mut j[2]).range(0.0..=1.0).speed(0.1).prefix("req "));
                    ui.add(egui::DragValue::new(&mut j[3]).speed(0.2).prefix("→ ").suffix(" s"));
                    if ui.small_button(icons::labelled(Icon::Delete, "")).clicked() {
                        jump_gone = Some(n);
                    }
                });
            }
            if let Some(n) = jump_gone {
                sheet.crossing.jumps.remove(n);
            }
        });
    }

    /// The light program's timeline: a row per light, a column per second, each phase a block
    /// of its colour, the phase's number written in it - the traffic light window's grid.
    fn timeline_ui(&mut self, ui: &mut egui::Ui, sheet: &mut Sheet) {
        let cycle = sheet.crossing.cycle_len();
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Timeline", "Временная диаграмма")).strong());
            ui.label(RichText::new(format!("{cycle:.0} s")).monospace().small().color(DIM));
            if ui.small_button("−").on_hover_text(tr("Zoom out", "Отдалить")).clicked() {
                self.sec_px = (self.sec_px / 1.25).clamp(3.0, 80.0);
                self.sec_fit = false;
            }
            if ui.small_button("+").on_hover_text(tr("Zoom in", "Приблизить")).clicked() {
                self.sec_px = (self.sec_px * 1.25).clamp(3.0, 80.0);
                self.sec_fit = false;
            }
            if ui.small_button(tr("Fit", "Вписать")).on_hover_text(tr("The whole cycle across the width (as the traffic light window has it)", "Весь цикл по ширине (как в окне светофора)")).clicked() {
                self.sec_fit = true;
            }
        });
        // (a second per pixel width the whole cycle fits in, until the zoom is used: the
        // window shows the program end to end, as OMSI's own does)
        if self.sec_fit && cycle > 0.0 {
            let room = (ui.available_width() - GUTTER - 24.0).max(120.0);
            self.sec_px = (room / cycle).clamp(2.0, 40.0);
        }
        ui.label(RichText::new(tr("Click a block to edit that phase; drag its right edge to change how long it lasts", "Нажмите на блок, чтобы править эту фазу; потяните его правый край, чтобы изменить её длительность")).small().color(DIM));

        let rows = sheet.crossing.lights.len();
        let size = Vec2::new((GUTTER + cycle * self.sec_px + 18.0).max(ui.available_width()), HEAD + rows.max(1) as f32 * ROW_H + 2.0);
        egui::ScrollArea::both().id_salt("crossing timeline").auto_shrink([false, false]).show(ui, |ui| {
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
            #[cfg(test)]
            {
                self.last_timeline = Some(rect);
            }
            let p = ui.painter_at(rect);
            let x = |t: f32| rect.left() + GUTTER + t * self.sec_px;
            let top = rect.top() + HEAD;
            p.rect_filled(rect, egui::CornerRadius::ZERO, Color32::from_rgb(26, 28, 26));
            p.rect_filled(egui::Rect::from_min_max(rect.min, Pos2::new(rect.right(), top)), egui::CornerRadius::ZERO, Color32::from_rgb(38, 41, 38));

            // the seconds: a line each, a number every five
            let tick = if self.sec_px >= 9.0 { 1.0 } else if self.sec_px >= 4.5 { 2.0 } else { 5.0 };
            let mut t = 0.0f32;
            while t <= cycle + 0.01 {
                let major = (t % 5.0).abs() < 1e-3;
                let xa = x(t);
                p.line_segment([Pos2::new(xa, rect.top()), Pos2::new(xa, rect.bottom())], Stroke::new(1.0, Color32::from_white_alpha(if major { 42 } else { 14 })));
                if major && self.sec_px >= 7.0 || (t % 10.0).abs() < 1e-3 {
                    p.text(Pos2::new(xa + 2.0, rect.top() + 2.0), egui::Align2::LEFT_TOP, format!("{t:.0}"), egui::FontId::monospace(9.0), DIM);
                }
                t += tick;
            }

            // the lights, one row each
            let segs = sheet.crossing.timeline();
            for i in 0..rows {
                let y0 = top + i as f32 * ROW_H;
                let row = egui::Rect::from_min_max(Pos2::new(rect.left(), y0), Pos2::new(rect.right(), y0 + ROW_H));
                if i % 2 == 1 {
                    p.rect_filled(row, egui::CornerRadius::ZERO, Color32::from_white_alpha(6));
                }
                if self.light == Some(i) {
                    p.rect_filled(egui::Rect::from_min_max(row.min, Pos2::new(rect.left() + GUTTER, row.bottom())), egui::CornerRadius::ZERO, Color32::from_rgb(46, 54, 48));
                }
                p.line_segment([Pos2::new(rect.left(), row.bottom()), Pos2::new(rect.right(), row.bottom())], Stroke::new(1.0, Color32::from_white_alpha(18)));
                let l = &sheet.crossing.lights[i];
                let name = if l.name.trim().is_empty() { "-" } else { l.name.trim() };
                p.text(Pos2::new(rect.left() + 5.0, y0 + ROW_H / 2.0), egui::Align2::LEFT_CENTER, format!("{i}  {name}"), egui::FontId::monospace(9.0), if self.light == Some(i) { HANDLE } else { DIM });
                p.line_segment([Pos2::new(rect.left() + GUTTER, y0), Pos2::new(rect.left() + GUTTER, y0 + ROW_H)], Stroke::new(1.0, Color32::from_white_alpha(28)));
                for s in segs.iter().filter(|s| s.light == i) {
                    let block = egui::Rect::from_min_max(
                        Pos2::new(x(s.start), y0 + 1.0),
                        Pos2::new(x(s.end).max(x(s.start) + 1.0), y0 + ROW_H - 1.0),
                    );
                    let fill = aspect_color(s.state);
                    p.rect_filled(block, egui::CornerRadius::ZERO, fill);
                    if s.own_end < s.end - 0.001 {
                        // (the last phase holds to the cycle's end: that tail is paler)
                        p.rect_filled(egui::Rect::from_min_max(Pos2::new(x(s.own_end), block.top()), block.max), egui::CornerRadius::ZERO, fill.gamma_multiply(0.55));
                    }
                    let on = self.light == Some(i) && self.phase == Some(s.phase);
                    if on {
                        p.rect_stroke(block, egui::CornerRadius::ZERO, Stroke::new(1.4, HANDLE), egui::StrokeKind::Inside);
                    }
                    if block.width() >= 13.0 {
                        p.text(block.center(), egui::Align2::CENTER_CENTER, s.phase.to_string(), egui::FontId::monospace(9.0), Color32::from_black_alpha(190));
                    }
                }
            }
            p.rect_stroke(rect, egui::CornerRadius::ZERO, Stroke::new(1.0, Color32::from_white_alpha(30)), egui::StrokeKind::Inside);

            // ---- the mouse
            let time_at = |pos: Pos2| ((pos.x - rect.left() - GUTTER) / self.sec_px).clamp(0.0, cycle);
            let row_at = |pos: Pos2| -> Option<usize> {
                if pos.y < top {
                    return None;
                }
                let i = ((pos.y - top) / ROW_H).floor() as usize;
                (i < rows).then_some(i)
            };
            let edge_near = |pos: Pos2| -> Option<(usize, usize)> {
                let i = row_at(pos)?;
                let near = segs.iter().find(|s| s.light == i && (x(s.end) - pos.x).abs() <= 4.0)?;
                Some((i, near.phase))
            };
            if resp.hovered() && resp.hover_pos().and_then(edge_near).is_some() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if resp.drag_started_by(egui::PointerButton::Primary) {
                self.edge = None;
                // (where the press went down: by the time a drag starts the pointer has moved)
                let origin = ui.input(|i| i.pointer.press_origin()).or(resp.interact_pointer_pos());
                if let Some(pos) = origin {
                    if let Some(i) = row_at(pos) {
                        if pos.x < rect.left() + GUTTER {
                            self.light = Some(i);
                            self.phase = Some(0);
                        } else if let Some((l, ph)) = edge_near(pos) {
                            // (on a block's edge: that phase is the one being dragged)
                            self.light = Some(l);
                            self.phase = Some(ph);
                            self.edge = Some((l, ph));
                        } else if let Some(ph) = sheet.crossing.phase_at(i, time_at(pos)) {
                            self.light = Some(i);
                            self.phase = Some(ph);
                        }
                    }
                }
            }
            if resp.dragged_by(egui::PointerButton::Primary) {
                if let (Some((l, ph)), Some(pos)) = (self.edge, resp.interact_pointer_pos()) {
                    sheet.crossing.resize_phase(l, ph, time_at(pos));
                }
            }
            if resp.drag_stopped_by(egui::PointerButton::Primary) {
                self.edge = None;
            }
            if resp.clicked() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    if let Some(i) = row_at(pos) {
                        self.light = Some(i);
                        if pos.x < rect.left() + GUTTER {
                            self.phase = Some(0);
                        } else if let Some(ph) = sheet.crossing.phase_at(i, time_at(pos)) {
                            self.phase = Some(ph);
                        }
                    }
                }
            }
        });
    }

    fn save(&mut self, ed: &mut Editor, roots: &Roots, assets: &mut Assets) {
        let Some(Sel::Object(k, id)) = self.target else { return };
        let Some(sheet) = self.sheet.as_ref() else { return };
        let crossing = sheet.crossing.clone();
        // (the object may not be what this was opened on any more: another map, or an undo that
        // put the original back, which then gets the copy again)
        let now = ed.doc.as_ref().filter(|d| d.dir == self.map_dir).and_then(|d| d.object(k, id)).map(|o| o.file.clone());
        let on_ours = now.as_deref().is_some_and(|f| f.eq_ignore_ascii_case(&self.rel));
        let on_original = now.as_deref().is_some_and(|f| f.eq_ignore_ascii_case(&self.from_rel));
        if !on_ours && !on_original {
            self.message = Some((tr("The map has changed since this was opened: open the object again", "Карта изменилась с момента открытия: откройте объект заново").into(), true));
            return;
        }
        if crossing.helpers.iter().any(|h| h.spline.trim().is_empty()) {
            self.message = Some((tr("A spline helper has no road type: pick one or remove it", "У точки привязки нет типа дороги: выберите его или удалите точку").into(), true));
            return;
        }
        let text = sheet.text();
        let fresh = !self.copied;
        let (rel, abs) = if self.copied {
            (self.rel.clone(), self.abs.clone())
        } else {
            let map = ed.doc.as_ref().map(crate::junction::folder_name).unwrap_or_else(|| "map".into());
            let stem = std::path::Path::new(&self.rel.replace('\\', "/")).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "crossing".into());
            match crossing_edit::copy_into(roots, &self.rel, &map, &stem) {
                Ok(r) => {
                    let a = omsi_cfg::resolve_path(&roots.content, &r);
                    (r, a)
                }
                Err(e) => {
                    self.message = Some((e.to_string(), true));
                    return;
                }
            }
        };
        if let Err(e) = crossing_edit::write_checked(&abs, &text, &self.bytes, &crossing) {
            if fresh {
                crossing_edit::discard_copy(roots, &rel);
            }
            self.message = Some((e.to_string(), true));
            return;
        }
        // (the mark that this file is the editor's own: the next save writes it in place)
        let _ = std::fs::write(sidecar(&abs), format!("{{\"from\": {:?}}}", self.from_rel));
        self.copied = true;
        self.rel = rel.clone();
        self.abs = abs.clone();
        if let Ok(b) = std::fs::read(&abs) {
            self.bytes = b;
        }
        self.seen = crossing;
        crate::hot::own_change();
        assets.objects.remove(&rel.to_lowercase());
        let _ = assets.object(&rel);
        if fresh || !on_ours {
            ed.set_object_file(k, id, &rel);
        }
        self.message = Some((tr("Saved", "Сохранено").into(), false));
    }

    // ------------------------------------------------------------ the view

    fn fit_to(&mut self, size: Vec2, sheet: &Sheet) {
        let mut lo = DVec2::splat(f64::MAX);
        let mut hi = DVec2::splat(f64::MIN);
        for p in &sheet.crossing.paths {
            for q in p.points(1.0) {
                lo = lo.min(q.truncate());
                hi = hi.max(q.truncate());
            }
        }
        for h in &sheet.crossing.helpers {
            lo = lo.min(h.local().truncate());
            hi = hi.max(h.local().truncate());
        }
        if lo.x > hi.x {
            lo = DVec2::splat(-15.0);
            hi = DVec2::splat(15.0);
        }
        let centre = (lo + hi) / 2.0;
        let span = (hi - lo).max(DVec2::splat(12.0)) * 1.25;
        let s = (size.x as f64 / span.x).min(size.y as f64 / span.y);
        self.centre = centre;
        self.scale = (s as f32).clamp(0.6, 400.0);
    }

    fn canvas_ui(&mut self, ui: &mut egui::Ui, sheet: &mut Sheet) {
        let size = Vec2::new(ui.available_width().max(320.0), ui.available_height().max(320.0));
        #[cfg(test)]
        {
            self.last_canvas = size;
        }
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        if self.fit {
            self.fit = false;
            self.fit_to(rect.size(), sheet);
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            let pinch = ui.input(|i| i.zoom_delta());
            ui.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
            let factor = (1.0 + scroll * 0.0025).max(0.2) * pinch;
            if factor != 1.0 {
                if let Some(h) = resp.hover_pos() {
                    let (scale, centre) = (self.scale, self.centre);
                    let before = DVec2::new(centre.x + (h.x - rect.center().x) as f64 / scale as f64, centre.y - (h.y - rect.center().y) as f64 / scale as f64);
                    self.scale = (self.scale * factor).clamp(0.6, 400.0);
                    let after = DVec2::new(centre.x + (h.x - rect.center().x) as f64 / self.scale as f64, centre.y - (h.y - rect.center().y) as f64 / self.scale as f64);
                    self.centre += before - after;
                }
            }
        }
        if resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) {
            let d = resp.drag_delta();
            self.centre -= DVec2::new(d.x as f64, -d.y as f64) / self.scale as f64;
        }
        let (scale, centre) = (self.scale, self.centre);
        let to_screen = move |p: DVec2| Pos2::new(rect.center().x + ((p.x - centre.x) * scale as f64) as f32, rect.center().y - ((p.y - centre.y) * scale as f64) as f32);
        let from_screen = move |q: Pos2| DVec2::new(centre.x + (q.x - rect.center().x) as f64 / scale as f64, centre.y - (q.y - rect.center().y) as f64 / scale as f64);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3.0, Color32::from_rgb(30, 34, 31));
        // the grid: metres close, tens away
        {
            let step = if scale >= 14.0 { 1.0 } else if scale >= 4.0 { 5.0 } else if scale >= 1.2 { 10.0 } else { 50.0 };
            let (lo, hi) = (from_screen(rect.left_bottom()), from_screen(rect.right_top()));
            let line = |a: DVec2, b: DVec2, zero: bool, major: bool| painter.line_segment([to_screen(a), to_screen(b)], Stroke::new(if zero { 1.5 } else { 1.0 }, Color32::from_white_alpha(if zero { 70 } else if major { 26 } else { 12 })));
            let mut x = (lo.x / step).floor() * step;
            while x <= hi.x {
                line(DVec2::new(x, lo.y), DVec2::new(x, hi.y), x == 0.0, (x / (step * 5.0)).fract().abs() < 1e-6);
                x += step;
            }
            let mut y = (lo.y / step).floor() * step;
            while y <= hi.y {
                line(DVec2::new(lo.x, y), DVec2::new(hi.x, y), y == 0.0, (y / (step * 5.0)).fract().abs() < 1e-6);
                y += step;
            }
        }
        // the object's own frame: x to the right, y (forward) up the page
        painter.line_segment([to_screen(DVec2::new(-3.0, 0.0)), to_screen(DVec2::new(3.0, 0.0))], Stroke::new(1.4, Color32::from_rgb(150, 90, 90)));
        painter.line_segment([to_screen(DVec2::new(0.0, -3.0)), to_screen(DVec2::new(0.0, 3.0))], Stroke::new(1.4, Color32::from_rgb(90, 150, 90)));

        // the paths
        for (i, p) in sheet.crossing.paths.iter().enumerate() {
            let on = self.selected == Some(i);
            let base = if on { PICKED } else { kind_color(p.kind()) };
            let pts = p.points(0.5);
            // its edges, so the width is seen
            let half = (p.width() / 2.0).max(0.0);
            let mut left = Vec::with_capacity(pts.len());
            let mut right = Vec::with_capacity(pts.len());
            for (n, q) in pts.iter().enumerate() {
                let t = if n + 1 < pts.len() { pts[n + 1] - *q } else if n > 0 { *q - pts[n - 1] } else { DVec3::Y };
                let t = t.truncate();
                let len = t.length();
                let tangent = if len > 1e-9 { t / len } else { DVec2::Y };
                let normal = DVec2::new(tangent.y, -tangent.x);
                left.push(q.truncate() + normal * half);
                right.push(q.truncate() - normal * half);
            }
            let edge = Stroke::new(1.0, base.gamma_multiply(0.55));
            for w in left.windows(2) {
                painter.line_segment([to_screen(w[0]), to_screen(w[1])], edge);
            }
            for w in right.windows(2) {
                painter.line_segment([to_screen(w[0]), to_screen(w[1])], edge);
            }
            for w in pts.windows(2) {
                painter.line_segment([to_screen(w[0].truncate()), to_screen(w[1].truncate())], Stroke::new(if on { 2.0 } else { 1.4 }, base));
            }
            // the way it goes
            let dir_arrow = |at: DVec2, along: DVec2, color: Color32| {
                let len = along.length();
                if len < 1e-6 {
                    return;
                }
                let d = along / len;
                let n = DVec2::new(-d.y, d.x);
                let tip = at + d * (10.0 / scale as f64);
                painter.add(egui::Shape::convex_polygon(vec![to_screen(tip), to_screen(at + n * (4.0 / scale as f64)), to_screen(at - n * (4.0 / scale as f64))], color, Stroke::NONE));
            };
            if let (Some(a), Some(b)) = (pts.first(), pts.last()) {
                let mid = (a.truncate() + b.truncate()) / 2.0;
                match p.direction() {
                    1 => dir_arrow(mid, a.truncate() - b.truncate(), base),
                    2 => {
                        dir_arrow(mid, b.truncate() - a.truncate(), base);
                        dir_arrow(mid, a.truncate() - b.truncate(), base);
                    }
                    _ => dir_arrow(mid, b.truncate() - a.truncate(), base),
                }
            }
            // its ends
            for at in [p.start().truncate(), p.end().truncate()] {
                let s = to_screen(at);
                painter.circle_filled(s, if on { 4.0 } else { 3.0 }, if on { HANDLE } else { base });
                painter.circle_stroke(s, 6.0, Stroke::new(1.0, Color32::from_white_alpha(if on { 120 } else { 40 })));
            }
        }
        // the spline helpers: where roads snap on
        for h in &sheet.crossing.helpers {
            let at = to_screen(h.local().truncate());
            painter.circle_stroke(at, 5.0, Stroke::new(1.6, SNAPPED));
            let d = crate::roads::dir(h.heading as f64);
            painter.line_segment([at, to_screen(h.local().truncate() + d * 4.0)], Stroke::new(1.4, SNAPPED));
        }

        // ---- the mouse
        let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
        let at = |pos: Pos2| -> DVec2 { from_screen(pos) };
        let handle = |sheet: &Sheet, pos: Pos2| -> Option<(usize, bool)> {
            let mut best: Option<(f32, usize, bool)> = None;
            for (i, p) in sheet.crossing.paths.iter().enumerate() {
                for (at_end, q) in [(true, p.end()), (false, p.start())] {
                    let d = to_screen(q.truncate()).distance(pos);
                    if d <= GRAB && best.map(|b| d < b.0).unwrap_or(true) {
                        best = Some((d, i, at_end));
                    }
                }
            }
            best.map(|b| (b.1, b.2))
        };
        let path_at = |sheet: &Sheet, pos: Pos2| -> Option<usize> {
            let mut best: Option<(f32, usize)> = None;
            for (i, p) in sheet.crossing.paths.iter().enumerate() {
                for q in p.points(0.5) {
                    let d = to_screen(q.truncate()).distance(pos);
                    if d <= GRAB && best.map(|b| d < b.0).unwrap_or(true) {
                        best = Some((d, i));
                    }
                }
            }
            best.map(|b| b.1)
        };
        if resp.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = resp.interact_pointer_pos() {
                self.drag = handle(sheet, pos);
                if self.drag.is_none() {
                    self.selected = path_at(sheet, pos).or(self.selected);
                }
            }
        }
        if resp.dragged_by(egui::PointerButton::Primary) {
            if let (Some((i, at_end)), Some(pos)) = (self.drag, resp.interact_pointer_pos()) {
                if i < sheet.crossing.paths.len() {
                    let mut q = at(pos);
                    let mut snapped: Option<f64> = None;
                    if ctrl {
                        if let Some((j, other_end, hit)) = sheet.crossing.nearest_end(DVec3::new(q.x, q.y, 0.0), crate::lanes::LINK, i) {
                            q = hit.truncate();
                            // (the way out of the other path is the way out of this one)
                            let c = sheet.crossing.paths[j].curve();
                            snapped = Some(if other_end { c.heading_at(c.length) } else { c.heading_at(0.0) });
                        }
                    }
                    let p = &mut sheet.crossing.paths[i];
                    if at_end {
                        p.set_end(DVec3::new(q.x, q.y, p.end().z));
                        if let (Some(out), true) = (snapped, p.radius().abs() < 1e-6) {
                            // (arriving along it: the same way round)
                            p.v[3] = crate::roads::norm_deg(out + 180.0) as f32;
                            let chord = (p.end().truncate() - p.start().truncate()).length();
                            p.v[5] = chord as f32;
                        }
                    } else {
                        p.set_start(DVec3::new(q.x, q.y, p.start().z));
                        if let (Some(out), true) = (snapped, p.radius().abs() < 1e-6) {
                            let end = p.end();
                            p.v[3] = crate::roads::norm_deg(out) as f32;
                            p.set_end(end);
                        }
                    }
                }
            }
        }
        if resp.drag_stopped_by(egui::PointerButton::Primary) {
            self.drag = None;
        }
        if resp.clicked() && self.drag.is_none() {
            if let Some(pos) = resp.interact_pointer_pos() {
                self.selected = path_at(sheet, pos);
            }
        }
        if resp.hovered() {
            if let Some(pos) = resp.hover_pos() {
                if handle(sheet, pos).is_some() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            }
        }
        // the picked path's ends, again, over everything: these are what is dragged
        if let Some(i) = self.selected.filter(|i| *i < sheet.crossing.paths.len()) {
            let p = &sheet.crossing.paths[i];
            for at in [p.start().truncate(), p.end().truncate()] {
                painter.circle_stroke(to_screen(at), 9.0, Stroke::new(1.6, PICKED));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A crossing with a few paths, so the left side has a list to lay out.
    const SCO: &str = "\
[friendlyname]
Kreuzung

[mesh]
Kreuzung.o3d

[path]
-6.5
-12
0.1
90
0
12
0
0
0
3.5
0
3

[path]
6.5
-12
0.1
90
0
12
0
0
0
3.5
0
2

[path]
0
0
0
0
0
10
0
0
0
3.5
2
0

[use_traffic_light]
0

[traffic_lights_group]
60

[traffic_light]
Main
[phase]
0
30
[phase]
6
25
";

    /// The window gives the properties the left third and the drawing the rest: a scroll area
    /// takes the layout it is made in, so laid straight into the row it would put the whole
    /// path list sideways in one line and push the drawing out of the window (which is what
    /// the window looked like: one row of paths and nothing else).
    #[test]
    fn the_window_puts_the_drawing_beside_the_properties() {
        let mut w = opened(Tab::Paths);
        let mut ed = Editor::default();
        let ctx = context();
        run(&ctx, &mut w, &mut ed, vec![], 0.0);
        run(&ctx, &mut w, &mut ed, vec![], 0.1);
        let canvas = w.last_canvas;
        assert!(canvas.x > 400.0, "the drawing got {canvas:?}: the properties are laid out beside it");
        assert!(canvas.y > 300.0, "the drawing got {canvas:?}");
        // and the map is shown the paths as the window has them
        let draft = ed.crossing_draft.as_ref().expect("the map shows the draft");
        assert_eq!(draft.2.len(), 3);
    }

    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_fonts(icons::fonts());
        ctx
    }

    /// With nothing open the window still shows the picker, whole: a window collapsed to its
    /// title bar would be a blank grey strip.
    #[test]
    fn the_window_asks_for_a_crossing_when_none_is_open() {
        let mut w = CrossingPaths { open: true, ..Default::default() };
        let mut ed = Editor::default();
        let ctx = context();
        run(&ctx, &mut w, &mut ed, vec![], 0.0);
        run(&ctx, &mut w, &mut ed, vec![], 0.1);
        let rect = w.last_window.expect("the window is drawn");
        assert!(rect.height() > 200.0, "the window's body is collapsed: {rect:?}");
        assert!(rect.width() > 400.0, "the window is {rect:?}");
    }

    fn opened(tab: Tab) -> CrossingPaths {
        let mut w = CrossingPaths { open: true, ..Default::default() };
        w.target = Some(Sel::Object((0, 0), 1));
        w.rel = "Sceneryobjects\\Test\\Kreuzung.sco".into();
        w.selected = Some(0);
        w.sheet = Some(crate::crossing_edit::Sheet::parse(SCO));
        w.tab = tab;
        w
    }

    fn run(ctx: &egui::Context, w: &mut CrossingPaths, ed: &mut Editor, events: Vec<egui::Event>, time: f64) {
        let lib = Library::default();
        let mut actions = Vec::new();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))),
            time: Some(time),
            events,
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            w.window(&ctx, ed, &lib, None, None, &mut actions);
        });
    }

    fn press(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::default() }
    }

    /// The light page gives the timeline the room beside the program, and draws the phases
    /// the file has: red for 30 s, then green to the end of the cycle.
    #[test]
    fn the_lights_page_lays_the_timeline_beside_the_program() {
        let mut w = opened(Tab::Lights);
        let mut ed = Editor::default();
        let ctx = context();
        run(&ctx, &mut w, &mut ed, vec![], 0.0);
        run(&ctx, &mut w, &mut ed, vec![], 0.1);
        let rect = w.last_timeline.expect("the timeline is drawn");
        assert!(rect.width() > 300.0, "the timeline got {rect:?}");
        assert!(rect.left() > 330.0, "the program keeps the left side: {rect:?}");
        assert!(rect.height() >= HEAD + ROW_H, "a row per light: {rect:?}");
        // the crossing is picked when the page is opened, and its first phase with it
        let sheet = w.sheet.as_ref().unwrap();
        assert_eq!(sheet.crossing.lights.len(), 1);
        assert_eq!(w.light, Some(0));
        assert_eq!(w.phase, Some(0));
        // the cycle's 60 s are on the timeline
        assert!(rect.width() >= GUTTER + 60.0 * w.sec_px - 1.0, "{rect:?}");
    }

    /// A longer cycle still shows end to end: the timeline takes the window's width on its own,
    /// as the traffic light window's does.
    #[test]
    fn a_longer_cycle_still_fits_the_width() {
        let mut w = opened(Tab::Lights);
        let mut ed = Editor::default();
        let ctx = context();
        {
            let c = &mut w.sheet.as_mut().unwrap().crossing;
            c.cycle = Some(240.0);
            for p in &mut c.lights[0].phases {
                p.duration *= 4.0;
            }
        }
        run(&ctx, &mut w, &mut ed, vec![], 0.0);
        run(&ctx, &mut w, &mut ed, vec![], 0.1);
        let rect = w.last_timeline.expect("the timeline is drawn");
        let cycle = w.sheet.as_ref().unwrap().crossing.cycle_len();
        assert!(cycle >= 230.0, "the cycle grew to {cycle} s");
        assert!(
            GUTTER + cycle * w.sec_px <= rect.width() + 1.0,
            "{cycle} s at {} px/s does not fit {rect:?}",
            w.sec_px
        );
    }

    /// Clicking a block picks its light and its phase, as the numbers on the left edit them.
    #[test]
    fn clicking_a_block_picks_that_phase() {
        let mut w = opened(Tab::Lights);
        let mut ed = Editor::default();
        let ctx = context();
        run(&ctx, &mut w, &mut ed, vec![], 0.0);
        run(&ctx, &mut w, &mut ed, vec![], 0.1);
        let rect = w.last_timeline.expect("the timeline is drawn");
        // (the second phase runs from 30 s to 55 s)
        let at = Pos2::new(rect.left() + GUTTER + 40.0 * w.sec_px, rect.top() + HEAD + ROW_H * 0.5);
        run(&ctx, &mut w, &mut ed, vec![egui::Event::PointerMoved(at), press(at, true)], 0.2);
        run(&ctx, &mut w, &mut ed, vec![press(at, false)], 0.3);
        assert_eq!(w.light, Some(0));
        assert_eq!(w.phase, Some(1), "the block at 40 s is the green phase");
        // and the phase of the first block is picked by clicking further left
        let at = Pos2::new(rect.left() + GUTTER + 10.0 * w.sec_px, rect.top() + HEAD + ROW_H * 0.5);
        run(&ctx, &mut w, &mut ed, vec![egui::Event::PointerMoved(at), press(at, true)], 0.4);
        run(&ctx, &mut w, &mut ed, vec![press(at, false)], 0.5);
        assert_eq!(w.phase, Some(0), "the block at 10 s is the red phase");
    }

    /// Dragging the right edge of a block changes that phase's duration, and the ones after it
    /// move along; the file is what it says afterwards.
    #[test]
    fn dragging_a_blocks_edge_changes_its_duration() {
        let mut w = opened(Tab::Lights);
        let mut ed = Editor::default();
        let ctx = context();
        run(&ctx, &mut w, &mut ed, vec![], 0.0);
        run(&ctx, &mut w, &mut ed, vec![], 0.1);
        let rect = w.last_timeline.expect("the timeline is drawn");
        let y = rect.top() + HEAD + ROW_H * 0.5;
        let from = Pos2::new(rect.left() + GUTTER + 30.0 * w.sec_px, y);
        let to = Pos2::new(rect.left() + GUTTER + 20.0 * w.sec_px, y);
        run(&ctx, &mut w, &mut ed, vec![egui::Event::PointerMoved(from), press(from, true)], 0.2);
        run(&ctx, &mut w, &mut ed, vec![egui::Event::PointerMoved(to)], 0.3);
        run(&ctx, &mut w, &mut ed, vec![press(to, false)], 0.4);
        let sheet = w.sheet.as_ref().unwrap();
        assert_eq!(sheet.crossing.lights[0].phases[0].duration, 20.0);
        assert_eq!(sheet.crossing.lights[0].phases[1].duration, 25.0, "the green phase keeps its own");
        assert_eq!(w.edge, None, "the drag has ended");
        // the second phase now starts at 20 s, and still holds to the cycle's end
        let segs = sheet.crossing.segments(0);
        assert_eq!(segs[1].start, 20.0);
        assert_eq!(segs[1].end, 60.0);
    }
}
