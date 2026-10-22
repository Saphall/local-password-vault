use std::sync::Arc;

use eframe::egui::{self, FontData, FontFamily, RichText};

// Phosphor icons (MIT) — codepoints from the font's regular/fill variants.
pub const LOCK_KEY: &str = "\u{E2FE}";
pub const LOCK_KEY_OPEN: &str = "\u{E300}";
pub const STAR: &str = "\u{E46A}";
pub const KEY: &str = "\u{E2D6}";
pub const EYE: &str = "\u{E220}";
pub const EYE_SLASH: &str = "\u{E224}";
pub const COPY: &str = "\u{E1CA}";
pub const PENCIL: &str = "\u{E3B4}";
pub const TRASH: &str = "\u{E4A6}";
pub const PLUS: &str = "\u{E3D4}";
pub const SEARCH: &str = "\u{E30C}";
pub const TAG: &str = "\u{E478}";
pub const GLOBE: &str = "\u{E288}";
pub const SHIELD: &str = "\u{E40C}";
pub const CLOSE: &str = "\u{E4F6}";
pub const HISTORY: &str = "\u{E1A0}";
pub const LIST: &str = "\u{E2F0}";
pub const USER: &str = "\u{E4C2}";
pub const WAND: &str = "\u{E6B6}";
pub const SORT: &str = "\u{E444}";
pub const CHECK: &str = "\u{E184}";
pub const WARNING: &str = "\u{E4E2}";
pub const VAULT: &str = "\u{E76E}";

const FILL_FAMILY: &str = "phosphor-fill";

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "phosphor".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Phosphor.ttf"))),
    );
    fonts.font_data.insert(
        FILL_FAMILY.into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/fonts/Phosphor-Fill.ttf"))),
    );

    // Last in the fallback chain so it never shadows the text font.
    if let Some(p) = fonts.families.get_mut(&FontFamily::Proportional) {
        p.push("phosphor".into());
    }
    fonts
        .families
        .insert(FontFamily::Name(FILL_FAMILY.into()), vec![FILL_FAMILY.into()]);
    ctx.set_fonts(fonts);
}

/// Icon followed by a label, for buttons and headings.
pub fn with_text(icon: &str, text: &str) -> String {
    format!("{icon}  {text}")
}

pub fn filled(icon: &str) -> RichText {
    RichText::new(icon).family(FontFamily::Name(FILL_FAMILY.into()))
}
