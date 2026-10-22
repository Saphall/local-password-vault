use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};

use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Vec2};
use passlock::{
    models::{self, Entry, Strength},
    storage::Vault,
};
use zeroize::Zeroize;

use crate::icons::{self, with_text};

const ACCENT: Color32 = Color32::from_rgb(147, 51, 234);
const ORANGE: Color32 = Color32::from_rgb(249, 115, 22);
const MUTED: Color32 = Color32::from_rgb(130, 138, 155);
const CLIPBOARD_TTL: Duration = Duration::from_secs(30);

pub fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = Color32::from_rgb(11, 15, 26);
    v.window_fill = Color32::from_rgb(17, 22, 36);
    v.extreme_bg_color = Color32::from_rgb(8, 11, 20);
    v.selection.bg_fill = ACCENT;
    v.widgets.hovered.bg_fill = Color32::from_rgb(40, 46, 68);
    ctx.set_visuals(v);
}

#[derive(Clone, PartialEq)]
enum Filter {
    All,
    Favorites,
    Tag(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Sort {
    NameAsc,
    NameDesc,
    Recent,
}

impl Sort {
    fn label(self) -> &'static str {
        match self {
            Sort::NameAsc => "A-Z",
            Sort::NameDesc => "Z-A",
            Sort::Recent => "Recent",
        }
    }
}

enum Action {
    Select(u64),
    ToggleFavorite(u64),
    CopyPassword(u64),
    CopyText(String),
    New,
    Edit(u64),
    AskDelete(u64),
    Lock,
}

struct Editor {
    id: Option<u64>,
    name: String,
    url: String,
    username: String,
    password: String,
    tags: String,
    show_password: bool,
}

impl Editor {
    fn blank() -> Self {
        Editor {
            id: None,
            name: String::new(),
            url: String::new(),
            username: String::new(),
            password: String::new(),
            tags: String::new(),
            show_password: false,
        }
    }

    fn from_entry(e: &Entry) -> Self {
        Editor {
            id: Some(e.id),
            name: e.name.clone(),
            url: e.url.clone(),
            username: e.username.clone(),
            password: e.password.clone(),
            tags: e.tags.join(", "),
            show_password: false,
        }
    }
}

/// Lightweight copy of an entry for the list, so rendering never holds the vault borrowed.
struct Row {
    id: u64,
    name: String,
    username: String,
    url: String,
    tags: Vec<String>,
    favorite: bool,
    strength: Strength,
    updated: i64,
}

pub struct App {
    path: PathBuf,
    vault: Option<Vault>,
    master: String,
    confirm: String,
    error: String,
    filter: Filter,
    sort: Sort,
    search: String,
    selected: Option<u64>,
    show_password: bool,
    shown_history: HashSet<usize>,
    editor: Option<Editor>,
    confirm_delete: Option<u64>,
    clipboard_clear_at: Option<Instant>,
    toast: Option<(String, Instant)>,
}

impl App {
    pub fn new(path: PathBuf) -> Self {
        App {
            path,
            vault: None,
            master: String::new(),
            confirm: String::new(),
            error: String::new(),
            filter: Filter::All,
            sort: Sort::NameAsc,
            search: String::new(),
            selected: None,
            show_password: false,
            shown_history: HashSet::new(),
            editor: None,
            confirm_delete: None,
            clipboard_clear_at: None,
            toast: None,
        }
    }

    fn lock(&mut self) {
        self.vault = None;
        self.selected = None;
        self.editor = None;
        self.confirm_delete = None;
        self.show_password = false;
        self.shown_history.clear();
        self.search.clear();
        self.filter = Filter::All;
        self.error.clear();
    }

    fn copy(&mut self, ctx: &egui::Context, text: String, what: &str) {
        ctx.copy_text(text);
        self.clipboard_clear_at = Some(Instant::now() + CLIPBOARD_TTL);
        self.toast = Some((format!("{what} copied (clears in 30s)"), Instant::now() + Duration::from_secs(3)));
    }

