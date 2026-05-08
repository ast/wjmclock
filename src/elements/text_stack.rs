//! Centered, scale-to-fit text stack — the shared visual primitive used by
//! `Clock`, `Callsign`, and similar painter-based elements.
//!
//! Each row picks its own font size as the smaller of its height budget
//! (`height_frac * rect.height()`) and its width budget
//! (`rect.width() / (max_chars * em_factor)`). Leftover vertical space is
//! distributed evenly above, between, and below the rows.

use egui::{Align2, Color32, FontId, Painter, Rect, pos2};

#[derive(Debug, Clone, Copy)]
pub enum FontKind {
    Monospace,
    Proportional,
}

impl FontKind {
    fn font(self, size: f32) -> FontId {
        match self {
            Self::Monospace => FontId::monospace(size),
            Self::Proportional => FontId::proportional(size),
        }
    }

    /// Approximate em-advance for the bundled egui font. Used to estimate how
    /// many characters fit in a given width when sizing rows.
    fn default_em_factor(self) -> f32 {
        match self {
            Self::Monospace => 0.62,
            Self::Proportional => 0.55,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextRow {
    pub text: String,
    pub kind: FontKind,
    /// Fraction of the rect's height this row would like to occupy.
    pub height_frac: f32,
    /// Approximate character count, used to derive a width cap.
    pub max_chars: f32,
    /// Override the kind's default em-advance factor. Rarely needed.
    pub em_factor: Option<f32>,
    pub color: Color32,
}

impl TextRow {
    pub fn monospace(
        text: impl Into<String>,
        height_frac: f32,
        max_chars: f32,
        color: Color32,
    ) -> Self {
        Self {
            text: text.into(),
            kind: FontKind::Monospace,
            height_frac,
            max_chars,
            em_factor: None,
            color,
        }
    }

    pub fn proportional(
        text: impl Into<String>,
        height_frac: f32,
        max_chars: f32,
        color: Color32,
    ) -> Self {
        Self {
            text: text.into(),
            kind: FontKind::Proportional,
            height_frac,
            max_chars,
            em_factor: None,
            color,
        }
    }
}

/// Paint a centered, top-down stack of single-line text rows that scale to
/// fit `rect`. See module docs for the sizing model.
pub fn paint_text_stack(painter: &Painter, rect: Rect, rows: &[TextRow]) {
    if rows.is_empty() {
        return;
    }
    let sizes: Vec<f32> = rows
        .iter()
        .map(|r| {
            let em = r.em_factor.unwrap_or_else(|| r.kind.default_em_factor());
            let h_max = r.height_frac * rect.height();
            let w_max = rect.width() / (r.max_chars * em);
            h_max.min(w_max)
        })
        .collect();
    let used: f32 = sizes.iter().sum();
    let gap = ((rect.height() - used) / (rows.len() as f32 + 1.0)).max(0.0);

    let center_x = rect.center().x;
    let mut y = rect.min.y + gap;
    for (row, &size) in rows.iter().zip(&sizes) {
        painter.text(
            pos2(center_x, y),
            Align2::CENTER_TOP,
            &row.text,
            row.kind.font(size),
            row.color,
        );
        y += size + gap;
    }
}
