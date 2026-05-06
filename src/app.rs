use crate::config::{Config, FractionalRect, parse_color};
use crate::elements::{Element, Globals, make_element};
use crate::error::AppError;
use crate::layout::Layout;

pub struct App {
    bg: egui::Color32,
    elements: Vec<(FractionalRect, Box<dyn Element>)>,
    no_cursor: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, config: Config) -> Result<Self, AppError> {
        let bg = parse_color(&config.window.background);
        let mut markers = Vec::with_capacity(config.markers.len());
        for m in &config.markers {
            markers.push(m.resolve()?);
        }
        let globals = Globals { markers };
        let mut elements: Vec<(FractionalRect, Box<dyn Element>)> = Vec::new();
        for el in &config.elements {
            let e = make_element(el, &globals)?;
            elements.push((el.rect, e));
        }
        Ok(Self {
            bg,
            elements,
            no_cursor: config.window.no_cursor,
        })
    }
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
        //   F11              → toggle fullscreen
        let (quit, toggle_full) = ctx.input(|i| {
            let quit = i.key_pressed(egui::Key::Escape)
                || (i.modifiers.command && i.key_pressed(egui::Key::Q));
            let toggle_full = i.key_pressed(egui::Key::F11);
            (quit, toggle_full)
        });
        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if toggle_full {
            let now_full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!now_full));
        }

        if self.no_cursor {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        }

        for (_, el) in &mut self.elements {
            el.update(&ctx);
        }

        let screen = ui.max_rect();
        ui.painter().rect_filled(screen, 0.0, self.bg);

        for (frac, el) in &mut self.elements {
            let rect = Layout::resolve(screen, *frac);
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            el.ui(&mut child);
        }
    }
}
