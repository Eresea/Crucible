use crate::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub panel: Color,
    pub panel_alt: Color,
    pub panel_header: Color,
    pub border: Color,
    pub border_strong: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub warning: Color,
    pub error: Color,
    pub code_keyword: Color,
    pub code_string: Color,
    pub code_number: Color,
    pub code_comment: Color,
    pub code_function: Color,
}

impl Theme {
    #[must_use]
    pub fn minimal_dark() -> Self {
        Self {
            background: Color::rgb_u8(13, 15, 18),
            panel: Color::rgb_u8(22, 25, 30),
            panel_alt: Color::rgb_u8(27, 31, 37),
            panel_header: Color::rgb_u8(31, 35, 42),
            border: Color::rgb_u8(48, 54, 63),
            border_strong: Color::rgb_u8(69, 78, 90),
            text: Color::rgb_u8(226, 231, 238),
            text_muted: Color::rgb_u8(143, 153, 166),
            accent: Color::rgb_u8(74, 163, 255),
            accent_soft: Color::rgba(74.0 / 255.0, 163.0 / 255.0, 255.0 / 255.0, 0.22),
            warning: Color::rgb_u8(232, 174, 74),
            error: Color::rgb_u8(238, 91, 104),
            code_keyword: Color::rgb_u8(128, 183, 255),
            code_string: Color::rgb_u8(130, 205, 161),
            code_number: Color::rgb_u8(232, 190, 117),
            code_comment: Color::rgb_u8(110, 122, 137),
            code_function: Color::rgb_u8(182, 158, 255),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::minimal_dark()
    }
}
