//! The chat's style, `cg_chatStyle` (Sol, 23/11/2026): `sjk`, SJK's chat (the composer
//! with its channel buttons, sender cards, emblems and labels, lines that slide and
//! fade), or `classic`, JoF EJK's (EternalJK's): the message field at the top left
//! under the notify lines, the chat box laid out as `CG_ChatBox_DrawStrings` lays it,
//! plain names, retail's drop shadow, no pointer. The chat code branches on this and
//! nothing else: EJK mode (`ejk_mode.rs`) only sets the cvar.
//!
//! [`classic`] holds JoF EJK's numbers, each with the source line it comes from; the
//! drawing is `chat/view/classic.rs`.

use crate::console::ViewerConsole;

/// Archived, `sjk` by default.
pub(crate) const CVAR: &str = "cg_chatStyle";

/// How the chat box and the chat field are drawn and behave.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Style {
    /// SJK's chat.
    #[default]
    Sjk,
    /// JoF EJK's chat.
    Classic,
}

impl Style {
    /// The values the cvar and its Settings row take.
    pub(crate) const NAMES: [&'static str; 2] = ["sjk", "classic"];
    /// A new profile's.
    pub(crate) const DEFAULT_NAME: &'static str = Self::NAMES[0];

    /// The style `value` names, whatever its case; anything else is SJK's.
    pub(crate) fn parse(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case(Self::NAMES[1]) {
            Self::Classic
        } else {
            Self::Sjk
        }
    }

    /// The style [`CVAR`] names in `console`; SJK's without one.
    pub(crate) fn read(console: Option<&ViewerConsole>) -> Self {
        console
            .and_then(|console| console.text_value(CVAR))
            .map_or(Self::Sjk, Self::parse)
    }

    /// The cvar's value for this style.
    #[cfg(test)]
    pub(crate) fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}

/// JoF EJK's chat in numbers. Lengths are in the game's virtual 640x480 units, a unit
/// being a 480th of the frame's height both ways: with `cl_ratioFix` (on by default,
/// `client/cl_main.cpp:4206`) JoF EJK multiplies every horizontal length here by
/// `widthRatioCoef` (`cl_main.cpp:2905`), which gives a 640-unit length the same
/// pixels as a 480-unit one. Paths are JoF EJK's `codemp/`, lines at
/// JediofFreedom/JoF_EJK `beta` 10b940314.
pub(crate) mod classic {
    /// `CG_ChatBox_DrawStrings` draws at `0.65 * cg_chatBoxFontSize`
    /// (`cgame/cg_draw.c:11126`).
    pub(crate) const FONT_SCALE: f32 = 0.65;
    /// `ocr_a`'s line box (`mHeight`, `fonts/ocr_a.fontdat`), which SJK Chat keeps.
    pub(crate) const LINE: f32 = 21.0;
    /// `ocr_a`'s ascender (`mAscender`): the baseline below the line box's top as SJK
    /// Chat is laid out.
    pub(crate) const ASCENDER: f32 = 17.0;
    /// `ocr_a`'s descender (`mDescender`).
    pub(crate) const DESCENDER: i32 = 4;
    /// `ocr_a`'s point size (`mPointSize`): what a line feed inside one string
    /// advances by (`rd-common/tr_font.cpp:1621`), so the rows of one wrapped message.
    pub(crate) const ROW: f32 = 18.0;
    /// `CHATBOX_FONT_HEIGHT` (`cg_draw.c:10258`): what the box gives each row of a
    /// message, so a message of two rows takes 40 and its second row sits 18 under the
    /// first (`cg_draw.c:11167`, `11215`).
    pub(crate) const ITEM_ROW: f32 = 20.0;
    /// Where `RE_Font_DrawString` puts the baseline under the y it is given:
    /// `GetHeight() - (GetDescender() >> 1)` (`tr_font.cpp:1608`), 19 for `ocr_a`.
    pub(crate) const BASELINE: f32 = LINE - (DESCENDER >> 1) as f32;
    /// A client icon before an SJK chat message: its side, the gap after it, and how
    /// far under the row's y it sits (`3.7 * fontScale + 2`), `cg_draw.c:11174-11178`.
    pub(crate) const ICON: f32 = 17.0;
    pub(crate) const ICON_GAP: f32 = 3.0;
    pub(crate) const ICON_DROP: (f32, f32) = (3.7, 2.0);
    /// SJK chat's golds as JoF EJK draws them (`cg_draw.c:10907-10908`): the label
    /// after the sender's name, `#E8B84A`, and the text, `#F5C756`.
    pub(crate) const SJK_LABEL: [f32; 3] = [0.9098, 0.7216, 0.2902];
    pub(crate) const SJK_TEXT: [f32; 3] = [0.9608, 0.7804, 0.3373];

