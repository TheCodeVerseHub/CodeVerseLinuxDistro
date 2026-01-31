//! Rendering module for desktop icons
//!
//! Uses tiny-skia for software rendering to Wayland surfaces.

use anyhow::Result;
use fontdue::{Font, FontSettings};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Pixmap, PixmapPaint, PathBuilder, Rect, Stroke,
    Transform,
};
use tracing::warn;

use crate::icons::DesktopIcon;
use crate::lua::DrawCommand;

/// Text alignment options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl TextAlign {
    /// Parse alignment from string (case-insensitive)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "center" => TextAlign::Center,
            "right" => TextAlign::Right,
            _ => TextAlign::Left,
        }
    }
}

/// Common system font paths to search for DejaVu Sans
const FONT_SEARCH_PATHS: &[&str] = &[
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/DejaVuSans.ttf",
    "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
];

/// Try to load a default font from common system paths
fn load_default_font() -> Option<Font> {
    for path in FONT_SEARCH_PATHS {
        if let Ok(font_data) = std::fs::read(path) {
            match Font::from_bytes(font_data, FontSettings::default()) {
                Ok(font) => {
                    tracing::debug!("Loaded font from: {}", path);
                    return Some(font);
                }
                Err(e) => {
                    tracing::trace!("Failed to parse font {}: {}", path, e);
                }
            }
        }
    }
    warn!("No system font found, text rendering will be disabled");
    None
}

/// Icon renderer
#[allow(dead_code)]
pub struct IconRenderer {
    /// Icon size
    size: u32,

    /// Font for labels
    font_size: f32,

    /// Loaded font for text rendering (None if loading failed)
    font: Option<Font>,

    /// Colors
    label_fg: Color,
    label_bg: Color,
    selection_color: Color,
}

#[allow(dead_code)]
impl IconRenderer {
    pub fn new(size: u32, font_size: f32) -> Self {
        Self {
            size,
            font_size,
            font: load_default_font(),
            label_fg: Color::WHITE,
            label_bg: Color::from_rgba8(0, 0, 0, 128),
            selection_color: Color::from_rgba8(136, 192, 208, 64),
        }
    }

    /// Create a renderer with a specific font (useful for testing)
    pub fn with_font(size: u32, font_size: f32, font: Option<Font>) -> Self {
        Self {
            size,
            font_size,
            font,
            label_fg: Color::WHITE,
            label_bg: Color::from_rgba8(0, 0, 0, 128),
            selection_color: Color::from_rgba8(136, 192, 208, 64),
        }
    }