    fn tick(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        if let Some(t) = self.clipboard_clear_at {
            if now >= t {
                ctx.copy_text(String::new());
                self.clipboard_clear_at = None;
            } else {
                ctx.request_repaint_after(t - now);
            }
        }
        if let Some((_, t)) = &self.toast {
            if now >= *t {
                self.toast = None;
            } else {
                ctx.request_repaint_after(*t - now);
            }
        }
    }

    fn save(&mut self) {
        if let Some(v) = &self.vault {
            if let Err(e) = v.save() {
                self.error = e.to_string();
            }
        }
    }

    // ---------- login ----------

    fn login_ui(&mut self, ui: &mut egui::Ui) {
        let creating = !Vault::exists(&self.path);
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(ui.available_height() * 0.25);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(icons::LOCK_KEY).size(56.0).color(ORANGE));
                wordmark(ui, 34.0);
                ui.label(RichText::new("Built with Rust").color(MUTED));
                ui.add_space(20.0);
                ui.label(if creating {
                    "Create a master password for your new vault"
                } else {
                    "Enter your master password"
                });
                ui.add_space(8.0);

                let mut submit = false;
                let r = ui.add(
                    egui::TextEdit::singleline(&mut self.master)
                        .password(true)
                        .hint_text("Master password")
                        .desired_width(280.0),
                );
                submit |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if creating {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.confirm)
                            .password(true)
                            .hint_text("Confirm password")
                            .desired_width(280.0),
                    );
                    submit |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                }
                ui.add_space(8.0);
                let label = if creating { "Create vault" } else { "Unlock" };
                submit |= ui
                    .add(egui::Button::new(RichText::new(with_text(icons::LOCK_KEY_OPEN, label)).color(Color32::WHITE)).fill(ACCENT).min_size(Vec2::new(280.0, 30.0)))
                    .clicked();

                if submit {
                    self.try_login(creating);
                }
                if !self.error.is_empty() {
                    ui.add_space(8.0);
                    ui.colored_label(Color32::LIGHT_RED, with_text(icons::WARNING, &self.error));
                }
            });
        });
    }

    fn try_login(&mut self, creating: bool) {
        self.error.clear();
        let result = if creating {
            if self.master.chars().count() < 8 {
                self.error = "Master password must be at least 8 characters".into();
                return;
            }
            if self.master != self.confirm {
                self.error = "Passwords do not match".into();
                return;
            }
            Vault::create(self.path.clone(), &self.master)
        } else {
            Vault::unlock(self.path.clone(), &self.master)
        };
        match result {
            Ok(v) => {
                self.vault = Some(v);
                self.master.zeroize();
                self.confirm.zeroize();
            }
            Err(e) => self.error = e.to_string(),
        }
    }

    // ---------- main screen ----------

    fn rows(&self) -> Vec<Row> {
        let Some(vault) = &self.vault else { return vec![] };
        let q = self.search.to_lowercase();
        let mut rows: Vec<Row> = vault
            .entries
            .iter()
            .filter(|e| match &self.filter {
                Filter::All => true,
                Filter::Favorites => e.favorite,
                Filter::Tag(t) => e.tags.contains(t),
            })
            .filter(|e| {
                q.is_empty()
                    || e.name.to_lowercase().contains(&q)
                    || e.username.to_lowercase().contains(&q)
                    || e.url.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .map(|e| Row {
                id: e.id,
                name: e.name.clone(),
                username: e.username.clone(),
                url: e.url.clone(),
                tags: e.tags.clone(),
                favorite: e.favorite,
                strength: models::strength(&e.password),
                updated: e.updated,
            })
            .collect();
        match self.sort {
            Sort::NameAsc => rows.sort_by_key(|r| r.name.to_lowercase()),
            Sort::NameDesc => rows.sort_by_key(|r| std::cmp::Reverse(r.name.to_lowercase())),
            Sort::Recent => rows.sort_by_key(|r| std::cmp::Reverse(r.updated)),
        }
        rows
    }

    fn main_ui(&mut self, ui: &mut egui::Ui) {
        let mut actions: Vec<Action> = Vec::new();
        let now = models::now();

        egui::Panel::top("header").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icons::LOCK_KEY).size(22.0).color(ORANGE));
                wordmark(ui, 22.0);
                ui.separator();
                if let Some(v) = &self.vault {
                    ui.label(RichText::new(with_text(icons::VAULT, "Vault file:")).color(MUTED));
                    ui.label(v.path().display().to_string());
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(RichText::new(with_text(icons::LOCK_KEY, "Lock Vault")).color(Color32::LIGHT_RED)))
                        .clicked()
                    {
                        actions.push(Action::Lock);
                    }
                    if let Some((msg, _)) = &self.toast {
                        ui.colored_label(Color32::LIGHT_GREEN, with_text(icons::CHECK, msg));
                    }
                    if !self.error.is_empty() {
                        ui.colored_label(Color32::LIGHT_RED, with_text(icons::WARNING, &self.error));
                    }
                });
            });
        });

        self.sidebar(ui);

        let selected = self
            .selected
            .and_then(|id| self.vault.as_ref()?.entries.iter().find(|e| e.id == id).cloned());
        if let Some(entry) = &selected {
            egui::Panel::right("detail")
                .default_size(300.0)
                .min_size(260.0)
                .show(ui, |ui| self.detail_ui(ui, entry, now, &mut actions));
        }

        let rows = self.rows();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(RichText::new(with_text(icons::PLUS, "New Entry")).color(Color32::WHITE)).fill(ACCENT))
                    .clicked()
                {
                    actions.push(Action::New);
                }
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(with_text(icons::SEARCH, "Search entries..."))
                        .desired_width(ui.available_width() - 120.0),
                );
                egui::ComboBox::from_id_salt("sort")
                    .selected_text(with_text(icons::SORT, self.sort.label()))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for s in [Sort::NameAsc, Sort::NameDesc, Sort::Recent] {
                            ui.selectable_value(&mut self.sort, s, s.label());
                        }
                    });
            });
            ui.add_space(6.0);

            if rows.is_empty() {
                ui.add_space(60.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(icons::VAULT).size(48.0).color(MUTED));
                    ui.label(RichText::new("No entries yet").size(16.0).color(MUTED));
                    ui.label(RichText::new("Click New Entry to add your first password").small().color(MUTED));
                });
                return;
            }

            let cols = Cols::new(ui.available_width());
            ui.horizontal(|ui| {
                ui.add_space(ROW_PAD);
                ui.spacing_mut().item_spacing.x = COL_GAP;
                for (w, h) in [
                    (cols.star, ""),
                    (cols.name, "NAME"),
                    (cols.url, "URL"),
                    (cols.tags, "TAGS"),
                    (cols.strength, "STRENGTH"),
                    (cols.actions, "ACTIONS"),
                ] {
                    column(ui, w, 20.0, |ui| {
                        ui.label(RichText::new(h).small().strong().color(MUTED));
                    });
                }
            });
            ui.separator();

            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for r in &rows {
                    self.row_ui(ui, r, now, &cols, &mut actions);
                }
            });
        });

        for a in actions {
            self.apply(ui.ctx(), a);
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let (total, favs, tags) = match &self.vault {
            Some(v) => {
                let mut tags: Vec<(String, usize)> = Vec::new();
                for t in v.entries.iter().flat_map(|e| e.tags.iter()) {
                    match tags.iter_mut().find(|(n, _)| n == t) {
                        Some((_, c)) => *c += 1,
                        None => tags.push((t.clone(), 1)),
                    }
                }
                tags.sort();
                (v.entries.len(), v.entries.iter().filter(|e| e.favorite).count(), tags)
            }
            None => (0, 0, vec![]),
        };

        egui::Panel::left("sidebar").exact_size(190.0).show(ui, |ui| {
            ui.add_space(6.0);
            let all = with_text(icons::LIST, &format!("All Entries  ({total})"));
            if ui.selectable_label(self.filter == Filter::All, all).clicked() {
                self.filter = Filter::All;
            }
            let fav = with_text(icons::STAR, &format!("Favorites  ({favs})"));
            if ui.selectable_label(self.filter == Filter::Favorites, fav).clicked() {
                self.filter = Filter::Favorites;
            }
            ui.add_space(12.0);
            ui.label(RichText::new("TAGS").small().strong().color(MUTED));
            for (tag, count) in tags {
                let f = Filter::Tag(tag.clone());
                if ui.selectable_label(self.filter == f, with_text(icons::TAG, &format!("{tag}  ({count})"))).clicked() {
                    self.filter = f;
                }
            }
        });
    }

    fn row_ui(&mut self, ui: &mut egui::Ui, r: &Row, now: i64, cols: &Cols, actions: &mut Vec<Action>) {
        let selected = self.selected == Some(r.id);
        let fill = if selected { Color32::from_rgb(28, 22, 50) } else { Color32::from_rgb(15, 20, 33) };
        let stroke = if selected { egui::Stroke::new(1.0, ACCENT) } else { egui::Stroke::NONE };

        egui::Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(ROW_PAD as i8, 0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = COL_GAP;

                    column(ui, cols.star, ROW_H, |ui| {
                        let star = if r.favorite { icons::filled(icons::STAR).color(Color32::GOLD) } else { RichText::new(icons::STAR).color(MUTED) };
                        let tip = if r.favorite { "Remove from favorites" } else { "Add to favorites" };
                        if ui.add(egui::Button::new(star.size(16.0)).frame(false)).on_hover_text(tip).clicked() {
                            actions.push(Action::ToggleFavorite(r.id));
                        }
                    });

                    column(ui, cols.name, ROW_H, |ui| {
                        avatar(ui, &r.name);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            let mut name = RichText::new(&r.name).strong();
                            if selected {
                                name = name.color(ACCENT);
                            }
                            let resp = ui.add(egui::Label::new(name).truncate().sense(Sense::click()));
                            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                actions.push(Action::Select(r.id));
                            }
                            ui.add(egui::Label::new(RichText::new(&r.username).small().color(MUTED)).truncate());
                        });
                    });

                    column(ui, cols.url, ROW_H, |ui| {
                        if !r.url.is_empty() {
                            ui.add(egui::Label::new(RichText::new(with_text(icons::GLOBE, &r.url)).color(MUTED)).truncate());
                        }
                    });

                    column(ui, cols.tags, ROW_H, |ui| {
                        for t in r.tags.iter().take(2) {
                            tag_chip(ui, t);
                        }
                        if r.tags.len() > 2 {
                            ui.label(RichText::new(format!("+{}", r.tags.len() - 2)).small().color(MUTED));
                        }
                    });

                    column(ui, cols.strength, ROW_H, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.0;
                            ui.colored_label(strength_color(r.strength), with_text(icons::SHIELD, r.strength.label()));
                            ui.label(RichText::new(models::format_date(r.updated, now)).small().color(MUTED));
                        });
                    });

                    column(ui, cols.actions, ROW_H, |ui| {
                        if icon_button(ui, icons::COPY, "Copy password", MUTED) {
                            actions.push(Action::CopyPassword(r.id));
                        }
                        if icon_button(ui, icons::PENCIL, "Edit", MUTED) {
                            actions.push(Action::Edit(r.id));
                        }
                        if icon_button(ui, icons::TRASH, "Delete", Color32::from_rgb(239, 68, 68)) {
                            actions.push(Action::AskDelete(r.id));
                        }
                    });
                });
            });
    }

    fn detail_ui(&mut self, ui: &mut egui::Ui, e: &Entry, now: i64, actions: &mut Vec<Action>) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(6.0);
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    avatar(ui, &e.name);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.add(egui::Label::new(RichText::new(&e.name).heading()).truncate());
                        if !e.tags.is_empty() {
                            ui.horizontal_wrapped(|ui| {
                                for t in &e.tags {
                                    tag_chip(ui, t);
                                }
                            });
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if icon_button(ui, icons::CLOSE, "Close", MUTED) {
                            self.selected = None;
                        }
                        if icon_button(ui, icons::PENCIL, "Edit", MUTED) {
                            actions.push(Action::Edit(e.id));
                        }
                    });
                });
            });

            section(ui, icons::KEY, "Login credentials");
            card(ui, |ui| {
                field_label(ui, "Username");
                ui.horizontal(|ui| {
                    ui.monospace(&e.username);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, icons::COPY, "Copy username", MUTED) {
                            actions.push(Action::CopyText(e.username.clone()));
                        }
                    });
                });
                ui.separator();
                field_label(ui, "Password");
                ui.horizontal(|ui| {
                    ui.monospace(if self.show_password { e.password.as_str() } else { "••••••••••••" });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, icons::COPY, "Copy password", MUTED) {
                            actions.push(Action::CopyPassword(e.id));
                        }
                        let (icon, tip) = if self.show_password { (icons::EYE_SLASH, "Hide") } else { (icons::EYE, "Show") };
                        if icon_button(ui, icon, tip, MUTED) {
                            self.show_password = !self.show_password;
                        }
                    });
                });
                let s = models::strength(&e.password);
                ui.add(egui::ProgressBar::new(s.fraction()).fill(strength_color(s)).desired_height(6.0));
                ui.label(RichText::new(with_text(icons::SHIELD, &format!("Password strength: {}", s.label()))).small().color(strength_color(s)));
            });

            if !e.history.is_empty() {
                section(ui, icons::HISTORY, "Password history");
                card(ui, |ui| {
                    for (i, h) in e.history.iter().enumerate() {
                        if i > 0 {
                            ui.separator();
                        }
                        ui.label(RichText::new(format!("Changed on {}", models::format_date(h.changed, now))).small().color(MUTED));
                        ui.horizontal(|ui| {
                            let shown = self.shown_history.contains(&i);
                            ui.monospace(if shown { h.password.as_str() } else { "••••••••••••" });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if icon_button(ui, icons::COPY, "Copy", MUTED) {
                                    actions.push(Action::CopyText(h.password.clone()));
                                }
                                let (icon, tip) = if shown { (icons::EYE_SLASH, "Hide") } else { (icons::EYE, "Show") };
                                if icon_button(ui, icon, tip, MUTED) && !self.shown_history.remove(&i) {
                                    self.shown_history.insert(i);
                                }
                            });
                        });
                    }
                });
            }

            if !e.url.is_empty() {
                section(ui, icons::GLOBE, "Website");
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new(&e.url).truncate());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if icon_button(ui, icons::COPY, "Copy URL", MUTED) {
                                actions.push(Action::CopyText(e.url.clone()));
                            }
                        });
                    });
                });
            }

            section(ui, icons::LIST, "Item history");
            card(ui, |ui| {
                ui.label(RichText::new(format!("Last edited: {}", models::format_date(e.updated, now))).small().color(MUTED));
                ui.label(RichText::new(format!("Created: {}", models::format_date(e.created, now))).small().color(MUTED));
            });

            ui.add_space(10.0);
            if ui.button(RichText::new(with_text(icons::TRASH, "Delete entry")).color(Color32::LIGHT_RED)).clicked() {
                actions.push(Action::AskDelete(e.id));
            }
        });
    }

    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::Select(id) => {
                if self.selected != Some(id) {
                    self.selected = Some(id);
                    self.show_password = false;
                    self.shown_history.clear();
                }
            }
            Action::ToggleFavorite(id) => {
                if let Some(e) = self.entry_mut(id) {
                    e.favorite = !e.favorite;
                }
                self.save();
            }
            Action::CopyPassword(id) => {
                let pw = self.vault.as_ref().and_then(|v| v.entries.iter().find(|e| e.id == id)).map(|e| e.password.clone());
                if let Some(pw) = pw {
                    self.copy(ctx, pw, "Password");
                }
            }
            Action::CopyText(t) => self.copy(ctx, t, "Text"),
            Action::New => self.editor = Some(Editor::blank()),
            Action::Edit(id) => {
                if let Some(e) = self.vault.as_ref().and_then(|v| v.entries.iter().find(|e| e.id == id)) {
                    self.editor = Some(Editor::from_entry(e));
                }
            }
            Action::AskDelete(id) => self.confirm_delete = Some(id),
            Action::Lock => self.lock(),
        }
    }

    fn entry_mut(&mut self, id: u64) -> Option<&mut Entry> {
        self.vault.as_mut()?.entries.iter_mut().find(|e| e.id == id)
    }

    // ---------- dialogs ----------

    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.vault.is_none() {
            return;
        }
        self.editor_window(ctx);
        self.delete_window(ctx);
    }

    fn editor_window(&mut self, ctx: &egui::Context) {
        let Some(ed) = self.editor.as_mut() else { return };
        let mut save = false;
        let mut cancel = false;
        let title = if ed.id.is_some() { "Edit entry" } else { "New entry" };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Grid::new("editor").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label("Name");
                    ui.add(egui::TextEdit::singleline(&mut ed.name).desired_width(260.0));
                    ui.end_row();
                    ui.label(with_text(icons::GLOBE, "URL"));
                    ui.add(egui::TextEdit::singleline(&mut ed.url).desired_width(260.0));
                    ui.end_row();
                    ui.label(with_text(icons::USER, "Username"));
                    ui.add(egui::TextEdit::singleline(&mut ed.username).desired_width(260.0));
                    ui.end_row();
                    ui.label(with_text(icons::KEY, "Password"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut ed.password)
                                .password(!ed.show_password)
                                .desired_width(150.0),
                        );
                        let (icon, tip) = if ed.show_password { (icons::EYE_SLASH, "Hide") } else { (icons::EYE, "Show") };
                        if icon_button(ui, icon, tip, MUTED) {
                            ed.show_password = !ed.show_password;
                        }
                        if ui.button(with_text(icons::WAND, "Generate")).clicked() {
                            ed.password = models::generate_password(20);
                            ed.show_password = true;
                        }
                    });
                    ui.end_row();
                    ui.label(with_text(icons::TAG, "Tags"));
                    ui.add(egui::TextEdit::singleline(&mut ed.tags).hint_text("comma separated").desired_width(260.0));
                    ui.end_row();
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let ok = !ed.name.trim().is_empty();
                    if ui.add_enabled(ok, egui::Button::new(RichText::new(with_text(icons::CHECK, "Save")).color(Color32::WHITE)).fill(ACCENT)).clicked() {
                        save = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

        if cancel {
            self.editor = None;
        } else if save {
            let ed = self.editor.take().unwrap();
            self.commit_editor(ed);
        }
    }

    fn commit_editor(&mut self, ed: Editor) {
        let now = models::now();
        let mut tags: Vec<String> = Vec::new();
        for t in ed.tags.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            if !tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
                tags.push(t.to_string());
            }
        }
        let Some(vault) = self.vault.as_mut() else { return };

        let id = match ed.id.and_then(|id| vault.entries.iter_mut().find(|e| e.id == id)) {
            Some(e) => {
                e.name = ed.name.trim().into();
                e.url = ed.url.trim().into();
                e.username = ed.username.trim().into();
                e.tags = tags;
                e.set_password(ed.password.clone(), now);
                e.updated = now;
                e.id
            }
            None => {
                let id = vault.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                vault.entries.push(Entry {
                    id,
                    name: ed.name.trim().into(),
                    url: ed.url.trim().into(),
                    username: ed.username.trim().into(),
                    password: ed.password.clone(),
                    tags,
                    created: now,
                    updated: now,
                    ..Default::default()
                });
                id
            }
        };
        self.selected = Some(id);
        self.save();
    }

    fn delete_window(&mut self, ctx: &egui::Context) {
        let Some(id) = self.confirm_delete else { return };
        let name = self
            .vault
            .as_ref()
            .and_then(|v| v.entries.iter().find(|e| e.id == id))
            .map(|e| e.name.clone())
            .unwrap_or_default();
        let mut decision = None;

        egui::Window::new("Delete entry?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("\"{name}\" will be permanently removed."));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(with_text(icons::TRASH, "Delete")).color(Color32::LIGHT_RED)).clicked() {
                        decision = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        decision = Some(false);
                    }
                });
            });

        match decision {
            Some(true) => {
                if let Some(v) = self.vault.as_mut() {
                    v.entries.retain(|e| e.id != id);
                }
                if self.selected == Some(id) {
                    self.selected = None;
                }
                self.confirm_delete = None;
                self.save();
            }
            Some(false) => self.confirm_delete = None,
            None => {}
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.tick(ui.ctx());
        if self.vault.is_some() {
            self.main_ui(ui);
        } else {
            self.login_ui(ui);
        }
        self.dialogs(ui.ctx());
    }
}