    /// The message field's characters: `BIGCHAR_HEIGHT` (`qcommon/q_shared.h:632`).
    pub(crate) const FIELD_LINE: f32 = 16.0;
    /// `BIGCHAR_WIDTH` (`q_shared.h:631`): a character's cell in JoF EJK, and the room
    /// left after the prompt and for the whisper's field.
    pub(crate) const FIELD_CELL: f32 = 16.0;
    /// Where the prompt starts (`SCR_DrawStringExt2(8 * widthRatioCoef, v, ...)`,
    /// `client/cl_console.cpp:1152`).
    pub(crate) const FIELD_LEFT: f32 = 8.0;
    /// Cells the field leaves at its right for the HUD's upper right
    /// (`CHAT_UPPER_RIGHT_CHARS`, `cl_console.cpp:996`).
    pub(crate) const FIELD_RIGHT_CELLS: f32 = 11.0;
    /// The cursor shows while `(realtime >> 8) & 1` is 0 (`client/cl_keys.cpp:565`):
    /// 256 ms on, 256 off.
    pub(crate) const BLINK_SHIFT: u32 = 8;

    /// Whether the field's cursor shows `ms` after any fixed moment.
    pub(crate) fn cursor_shows(ms: u64) -> bool {
        (ms >> BLINK_SHIFT) & 1 == 0
    }

    /// Pixels of one virtual unit in a frame `viewport_height` pixels tall.
    pub(crate) fn unit(viewport_height: f32) -> f32 {
        viewport_height / 480.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_style_is_sjk_unless_classic_is_named() {
        for (value, style) in [
            ("sjk", Style::Sjk),
            ("classic", Style::Classic),
            (" Classic ", Style::Classic),
            ("CLASSIC", Style::Classic),
            ("", Style::Sjk),
            ("1", Style::Sjk),
            ("ejk", Style::Sjk),
        ] {
            assert_eq!(Style::parse(value), style, "{value:?}");
        }
        for style in [Style::Sjk, Style::Classic] {
            assert_eq!(Style::parse(style.name()), style);
        }
        assert_eq!(Style::default(), Style::Sjk);
        assert_eq!(Style::DEFAULT_NAME, "sjk");
    }

    #[test]
    fn the_cvar_is_sjk_in_a_new_profile_and_follows_the_console() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(Style::read(None), Style::Sjk);
        assert_eq!(Style::read(Some(&console)), Style::Sjk);
        assert!(console.set_cvar(CVAR, "classic"));
        assert_eq!(Style::read(Some(&console)), Style::Classic);
        assert!(console.set_cvar(CVAR, "sjk"));
        assert_eq!(Style::read(Some(&console)), Style::Sjk);
    }

    /// JoF EJK's numbers, from its source (the module's comments name the lines).
    #[test]
    fn the_classic_numbers_are_jof_ejks() {
        // cg_draw.c:10258, 11126; ocr_a.fontdat: point 18, height 21, ascender 17,
        // descender 4; tr_font.cpp:1608, 1621.
        assert_eq!(classic::FONT_SCALE, 0.65);
        assert_eq!(classic::ITEM_ROW, 20.0);
        assert_eq!(classic::ROW, 18.0);
        assert_eq!(classic::BASELINE, 19.0);
        // q_shared.h:631-632; cl_console.cpp:996, 1152; cl_keys.cpp:565.
        assert_eq!((classic::FIELD_LINE, classic::FIELD_CELL), (16.0, 16.0));
        assert_eq!(classic::FIELD_LEFT, 8.0);
        assert_eq!(classic::FIELD_RIGHT_CELLS, 11.0);
        assert!(classic::cursor_shows(0) && classic::cursor_shows(255));
        assert!(!classic::cursor_shows(256) && !classic::cursor_shows(511));
        assert!(classic::cursor_shows(512));
        // 2.25 pixels a unit at 1080 lines, 4.5 at 2160.
        assert_eq!(classic::unit(1080.0), 2.25);
        assert_eq!(classic::unit(2160.0), 4.5);
    }
}
