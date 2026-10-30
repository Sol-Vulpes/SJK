//! SJK-style reaction controls for a selected public chat message.
use super::*;
use crate::menu::sjk::{Frame, kit};

pub(super) const BASE: u16 = 1_600;

pub(super) fn draw(
    panel: &mut Panel,
    frame: &Frame,
    message: &sjk_identity::ChatMessage,
    area: [f32; 4],
    enabled: bool,
    own: Option<&str>,
) {
    panel.shown.reaction_mine = [false; 6];
    if !enabled || message.holocron.is_some() || message.official.is_some() {
        return;
    }
    let (counts, mine) = sjk_identity::reactions::counts(&message.reactions, own);
    panel.shown.reaction_mine = mine;
    let [x, y, width, height] = area;
    let cell = (width - 5.0 * 6.0) / 6.0;
    for (at, (_, emoji)) in sjk_identity::reactions::CATALOG.iter().enumerate() {
        let token = BASE + at as u16;
        // The kit takes a borrowed label; bounded stack formatting avoids allocating
        // one label per reaction every frame.
        let label = Label::new(emoji, counts[at]);
        kit::button(
            &mut panel.ui,
            frame,
            [x + at as f32 * (cell + 6.0), y, cell, height],
            label.as_str(),
            mine[at],
            enabled && own.is_some(),
            panel.focus == token,
            token,
        );
        if enabled && own.is_some() {
            panel.order.push(token);
        }
    }
}

pub(super) fn activate(panel: &mut Panel, token: u16) -> Option<PanelAction> {
    let at = usize::from(token.checked_sub(BASE)?);
    let (emoji, _) = *sjk_identity::reactions::CATALOG.get(at)?;
    if !panel.order.contains(&token) {
        return Some(PanelAction::None);
    }
    let Some((message, _, _, muted)) = panel.shown.chosen.as_ref() else {
        return Some(PanelAction::None);
    };
    if *muted {
        return Some(PanelAction::None);
    }
    panel.focus = token;
    Some(PanelAction::React {
        message: *message,
        emoji: emoji.to_owned(),
        active: !panel.shown.reaction_mine[at],
    })
}

struct Label {
    bytes: [u8; 32],
    len: usize,
}
impl Label {
    fn new(emoji: &str, count: usize) -> Self {
        use std::fmt::Write;
        let mut label = Self {
            bytes: [0; 32],
            len: 0,
        };
        let _ = write!(label, "{emoji} {count}");
        label
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}
impl std::fmt::Write for Label {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        if self.len + text.len() > self.bytes.len() {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..self.len + text.len()].copy_from_slice(text.as_bytes());
        self.len += text.len();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tests::{chat, drawn, inputs};
    #[test]
    fn the_selected_message_can_remove_its_own_reaction_without_sending_chat() {
        let mut state = chat(1);
        state.own_key = Some(*b"0123456789abcdef");
        state.messages[0].reactions = vec![sjk_identity::reactions::Reaction {
            key_id: "0123456789abcdef".into(),
            emoji: "heart".into(),
        }]
        .into();
        let mut panel = Panel::new();
        panel.open(true);
        panel.selected = Some(1);
        drawn(&mut panel, &inputs(&state, &[], false));
        assert!(panel.order.contains(&(BASE + 1)));
        assert!(
            matches!(panel.activate(BASE+1),PanelAction::React{message:1,ref emoji,active:false} if emoji=="heart")
        );
        assert!(panel.draft.is_empty());
        assert!(panel.last_sent.is_empty());
        let blocked = vec![state.messages[0].key_id.clone()];
        drawn(&mut panel, &inputs(&state, &blocked, false));
        assert!(!panel.order.contains(&(BASE + 1)));
        assert!(matches!(panel.activate(BASE + 1), PanelAction::None));
    }
    #[test]
    fn the_empty_reaction_is_an_add_and_a_disabled_control_does_nothing() {
        let mut state = chat(1);
        state.own_key = Some(*b"0123456789abcdef");
        let mut panel = Panel::new();
        panel.open(true);
        panel.selected = Some(1);
        drawn(&mut panel, &inputs(&state, &[], false));
        assert!(matches!(
            panel.activate(BASE),
            PanelAction::React {
                message: 1,
                active: true,
                ..
            }
        ));
        state.own_key = None;
        drawn(&mut panel, &inputs(&state, &[], false));
        assert!(matches!(panel.activate(BASE), PanelAction::None));
        assert_eq!(Label::new("??", 128).as_str(), "?? 128");
    }
}
