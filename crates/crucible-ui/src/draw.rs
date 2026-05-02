use crate::{Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Color {
    #[must_use]
    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    #[must_use]
    pub fn rgb_u8(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            1.0,
        )
    }

    #[must_use]
    pub fn to_array(self) -> [f32; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }

    #[must_use]
    pub fn to_rgb_u8(self) -> [u8; 3] {
        [
            (self.red.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.green.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.blue.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct TextPrimitive {
    pub text: String,
    pub position: Point,
    pub size: f32,
    pub color: Color,
    pub bounds: Rect,
    pub monospace: bool,
}

#[derive(Debug, Clone)]
pub enum DrawPrimitive {
    Rect {
        rect: Rect,
        color: Color,
    },
    Border {
        rect: Rect,
        color: Color,
        width: f32,
    },
    Line {
        from: Point,
        to: Point,
        width: f32,
        color: Color,
    },
    Text(TextPrimitive),
}

#[derive(Debug, Clone, Default)]
pub struct DrawList {
    primitives: Vec<DrawPrimitive>,
}

impl DrawList {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn primitives(&self) -> &[DrawPrimitive] {
        &self.primitives
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    pub fn rect(&mut self, rect: Rect, color: Color) {
        if rect.width > 0.0 && rect.height > 0.0 && color.alpha > 0.0 {
            self.primitives.push(DrawPrimitive::Rect { rect, color });
        }
    }

    pub fn border(&mut self, rect: Rect, color: Color, width: f32) {
        if rect.width > 0.0 && rect.height > 0.0 && width > 0.0 && color.alpha > 0.0 {
            self.primitives
                .push(DrawPrimitive::Border { rect, color, width });
        }
    }

    pub fn line(&mut self, from: Point, to: Point, width: f32, color: Color) {
        if width > 0.0 && color.alpha > 0.0 {
            self.primitives.push(DrawPrimitive::Line {
                from,
                to,
                width,
                color,
            });
        }
    }

    pub fn text(
        &mut self,
        text: impl Into<String>,
        position: Point,
        size: f32,
        color: Color,
        bounds: Rect,
    ) {
        self.push_text(text, position, size, color, bounds, false);
    }

    pub fn text_mono(
        &mut self,
        text: impl Into<String>,
        position: Point,
        size: f32,
        color: Color,
        bounds: Rect,
    ) {
        self.push_text(text, position, size, color, bounds, true);
    }

    fn push_text(
        &mut self,
        text: impl Into<String>,
        position: Point,
        size: f32,
        color: Color,
        bounds: Rect,
        monospace: bool,
    ) {
        let text = text.into();
        if text.is_empty() || size <= 0.0 || color.alpha <= 0.0 {
            return;
        }

        self.primitives.push(DrawPrimitive::Text(TextPrimitive {
            text,
            position,
            size,
            color,
            bounds,
            monospace,
        }));
    }
}
