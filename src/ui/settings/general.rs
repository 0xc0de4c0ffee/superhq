use gpui::*;
use super::SettingsPanel;
use super::card::*;
use crate::ui::components::select::{Select, SelectItem, SelectEvent};
use crate::ui::components::switch::{Switch, SwitchEvent};

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

    /// Generic helper: build a `Switch` and forward its changes to a callback.
    pub(super) fn init_switch<F>(
        value: bool,
        on_change: F,
        cx: &mut Context<Self>,
    ) -> Entity<Switch>
    where
        F: Fn(&mut Self, bool, &mut Context<Self>) + 'static,
    {
        let state = cx.new(|cx| Switch::new(value, cx));
        cx.subscribe(&state, move |this, _, event: &SwitchEvent, cx| {
            let SwitchEvent::Change(value) = *event;
            on_change(this, value, cx);
        })
        .detach();
        state
    }

    pub(super) fn init_auto_launch_switch(
        value: bool,
        cx: &mut Context<Self>,
    ) -> Entity<Switch> {
        Self::init_switch(value, |this, value, _cx| {
            if let Err(e) = this.db.update_auto_launch_agent(value) {
                eprintln!("Failed to save auto_launch_agent: {e}");
            }
        }, cx)
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
                    "Agent to open for new workspaces",
                    self.agent_dropdown.clone(),
                )
                .into_any_element(),
                settings_row(
                    "Auto-launch agent",
                    "Automatically start the default agent when opening a workspace",
                    self.auto_launch_switch.clone(),
                )
                .into_any_element(),
            ]))
    }
}
