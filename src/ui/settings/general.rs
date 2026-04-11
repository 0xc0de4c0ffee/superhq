use gpui::*;
use super::SettingsPanel;
use super::card::*;
use crate::ui::components::select::{Select, SelectItem, SelectEvent};

impl SettingsPanel {
    pub(super) fn init_agent_dropdown(
        agents: &[crate::db::Agent],
        selected: Option<i64>,
        cx: &mut Context<Self>,
    ) -> Entity<Select> {
        let items: Vec<SelectItem> = agents
            .iter()
            .map(|a| SelectItem {
                id: a.id,
                label: a.display_name.clone(),
                icon: a.icon.clone(),
            })
            .collect();

        let state = cx.new(|cx| Select::new(items, selected, cx));

        cx.subscribe(&state, |this: &mut Self, _, event: &SelectEvent, cx| {
            let SelectEvent::Change(value) = event;
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