const ROW_H: f32 = 52.0;
const ROW_PAD: f32 = 10.0;
const COL_GAP: f32 = 12.0;

struct Cols {
    star: f32,
    name: f32,
    url: f32,
    tags: f32,
    strength: f32,
    actions: f32,
}

impl Cols {
    fn new(total: f32) -> Self {
        let (star, strength, actions) = (24.0, 100.0, 100.0);
        let flexible = (total - 2.0 * ROW_PAD - 5.0 * COL_GAP - star - strength - actions - 8.0).max(240.0);
        Cols {
            star,
            name: flexible * 0.38,
            url: flexible * 0.32,
            tags: flexible * 0.30,
            strength,
            actions,
        }
    }
}

/// Fixed-width, vertically centered cell so header and rows line up.
fn column(ui: &mut egui::Ui, width: f32, height: f32, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(Vec2::new(width, height), Layout::left_to_right(Align::Center), |ui| {
        ui.set_width(width);
        ui.set_clip_rect(ui.max_rect().intersect(ui.clip_rect()));
        add(ui);
    });
}

fn avatar(ui: &mut egui::Ui, name: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
    ui.painter().rect_filled(rect, 8.0, ACCENT);
    let letter = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, letter, egui::FontId::proportional(15.0), Color32::WHITE);
}

