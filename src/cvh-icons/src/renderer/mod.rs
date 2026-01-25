//! Rendering module for desktop icons
//!
//! Uses tiny-skia for software rendering to Wayland surfaces.

use anyhow::Result;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Rect, Stroke,
    Transform,
};

use crate::icons::DesktopIcon;
use crate::lua::DrawCommand;

/// Icon renderer
pub struct IconRenderer {
    /// Icon size
    size: u32,

    /// Font for labels (would use fontdue in production)
    font_size: f32,

    /// Colors
    label_fg: Color,
    label_bg: Color,
    selection_color: Color,
}

impl IconRenderer {
    pub fn new(size: u32, font_size: f32) -> Self {
        Self {
            size,
            font_size,
            label_fg: Color::WHITE,
            label_bg: Color::from_rgba8(0, 0, 0, 128),
            selection_color: Color::from_rgba8(136, 192, 208, 64),
        }
    }

    /// Render an icon to a pixmap
    pub fn render(&self, icon: &DesktopIcon) -> Result<Pixmap> {
        let total_height = self.size + 24; // Icon + label space
        let mut pixmap = Pixmap::new(self.size, total_height)
            .ok_or_else(|| anyhow::anyhow!("Failed to create pixmap"))?;

        // Clear with transparent
        pixmap.fill(Color::TRANSPARENT);

        // Draw selection background if selected
        if icon.is_selected() {
            let mut paint = Paint::default();
            paint.set_color(self.selection_color);

            let rect = Rect::from_xywh(0.0, 0.0, self.size as f32, total_height as f32)
                .ok_or_else(|| anyhow::anyhow!("Invalid rect"))?;

            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }

        // Draw icon placeholder (would load actual icon in production)
        self.draw_icon_placeholder(&mut pixmap, icon)?;

        // Draw label
        self.draw_label(&mut pixmap, icon.name())?;

        Ok(pixmap)
    }