    /// Render text to a pixmap
    ///
    /// # Arguments
    /// * `pixmap` - Target pixmap to draw on
    /// * `text` - Text string to render
    /// * `x` - X position (meaning depends on alignment)
    /// * `y` - Y position (baseline)
    /// * `size` - Font size in pixels
    /// * `color` - Text color as tiny-skia Color
    /// * `align` - Text alignment (left, center, right)
    pub fn render_text(
        &self,
        pixmap: &mut Pixmap,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        align: TextAlign,
    ) {
        let font = match &self.font {
            Some(f) => f,
            None => return, // No font loaded, skip text rendering
        };

        if text.is_empty() {
            return;
        }

        // Calculate total text width for alignment
        let mut total_width = 0.0f32;
        let mut glyph_data: Vec<(fontdue::Metrics, Vec<u8>)> = Vec::new();

        for ch in text.chars() {
            let (metrics, bitmap) = font.rasterize(ch, size);
            total_width += metrics.advance_width;
            glyph_data.push((metrics, bitmap));
        }

        // Calculate starting x position based on alignment
        let start_x = match align {
            TextAlign::Left => x,
            TextAlign::Center => x - total_width / 2.0,
            TextAlign::Right => x - total_width,
        };

        // Extract color components (premultiplied alpha)
        let r = (color.red() * 255.0) as u8;
        let g = (color.green() * 255.0) as u8;
        let b = (color.blue() * 255.0) as u8;
        let base_alpha = color.alpha();

        let mut cursor_x = start_x;

        for (metrics, bitmap) in glyph_data {
            if bitmap.is_empty() {
                cursor_x += metrics.advance_width;
                continue;
            }

            // Calculate glyph position
            // y is baseline, ymin is typically negative for glyphs above baseline
            let glyph_x = cursor_x + metrics.xmin as f32;
            let glyph_y = y + metrics.ymin as f32;

            // Create a small pixmap for the glyph
            if metrics.width > 0 && metrics.height > 0 {
                if let Some(mut glyph_pixmap) = Pixmap::new(metrics.width as u32, metrics.height as u32) {
                    // Fill glyph pixmap with colored text
                    let pixels = glyph_pixmap.pixels_mut();
                    for (i, coverage) in bitmap.iter().enumerate() {
                        if *coverage > 0 {
                            let alpha = (*coverage as f32 / 255.0) * base_alpha;
                            // tiny-skia uses premultiplied alpha
                            let pm_r = (r as f32 * alpha) as u8;
                            let pm_g = (g as f32 * alpha) as u8;
                            let pm_b = (b as f32 * alpha) as u8;
                            let pm_a = (alpha * 255.0) as u8;
                            pixels[i] = tiny_skia::PremultipliedColorU8::from_rgba(pm_r, pm_g, pm_b, pm_a)
                                .expect("valid premultiplied color");
                        }
                    }

                    // Blit glyph to main pixmap
                    let glyph_x_int = glyph_x.round() as i32;
                    let glyph_y_int = glyph_y.round() as i32;

                    pixmap.draw_pixmap(
                        glyph_x_int,
                        glyph_y_int,
                        glyph_pixmap.as_ref(),
                        &PixmapPaint::default(),
                        Transform::identity(),
                        None,
                    );
                }
            }

            cursor_x += metrics.advance_width;
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

        // Render text centered horizontally, with baseline near bottom of label area
        let text_x = self.size as f32 / 2.0;
        let text_y = label_y + label_height - 4.0; // Position baseline
        self.render_text(
            pixmap,
            &display_name,
            text_x,
            text_y,
            self.font_size,
            self.label_fg,
            TextAlign::Center,
        );

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
                DrawCommand::Text { text, x, y, size, color, align } => {
                    if let Some(text_color) = parse_color(color) {
                        let alignment = TextAlign::from_str(align);
                        self.render_text(pixmap, text, *x, *y, *size, text_color, alignment);
                    }
                }
                DrawCommand::Image { .. } => {
                    // Image rendering would need additional implementation
                }
                DrawCommand::StrokeCircle { .. } => {
                    // StrokeCircle rendering would need additional implementation
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Color Parsing Tests
    // ========================================================================

    #[test]
    fn test_parse_color_6_digit_hex_with_hash() {
        let color = parse_color("#ff0000").unwrap();
        // Color::from_rgba8 returns premultiplied colors, check components
        assert_eq!(color.red(), 1.0, "Red component should be 1.0");
        assert_eq!(color.green(), 0.0, "Green component should be 0.0");
        assert_eq!(color.blue(), 0.0, "Blue component should be 0.0");
        assert_eq!(color.alpha(), 1.0, "Alpha should be 1.0 for 6-digit hex");
    }

    #[test]
    fn test_parse_color_6_digit_hex_without_hash() {
        let color = parse_color("00ff00").unwrap();
        assert_eq!(color.red(), 0.0, "Red component should be 0.0");
        assert_eq!(color.green(), 1.0, "Green component should be 1.0");
        assert_eq!(color.blue(), 0.0, "Blue component should be 0.0");
        assert_eq!(color.alpha(), 1.0, "Alpha should be 1.0 for 6-digit hex");
    }

    #[test]
    fn test_parse_color_6_digit_hex_blue() {
        let color = parse_color("#0000ff").unwrap();
        assert_eq!(color.red(), 0.0);
        assert_eq!(color.green(), 0.0);
        assert_eq!(color.blue(), 1.0);
        assert_eq!(color.alpha(), 1.0);
    }

    #[test]
    fn test_parse_color_6_digit_hex_white() {
        let color = parse_color("#ffffff").unwrap();
        assert_eq!(color.red(), 1.0);
        assert_eq!(color.green(), 1.0);
        assert_eq!(color.blue(), 1.0);
        assert_eq!(color.alpha(), 1.0);
    }

    #[test]
    fn test_parse_color_6_digit_hex_black() {
        let color = parse_color("#000000").unwrap();
        assert_eq!(color.red(), 0.0);
        assert_eq!(color.green(), 0.0);
        assert_eq!(color.blue(), 0.0);
        assert_eq!(color.alpha(), 1.0);
    }

    #[test]
    fn test_parse_color_8_digit_hex_with_alpha() {
        let color = parse_color("#ff000080").unwrap();
        // Note: tiny-skia uses premultiplied alpha
        // With alpha = 128/255 ≈ 0.502, and red = 255, premultiplied red ≈ 0.502
        let alpha = color.alpha();
        assert!((alpha - 128.0 / 255.0).abs() < 0.01, "Alpha should be ~0.502, got {}", alpha);
    }

    #[test]
    fn test_parse_color_8_digit_hex_full_alpha() {
        let color = parse_color("#00ff00ff").unwrap();
        assert_eq!(color.green(), 1.0, "Green should be 1.0");
        assert_eq!(color.alpha(), 1.0, "Alpha should be 1.0 (ff = 255)");
    }

    #[test]
    fn test_parse_color_8_digit_hex_zero_alpha() {
        let color = parse_color("#ff000000").unwrap();
        assert_eq!(color.alpha(), 0.0, "Alpha should be 0.0 (00 = 0)");
    }

    #[test]
    fn test_parse_color_invalid_length() {
        assert!(parse_color("#fff").is_none(), "3-digit hex should return None");
        assert!(parse_color("#ffff").is_none(), "4-digit hex should return None");
        assert!(parse_color("#fffff").is_none(), "5-digit hex should return None");
        assert!(parse_color("#fffffff").is_none(), "7-digit hex should return None");
        assert!(parse_color("#fffffffff").is_none(), "9-digit hex should return None");
    }

    #[test]
    fn test_parse_color_invalid_characters() {
        assert!(parse_color("#gggggg").is_none(), "Invalid hex chars should return None");
        assert!(parse_color("#xyz123").is_none(), "Invalid hex chars should return None");
    }

    #[test]
    fn test_parse_color_empty_string() {
        assert!(parse_color("").is_none(), "Empty string should return None");
        assert!(parse_color("#").is_none(), "Just hash should return None");
    }

    #[test]
    fn test_parse_color_case_insensitive() {
        let lower = parse_color("#aabbcc").unwrap();
        let upper = parse_color("#AABBCC").unwrap();
        assert_eq!(lower.red(), upper.red(), "Color parsing should be case-insensitive");
        assert_eq!(lower.green(), upper.green());
        assert_eq!(lower.blue(), upper.blue());
    }

    // ========================================================================
    // DrawCommand Clear Tests
    // ========================================================================

    #[test]
    fn test_clear_fills_pixmap_with_color() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![DrawCommand::Clear {
            color: "#ff0000".to_string(),
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check a sample pixel
        let pixel = pixmap.pixel(32, 32).unwrap();
        assert_eq!(pixel.red(), 255, "Pixel red should be 255 after clear");
        assert_eq!(pixel.green(), 0, "Pixel green should be 0 after clear");
        assert_eq!(pixel.blue(), 0, "Pixel blue should be 0 after clear");
        assert_eq!(pixel.alpha(), 255, "Pixel alpha should be 255 after clear");
    }

    #[test]
    fn test_clear_with_transparent_color() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        // First fill with red
        pixmap.fill(Color::from_rgba8(255, 0, 0, 255));

        // Clear with transparent
        let commands = vec![DrawCommand::Clear {
            color: "#00000000".to_string(),
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        let pixel = pixmap.pixel(32, 32).unwrap();
        assert_eq!(pixel.alpha(), 0, "Pixel should be transparent after clear");
    }

    // ========================================================================
    // DrawCommand FillRect Tests
    // ========================================================================

    #[test]
    fn test_fill_rect_draws_rectangle_at_correct_position() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        // Clear with black first
        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::FillRect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
            color: "#00ff00".to_string(),
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check pixel inside the rectangle
        let inside_pixel = pixmap.pixel(15, 15).unwrap();
        assert_eq!(inside_pixel.green(), 255, "Pixel inside rect should be green");

        // Check pixel outside the rectangle (should still be black)
        let outside_pixel = pixmap.pixel(5, 5).unwrap();
        assert_eq!(outside_pixel.green(), 0, "Pixel outside rect should not be green");
    }

    #[test]
    fn test_fill_rect_with_alpha() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::FillRect {
            x: 0.0,
            y: 0.0,
            w: 64.0,
            h: 64.0,
            color: "#ff000080".to_string(), // Red with 50% alpha
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        let pixel = pixmap.pixel(32, 32).unwrap();
        // With alpha blending, red should be mixed
        assert!(pixel.red() > 0, "Red should be present after fill with alpha");
    }

    #[test]
    fn test_fill_rect_at_origin() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![DrawCommand::FillRect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            color: "#0000ff".to_string(),
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        let corner_pixel = pixmap.pixel(0, 0).unwrap();
        assert_eq!(corner_pixel.blue(), 255, "Pixel at (0,0) should be blue");

        let edge_pixel = pixmap.pixel(9, 9).unwrap();
        assert_eq!(edge_pixel.blue(), 255, "Pixel at (9,9) should be blue");
    }

    // ========================================================================
    // DrawCommand StrokeRect Tests
    // ========================================================================

    #[test]
    fn test_stroke_rect_draws_outline() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::StrokeRect {
            x: 10.0,
            y: 10.0,
            w: 40.0,
            h: 40.0,
            color: "#ffffff".to_string(),
            width: 2.0,
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check a pixel on the edge (should be white)
        let edge_pixel = pixmap.pixel(10, 10).unwrap();
        assert!(edge_pixel.red() > 0 || edge_pixel.green() > 0 || edge_pixel.blue() > 0,
            "Edge pixel should not be black (stroke should be drawn)");

        // Check a pixel in the center (should still be black - it's just an outline)
        let center_pixel = pixmap.pixel(30, 30).unwrap();
        assert_eq!(center_pixel.red(), 0, "Center pixel should be black (not filled)");
        assert_eq!(center_pixel.green(), 0);
        assert_eq!(center_pixel.blue(), 0);
    }

    #[test]
    fn test_stroke_rect_with_width() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![DrawCommand::StrokeRect {
            x: 20.0,
            y: 20.0,
            w: 24.0,
            h: 24.0,
            color: "#ff00ff".to_string(),
            width: 4.0,
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // The stroke should exist on the edge
        let top_edge = pixmap.pixel(32, 20).unwrap();
        assert!(top_edge.red() > 0 || top_edge.blue() > 0, "Top edge should have stroke color");
    }

    // ========================================================================
    // DrawCommand FillCircle Tests
    // ========================================================================

    #[test]
    fn test_fill_circle_draws_circle() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::FillCircle {
            cx: 32.0,
            cy: 32.0,
            r: 15.0,
            color: "#ffff00".to_string(),
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check center of circle (should be yellow)
        let center_pixel = pixmap.pixel(32, 32).unwrap();
        assert_eq!(center_pixel.red(), 255, "Circle center red should be 255");
        assert_eq!(center_pixel.green(), 255, "Circle center green should be 255");
        assert_eq!(center_pixel.blue(), 0, "Circle center blue should be 0");

        // Check corner (should still be black, outside circle)
        let corner_pixel = pixmap.pixel(0, 0).unwrap();
        assert_eq!(corner_pixel.red(), 0, "Corner should be black (outside circle)");
    }

    #[test]
    fn test_fill_circle_at_edge() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![DrawCommand::FillCircle {
            cx: 0.0,
            cy: 0.0,
            r: 20.0,
            color: "#00ffff".to_string(),
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Origin should have the circle color (cyan)
        let origin_pixel = pixmap.pixel(0, 0).unwrap();
        assert_eq!(origin_pixel.green(), 255, "Origin should be cyan (green component)");
        assert_eq!(origin_pixel.blue(), 255, "Origin should be cyan (blue component)");
    }

    // ========================================================================
    // DrawCommand Line Tests
    // ========================================================================

    #[test]
    fn test_line_draws_between_points() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::Line {
            x1: 0.0,
            y1: 0.0,
            x2: 63.0,
            y2: 63.0,
            color: "#ffffff".to_string(),
            width: 2.0,
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check a pixel along the diagonal (should be white)
        let mid_pixel = pixmap.pixel(32, 32).unwrap();
        assert!(mid_pixel.red() > 0 || mid_pixel.green() > 0 || mid_pixel.blue() > 0,
            "Pixel on the line should not be black");

        // Check a pixel far from the line
        let off_line_pixel = pixmap.pixel(0, 63).unwrap();
        assert_eq!(off_line_pixel.red(), 0, "Pixel off the line should be black");
    }

    #[test]
    fn test_line_horizontal() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::Line {
            x1: 10.0,
            y1: 32.0,
            x2: 54.0,
            y2: 32.0,
            color: "#ff0000".to_string(),
            width: 1.0,
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check a pixel on the line
        let on_line = pixmap.pixel(30, 32).unwrap();
        assert!(on_line.red() > 0, "Pixel on horizontal line should be red");
    }

    #[test]
    fn test_line_vertical() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::Line {
            x1: 32.0,
            y1: 10.0,
            x2: 32.0,
            y2: 54.0,
            color: "#00ff00".to_string(),
            width: 1.0,
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check a pixel on the line
        let on_line = pixmap.pixel(32, 30).unwrap();
        assert!(on_line.green() > 0, "Pixel on vertical line should be green");
    }

    #[test]
    fn test_line_with_different_widths() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![DrawCommand::Line {
            x1: 0.0,
            y1: 32.0,
            x2: 64.0,
            y2: 32.0,
            color: "#0000ff".to_string(),
            width: 5.0,
        }];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // A wider line should affect pixels above and below the center
        let on_line = pixmap.pixel(32, 32).unwrap();
        assert!(on_line.blue() > 0, "Center of wide line should be blue");

        // With width 5, pixels at y=30 should also be affected
        let near_line = pixmap.pixel(32, 30).unwrap();
        assert!(near_line.blue() > 0, "Pixel near wide line should also be blue");
    }

    // ========================================================================
    // Multiple Commands Tests
    // ========================================================================

    #[test]
    fn test_multiple_commands_execute_in_order() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![
            // First, fill with red
            DrawCommand::Clear {
                color: "#ff0000".to_string(),
            },
            // Then draw a green rectangle on top
            DrawCommand::FillRect {
                x: 20.0,
                y: 20.0,
                w: 24.0,
                h: 24.0,
                color: "#00ff00".to_string(),
            },
        ];

        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Check corner (should be red from clear)
        let corner = pixmap.pixel(5, 5).unwrap();
        assert_eq!(corner.red(), 255, "Corner should be red");
        assert_eq!(corner.green(), 0, "Corner should not be green");

        // Check center (should be green from rect)
        let center = pixmap.pixel(32, 32).unwrap();
        assert_eq!(center.green(), 255, "Center should be green");
        assert_eq!(center.red(), 0, "Center should not be red");
    }

    // ========================================================================
    // IconRenderer Tests
    // ========================================================================

    #[test]
    fn test_icon_renderer_new() {
        let renderer = IconRenderer::new(64, 12.0);
        // Just verify it creates without panicking
        assert_eq!(renderer.size, 64);
        assert_eq!(renderer.font_size, 12.0);
    }

    #[test]
    fn test_icon_renderer_different_sizes() {
        let small = IconRenderer::new(32, 10.0);
        let large = IconRenderer::new(128, 16.0);

        assert_eq!(small.size, 32);
        assert_eq!(large.size, 128);
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_execute_empty_commands() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        pixmap.fill(Color::from_rgba8(128, 128, 128, 255));

        let commands: Vec<DrawCommand> = vec![];
        renderer.execute_commands(&mut pixmap, &commands).unwrap();

        // Pixmap should be unchanged
        let pixel = pixmap.pixel(32, 32).unwrap();
        assert_eq!(pixel.red(), 128, "Pixmap should be unchanged with empty commands");
    }

    #[test]
    fn test_invalid_color_in_command() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        // Invalid color should be silently ignored
        let commands = vec![DrawCommand::Clear {
            color: "invalid".to_string(),
        }];

        // Should not panic
        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Invalid color should not cause error");

        // Pixmap should be unchanged (invalid color was ignored)
        let pixel = pixmap.pixel(32, 32).unwrap();
        assert_eq!(pixel.red(), 0, "Pixmap should be unchanged with invalid color");
    }

    #[test]
    fn test_zero_dimension_rect() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        // Zero width or height rect - should be handled gracefully
        let commands = vec![DrawCommand::FillRect {
            x: 10.0,
            y: 10.0,
            w: 0.0,
            h: 10.0,
            color: "#ff0000".to_string(),
        }];

        // Should not panic
        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Zero-dimension rect should not cause error");
    }

    #[test]
    fn test_zero_radius_circle() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![DrawCommand::FillCircle {
            cx: 32.0,
            cy: 32.0,
            r: 0.0,
            color: "#ff0000".to_string(),
        }];

        // Should not panic
        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Zero-radius circle should not cause error");
    }

    // ========================================================================
    // Text Alignment Tests
    // ========================================================================

    #[test]
    fn test_text_align_from_str_left() {
        assert_eq!(TextAlign::from_str("left"), TextAlign::Left);
        assert_eq!(TextAlign::from_str("LEFT"), TextAlign::Left);
        assert_eq!(TextAlign::from_str("Left"), TextAlign::Left);
    }

    #[test]
    fn test_text_align_from_str_center() {
        assert_eq!(TextAlign::from_str("center"), TextAlign::Center);
        assert_eq!(TextAlign::from_str("CENTER"), TextAlign::Center);
        assert_eq!(TextAlign::from_str("Center"), TextAlign::Center);
    }

    #[test]
    fn test_text_align_from_str_right() {
        assert_eq!(TextAlign::from_str("right"), TextAlign::Right);
        assert_eq!(TextAlign::from_str("RIGHT"), TextAlign::Right);
        assert_eq!(TextAlign::from_str("Right"), TextAlign::Right);
    }

    #[test]
    fn test_text_align_from_str_default() {
        // Unknown values should default to Left
        assert_eq!(TextAlign::from_str(""), TextAlign::Left);
        assert_eq!(TextAlign::from_str("unknown"), TextAlign::Left);
        assert_eq!(TextAlign::from_str("justify"), TextAlign::Left);
    }

    // ========================================================================
    // Text Rendering Tests
    // ========================================================================

    #[test]
    fn test_render_text_no_font_graceful() {
        // Create renderer without a font
        let renderer = IconRenderer::with_font(64, 12.0, None);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        // Should not panic or error, just skip rendering
        renderer.render_text(
            &mut pixmap,
            "Hello",
            32.0,
            32.0,
            12.0,
            Color::WHITE,
            TextAlign::Left,
        );

        // Pixmap should be unchanged (no font = no rendering)
        let pixel = pixmap.pixel(32, 32).unwrap();
        assert_eq!(pixel.red(), 0, "Pixmap should be unchanged when no font");
    }

    #[test]
    fn test_render_text_empty_string() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        // Empty string should not cause issues
        renderer.render_text(
            &mut pixmap,
            "",
            32.0,
            32.0,
            12.0,
            Color::WHITE,
            TextAlign::Left,
        );

        // Pixmap should be unchanged
        let pixel = pixmap.pixel(32, 32).unwrap();
        assert_eq!(pixel.red(), 0, "Pixmap should be unchanged with empty text");
    }