fn tag_chip(ui: &mut egui::Ui, tag: &str) {
    ui.label(
        RichText::new(format!(" {} {tag} ", icons::TAG))
            .small()
            .color(Color32::from_rgb(216, 180, 254))
            .background_color(Color32::from_rgb(52, 28, 84)),
    );
}

fn strength_color(s: Strength) -> Color32 {
    match s {
        Strength::Weak => Color32::from_rgb(239, 68, 68),
        Strength::Fair => Color32::from_rgb(234, 179, 8),
        Strength::Strong => Color32::from_rgb(34, 197, 94),
    }
}

fn section(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.add_space(12.0);
    ui.label(RichText::new(with_text(icon, title)).small().strong().color(MUTED));
    ui.add_space(2.0);
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::from_rgb(17, 22, 36))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(32, 39, 58)))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

fn icon_button(ui: &mut egui::Ui, icon: &str, tip: &str, color: Color32) -> bool {
    ui.add(egui::Button::new(RichText::new(icon).size(16.0).color(color)).frame(false))
        .on_hover_text(tip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn wordmark(ui: &mut egui::Ui, size: f32) {
    let mut job = egui::text::LayoutJob::default();
    for (text, color) in [("Pass", Color32::WHITE), ("Lock", ORANGE)] {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(size),
                color,
                ..Default::default()
            },
        );
    }
    ui.label(job);
}

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(MUTED));
}