    /// Draw a placeholder icon shape
    fn draw_icon_placeholder(&self, pixmap: &mut Pixmap, icon: &DesktopIcon) -> Result<()> {
        let mut paint = Paint::default();

        // Choose color based on type
        let color = match icon.icon_type() {
            crate::icons::IconType::Folder => Color::from_rgba8(229, 192, 123, 255),
            crate::icons::IconType::File => Color::from_rgba8(171, 178, 191, 255),
            crate::icons::IconType::Executable => Color::from_rgba8(152, 195, 121, 255),
            crate::icons::IconType::Image => Color::from_rgba8(198, 120, 221, 255),
            crate::icons::IconType::Document => Color::from_rgba8(97, 175, 239, 255),
            crate::icons::IconType::Archive => Color::from_rgba8(224, 108, 117, 255),
            crate::icons::IconType::Video => Color::from_rgba8(209, 154, 102, 255),
            crate::icons::IconType::Audio => Color::from_rgba8(86, 182, 194, 255),
            _ => Color::from_rgba8(171, 178, 191, 255),
        };

        paint.set_color(color);

        let margin = 8.0;
        let icon_size = self.size as f32 - margin * 2.0;

        match icon.icon_type() {
            crate::icons::IconType::Folder => {
                // Draw folder shape
                let mut pb = PathBuilder::new();
                pb.move_to(margin, margin + 8.0);
                pb.line_to(margin + icon_size * 0.4, margin + 8.0);
                pb.line_to(margin + icon_size * 0.5, margin);
                pb.line_to(margin + icon_size, margin);
                pb.line_to(margin + icon_size, margin + icon_size);
                pb.line_to(margin, margin + icon_size);
                pb.close();

                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
            _ => {
                // Draw file shape
                let mut pb = PathBuilder::new();
                let fold = 12.0;
                pb.move_to(margin, margin);
                pb.line_to(margin + icon_size - fold, margin);
                pb.line_to(margin + icon_size, margin + fold);
                pb.line_to(margin + icon_size, margin + icon_size);
                pb.line_to(margin, margin + icon_size);
                pb.close();

                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                }

                // Draw fold
                let mut fold_paint = Paint::default();
                fold_paint.set_color(Color::from_rgba8(0, 0, 0, 50));

                let mut pb = PathBuilder::new();
                pb.move_to(margin + icon_size - fold, margin);
                pb.line_to(margin + icon_size - fold, margin + fold);
                pb.line_to(margin + icon_size, margin + fold);
                pb.close();

                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path, &fold_paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }

        Ok(())
    }

    /// Draw the label below the icon
    fn draw_label(&self, pixmap: &mut Pixmap, name: &str) -> Result<()> {
        // Truncate name if too long
        let max_chars = 12;
        let display_name = if name.len() > max_chars {
            format!("{}...", &name[..max_chars - 3])
        } else {
            name.to_string()
        };

        // Label background
        let label_y = self.size as f32 + 2.0;
        let label_height = 18.0;

        let mut bg_paint = Paint::default();
        bg_paint.set_color(self.label_bg);

        if let Some(rect) = Rect::from_xywh(0.0, label_y, self.size as f32, label_height) {
            pixmap.fill_rect(rect, &bg_paint, Transform::identity(), None);
        }

        // Note: Actual text rendering would use fontdue
        // For now, we just have the background

        Ok(())
    }

    /// Execute Lua draw commands
    pub fn execute_commands(&self, pixmap: &mut Pixmap, commands: &[DrawCommand]) -> Result<()> {
        for cmd in commands {
            match cmd {
                DrawCommand::Clear { color } => {
                    if let Some(c) = parse_color(color) {
                        pixmap.fill(c);
                    }
                }
                DrawCommand::FillRect { x, y, w, h, color } => {
                    if let (Some(rect), Some(color)) = (
                        Rect::from_xywh(*x, *y, *w, *h),
                        parse_color(color),
                    ) {
                        let mut paint = Paint::default();
                        paint.set_color(color);
                        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
                    }
                }
                DrawCommand::StrokeRect { x, y, w, h, color, width } => {
                    if let Some(color) = parse_color(color) {
                        let mut paint = Paint::default();
                        paint.set_color(color);

                        let stroke = Stroke {
                            width: *width,
                            line_cap: LineCap::Square,
                            line_join: LineJoin::Miter,
                            ..Default::default()
                        };

                        let mut pb = PathBuilder::new();
                        pb.move_to(*x, *y);
                        pb.line_to(x + w, *y);
                        pb.line_to(x + w, y + h);
                        pb.line_to(*x, y + h);
                        pb.close();

                        if let Some(path) = pb.finish() {
                            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                        }
                    }
                }
                DrawCommand::FillCircle { cx, cy, r, color } => {
                    if let Some(color) = parse_color(color) {
                        let mut paint = Paint::default();
                        paint.set_color(color);

                        // Approximate circle with path
                        let mut pb = PathBuilder::new();
                        pb.push_circle(*cx, *cy, *r);

                        if let Some(path) = pb.finish() {
                            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                        }
                    }
                }
                DrawCommand::Line { x1, y1, x2, y2, color, width } => {
                    if let Some(color) = parse_color(color) {
                        let mut paint = Paint::default();
                        paint.set_color(color);

                        let stroke = Stroke {
                            width: *width,
                            line_cap: LineCap::Round,
                            ..Default::default()
                        };

                        let mut pb = PathBuilder::new();
                        pb.move_to(*x1, *y1);
                        pb.line_to(*x2, *y2);

                        if let Some(path) = pb.finish() {
                            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                        }
                    }
                }
                _ => {
                    // Image and Text rendering would need additional implementation
                }
            }
        }

        Ok(())
    }
}

/// Parse a color string (hex format)
fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');

    match s.len() {
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Color::from_rgba8(r, g, b, 255))
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some(Color::from_rgba8(r, g, b, a))
        }
        _ => None,
    }
}
