use gpui::*;
use super::SettingsPanel;
use super::card::*;
use super::dropdown::{DropdownEvent, DropdownItem, DropdownState};

impl SettingsPanel {
    pub(super) fn init_agent_dropdown(
        agents: &[crate::db::Agent],
        selected: Option<i64>,
        cx: &mut Context<Self>,
    ) -> Entity<DropdownState> {
        let items: Vec<DropdownItem> = agents
            .iter()
            .map(|a| DropdownItem {
                id: a.id,
                label: a.display_name.clone(),
                icon: a.icon.clone(),
            })
            .collect();

        let state = cx.new(|cx| DropdownState::new(items, selected, cx));

        cx.subscribe(&state, |this: &mut Self, _, event: &DropdownEvent, cx| {
            let DropdownEvent::Change(value) = event;
            this.default_agent_id = *value;
            if let Err(e) = this.db.update_default_agent(*value) {
                eprintln!("Failed to save default agent: {e}");
            }
            cx.notify();
        })
        .detach();

        state
    }

    pub(super) fn render_general_tab(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .w_full()
            .child(section_header("General"))
            .child(settings_card(vec![
                settings_row(
                    "Default Agent",
                    "Opens automatically for new workspaces",
                    self.agent_dropdown.clone(),
                )
                .into_any_element(),
            ]))
    }
}
