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
#[allow(dead_code)]
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

#[allow(dead_code)]
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
        let _display_name = if name.len() > max_chars {
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
}
