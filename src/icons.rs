use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontFamily, RichText};

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
pub const HISTORY: &str = "\u{E1A0}";
pub const LIST: &str = "\u{E2F0}";
pub const USER: &str = "\u{E4C2}";
pub const WAND: &str = "\u{E6B6}";
pub const SORT: &str = "\u{E444}";
pub const CHECK: &str = "\u{E184}";
pub const WARNING: &str = "\u{E4E2}";
pub const VAULT: &str = "\u{E76E}";
pub const OPEN: &str = "\u{E5DE}";
pub const SUN: &str = "\u{E472}";
pub const MOON: &str = "\u{E330}";

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
    RichText::new(icon).family(fill_family())
}

pub fn fill_family() -> FontFamily {
    FontFamily::Name(FILL_FAMILY.into())
}

/// Logo tile for a known site. `glyph: None` means draw the first letter on the brand color.
pub struct Brand {
    pub glyph: Option<&'static str>,
    pub bg: Color32,
    pub fg: Color32,
}

fn logo(glyph: &'static str, r: u8, g: u8, b: u8) -> Brand {
    Brand { glyph: Some(glyph), bg: Color32::from_rgb(r, g, b), fg: Color32::WHITE }
}

fn letter(r: u8, g: u8, b: u8) -> Brand {
    Brand { glyph: None, bg: Color32::from_rgb(r, g, b), fg: Color32::WHITE }
}

/// Matches whole words in the name/URL, so `box.com` does not match `x`. Fully offline.
pub fn brand_for(name: &str, url: &str) -> Option<Brand> {
    format!("{name} {url}")
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find_map(lookup)
}

fn lookup(word: &str) -> Option<Brand> {
    Some(match word {
        "github" => logo("\u{E576}", 48, 54, 61),
        "gitlab" => logo("\u{E694}", 252, 109, 38),
        "google" | "gmail" => logo("\u{E292}", 66, 133, 244),
        "youtube" => logo("\u{E4FC}", 255, 0, 0),
        "facebook" | "fb" => logo("\u{E226}", 24, 119, 242),
        "messenger" => logo("\u{E6D8}", 0, 132, 255),
        "meta" => logo("\u{ED02}", 0, 100, 224),
        "instagram" => logo("\u{E2D0}", 225, 48, 108),
        "twitter" => logo("\u{E4BA}", 29, 155, 240),
        "x" => logo("\u{E4BC}", 30, 30, 30),
        "linkedin" => logo("\u{E2EE}", 10, 102, 194),
        "reddit" => logo("\u{E59C}", 255, 69, 0),
        "amazon" => logo("\u{E96C}", 255, 153, 0),
        "apple" | "icloud" => logo("\u{E516}", 70, 70, 74),
        "microsoft" | "windows" => logo("\u{E692}", 0, 120, 212),
        "outlook" | "hotmail" | "live" => logo("\u{EB70}", 0, 120, 212),
        "teams" => logo("\u{EB66}", 98, 100, 167),
        "discord" => logo("\u{E61A}", 88, 101, 242),
        "slack" => logo("\u{E5A8}", 224, 30, 90),
        "spotify" => logo("\u{E66E}", 30, 185, 84),
        "twitch" => logo("\u{E5CE}", 145, 70, 255),
        "whatsapp" => logo("\u{E5D0}", 37, 211, 102),
        "telegram" => logo("\u{E5BC}", 34, 158, 217),
        "dropbox" => logo("\u{E7D0}", 0, 97, 255),
        "paypal" => logo("\u{E98C}", 0, 112, 186),
        "pinterest" => logo("\u{E64E}", 230, 0, 35),
        "tiktok" => logo("\u{EAF2}", 30, 30, 30),
        "snapchat" => Brand { fg: Color32::BLACK, ..logo("\u{E668}", 255, 252, 0) },
        "steam" => logo("\u{EAD4}", 42, 71, 94),
        "figma" => logo("\u{E22E}", 162, 89, 255),
        "notion" => logo("\u{E9A0}", 50, 50, 50),
        "medium" => logo("\u{E322}", 40, 40, 40),
        "stripe" => logo("\u{E698}", 99, 91, 255),
        "openai" | "chatgpt" => logo("\u{E7D2}", 16, 163, 127),
        "stackoverflow" => logo("\u{EB78}", 244, 128, 36),
        "netflix" => letter(229, 9, 20),
        "yahoo" => letter(96, 1, 210),
        _ => return None,
    })
}

/// Stable color per name for sites without a known logo.
pub fn tint_for(name: &str) -> Color32 {
    const PALETTE: [Color32; 8] = [
        Color32::from_rgb(147, 51, 234),
        Color32::from_rgb(59, 130, 246),
        Color32::from_rgb(16, 185, 129),
        Color32::from_rgb(245, 158, 11),
        Color32::from_rgb(239, 68, 68),
        Color32::from_rgb(236, 72, 153),
        Color32::from_rgb(20, 184, 166),
        Color32::from_rgb(99, 102, 241),
    ];
    let hash = name.to_lowercase().bytes().fold(7usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize));
    PALETTE[hash % PALETTE.len()]
}