    #[test]
    fn test_text_command_execution() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        pixmap.fill(Color::from_rgba8(0, 0, 0, 255));

        let commands = vec![DrawCommand::Text {
            text: "Hi".to_string(),
            x: 32.0,
            y: 32.0,
            size: 16.0,
            color: "#ffffff".to_string(),
            align: "center".to_string(),
        }];

        // Should not panic
        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Text command should not cause error");
    }

    #[test]
    fn test_text_command_with_invalid_color() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        pixmap.fill(Color::from_rgba8(128, 128, 128, 255));

        let commands = vec![DrawCommand::Text {
            text: "Test".to_string(),
            x: 32.0,
            y: 32.0,
            size: 12.0,
            color: "invalid".to_string(),
            align: "left".to_string(),
        }];

        // Should not panic, just skip rendering
        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Invalid text color should not cause error");
    }

    #[test]
    fn test_text_command_various_alignments() {
        let renderer = IconRenderer::new(128, 12.0);
        let mut pixmap = Pixmap::new(128, 64).unwrap();

        for align in ["left", "center", "right"] {
            let commands = vec![DrawCommand::Text {
                text: "Test".to_string(),
                x: 64.0,
                y: 32.0,
                size: 14.0,
                color: "#ff0000".to_string(),
                align: align.to_string(),
            }];

            let result = renderer.execute_commands(&mut pixmap, &commands);
            assert!(result.is_ok(), "Text with {} alignment should work", align);
        }
    }

    #[test]
    fn test_icon_renderer_with_font_constructor() {
        // Test the with_font constructor
        let renderer_no_font = IconRenderer::with_font(64, 12.0, None);
        assert!(renderer_no_font.font.is_none(), "Font should be None");
        assert_eq!(renderer_no_font.size, 64);
        assert_eq!(renderer_no_font.font_size, 12.0);
    }

    #[test]
    fn test_text_rendering_does_not_panic_on_special_chars() {
        let renderer = IconRenderer::new(128, 12.0);
        let mut pixmap = Pixmap::new(128, 64).unwrap();

        // Test various special characters
        let test_strings = [
            "Hello!",
            "123",
            "a b c",
            "Ñ",       // Extended ASCII
            "→",       // Arrow
            "...",
            "___",
        ];

        for text in test_strings {
            let commands = vec![DrawCommand::Text {
                text: text.to_string(),
                x: 64.0,
                y: 32.0,
                size: 12.0,
                color: "#ffffff".to_string(),
                align: "center".to_string(),
            }];

            let result = renderer.execute_commands(&mut pixmap, &commands);
            assert!(result.is_ok(), "Text '{}' should not cause error", text);
        }
    }

    #[test]
    fn test_text_rendering_with_zero_size() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        let commands = vec![DrawCommand::Text {
            text: "Test".to_string(),
            x: 32.0,
            y: 32.0,
            size: 0.0,  // Zero font size
            color: "#ffffff".to_string(),
            align: "left".to_string(),
        }];

        // Should not panic
        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Zero font size should not cause error");
    }

    #[test]
    fn test_text_rendering_large_size() {
        let renderer = IconRenderer::new(128, 12.0);
        let mut pixmap = Pixmap::new(128, 128).unwrap();

        let commands = vec![DrawCommand::Text {
            text: "BIG".to_string(),
            x: 64.0,
            y: 100.0,
            size: 48.0,  // Large font size
            color: "#ff0000".to_string(),
            align: "center".to_string(),
        }];

        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Large font size should not cause error");
    }

    #[test]
    fn test_text_rendering_outside_bounds() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();

        // Text positioned outside the pixmap bounds
        let commands = vec![DrawCommand::Text {
            text: "Outside".to_string(),
            x: -100.0,
            y: -100.0,
            size: 12.0,
            color: "#ffffff".to_string(),
            align: "left".to_string(),
        }];

        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Text outside bounds should not cause error");
    }

    #[test]
    fn test_text_with_alpha_color() {
        let renderer = IconRenderer::new(64, 12.0);
        let mut pixmap = Pixmap::new(64, 64).unwrap();
        pixmap.fill(Color::from_rgba8(255, 255, 255, 255));

        let commands = vec![DrawCommand::Text {
            text: "Alpha".to_string(),
            x: 32.0,
            y: 32.0,
            size: 14.0,
            color: "#ff000080".to_string(),  // Red with 50% alpha
            align: "center".to_string(),
        }];

        let result = renderer.execute_commands(&mut pixmap, &commands);
        assert!(result.is_ok(), "Text with alpha should not cause error");
    }
}
