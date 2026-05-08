use crate::config::{Config, ElementConfig, FractionalRect, SlotKind, TopAlign};
use crate::elements::{Element, Globals, make_element};
use crate::error::AppError;
use crate::layout::Layout;

struct TopEntry {
    element: Box<dyn Element>,
    width: Option<f32>,
}

struct WindowEntry {
    element: Box<dyn Element>,
    title: String,
    rect: FractionalRect,
    open: bool,
    toggle_key: Option<egui::Key>,
}

pub struct App {
    bg: egui::Color32,
    top_left: Vec<TopEntry>,
    top_right: Vec<TopEntry>,
    center: Option<Box<dyn Element>>,
    windows: Vec<WindowEntry>,
    top_panel_height: f32,
    no_cursor: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, config: Config) -> Result<Self, AppError> {
        let bg = config.window.background.into();
        let home = match &config.home {
            Some(h) => Some(h.resolve()?),
            None => None,
        };
        let mut markers = Vec::with_capacity(config.markers.len() + 1);
        if let Some(h) = &home {
            markers.push(h.clone());
        }
        for m in &config.markers {
            markers.push(m.resolve()?);
        }
        let globals = Globals { markers, home };

        let mut top_left = Vec::new();
        let mut top_right = Vec::new();
        let mut center: Option<Box<dyn Element>> = None;
        let mut windows = Vec::new();

        for cfg in &config.elements {
            let element = make_element(cfg, &globals)?;
            match cfg.slot {
                SlotKind::Top => {
                    let entry = TopEntry {
                        element,
                        width: cfg.width,
                    };
                    match cfg.align {
                        TopAlign::Left => top_left.push(entry),
                        TopAlign::Right => top_right.push(entry),
                    }
                }
                SlotKind::Center => {
                    if center.is_some() {
                        return Err(AppError::MultipleCenterElements);
                    }
                    center = Some(element);
                }
                SlotKind::Window => {
                    let rect = cfg.rect.ok_or_else(|| AppError::InvalidSlot {
                        kind: cfg.kind.clone(),
                        msg: "slot=\"window\" requires `rect = { x, y, w, h }`".into(),
                    })?;
                    let title = window_title(cfg);
                    let toggle_key =
                        match &cfg.key {
                            None => None,
                            Some(s) => Some(egui::Key::from_name(s).ok_or_else(|| {
                                AppError::InvalidSlot {
                                    kind: cfg.kind.clone(),
                                    msg: format!("unknown key name {s:?}"),
                                }
                            })?),
                        };
                    windows.push(WindowEntry {
                        element,
                        title,
                        rect,
                        open: cfg.open,
                        toggle_key,
                    });
                }
            }
        }

        Ok(Self {
            bg,
            top_left,
            top_right,
            center,
            windows,
            top_panel_height: config.window.top_panel_height,
            no_cursor: config.window.no_cursor,
        })
    }

    fn render_top(&mut self, ui: &mut egui::Ui) {
        let panel = ui.max_rect();
        let total = panel.width();
        let half = total / 2.0;

        // Allocate per-element pixel widths within each side. If `width` is
        // set on an element it is taken as a fraction of the full window width
        // (consistent with FractionalRect). Unspecified entries split the
        // remaining half equally.
        let left_widths = pack_widths(&self.top_left, total, half);
        let right_widths = pack_widths(&self.top_right, total, half);

        let mut x = panel.min.x;
        for (entry, w) in self.top_left.iter_mut().zip(left_widths.iter()) {
            let rect = egui::Rect::from_min_size(
                egui::pos2(x, panel.min.y),
                egui::vec2(*w, panel.height()),
            );
            ui.put(rect, &mut *entry.element);
            x += w;
        }

        let mut x = panel.max.x;
        for (entry, w) in self.top_right.iter_mut().zip(right_widths.iter()) {
            x -= w;
            let rect = egui::Rect::from_min_size(
                egui::pos2(x, panel.min.y),
                egui::vec2(*w, panel.height()),
            );
            ui.put(rect, &mut *entry.element);
        }
    }
}

fn window_title(cfg: &ElementConfig) -> String {
    cfg.title.clone().unwrap_or_else(|| capitalize(&cfg.kind))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Compute pixel widths for one side of the top panel.
///
/// Explicit `width` values are taken as fractions of the full window width.
/// Unspecified entries divide the remaining `half_width` equally.
fn pack_widths(entries: &[TopEntry], total_width: f32, half_width: f32) -> Vec<f32> {
    let mut explicit: f32 = 0.0;
    let mut unspecified = 0usize;
    for e in entries {
        match e.width {
            Some(w) => explicit += w * total_width,
            None => unspecified += 1,
        }
    }
    let remaining = (half_width - explicit).max(0.0);
    let per_auto = if unspecified > 0 {
        remaining / unspecified as f32
    } else {
        0.0
    };
    entries
        .iter()
        .map(|e| match e.width {
            Some(w) => w * total_width,
            None => per_auto,
        })
        .collect()
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let [r, g, b, a] = self.bg.to_array();
        [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Standard quit / fullscreen hotkeys, handled centrally so they apply
        // regardless of which element has focus:
        //   Esc, Ctrl/Cmd+Q  → quit
        //   F                → toggle fullscreen
        // Per-window toggle keys come from the element config (`key = "..."`).
        let (quit, toggle_full, window_toggles) = ctx.input(|i| {
            let quit = i.key_pressed(egui::Key::Escape)
                || (i.modifiers.command && i.key_pressed(egui::Key::Q));
            let toggle_full = i.key_pressed(egui::Key::F);
            let toggles: Vec<usize> = self
                .windows
                .iter()
                .enumerate()
                .filter_map(|(idx, w)| match w.toggle_key {
                    Some(k) if i.key_pressed(k) => Some(idx),
                    _ => None,
                })
                .collect();
            (quit, toggle_full, toggles)
        });
        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if toggle_full {
            let now_full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!now_full));
        }
        for idx in window_toggles {
            self.windows[idx].open = !self.windows[idx].open;
        }

        if self.no_cursor {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }

        let panel_h = (self.top_panel_height * ctx.content_rect().height()).max(0.0);
        let bg = self.bg;

        if !self.top_left.is_empty() || !self.top_right.is_empty() {
            egui::Panel::top("wjmclock_top")
                .exact_size(panel_h)
                .frame(egui::Frame::NONE.fill(bg))
                .show_inside(ui, |ui| self.render_top(ui));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(bg))
            .show_inside(ui, |ui| {
                if let Some(c) = self.center.as_mut() {
                    ui.add(&mut **c);
                }
            });

        let window_frame = egui::Frame::window(&ctx.global_style())
            .fill(egui::Color32::from_rgba_unmultiplied(10, 10, 20, 153));
        for w in &mut self.windows {
            let init = Layout::resolve(ctx.content_rect(), w.rect);
            egui::Window::new(&w.title)
                .frame(window_frame)
                .default_pos(init.min)
                .default_size(init.size())
                .open(&mut w.open)
                .show(&ctx, |ui| {
                    ui.add(&mut *w.element);
                });
        }
    }
}
