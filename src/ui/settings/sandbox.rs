use gpui::*;
use crate::ui::theme as t;
use super::SettingsPanel;
use super::card::*;

impl SettingsPanel {
    pub(super) fn save_sandbox_settings(&self, cx: &mut Context<Self>) {
        let cpus: i32 = self.sandbox_inputs.cpus.read(cx).value().parse().unwrap_or(2);
        let mem: i64 = self.sandbox_inputs.memory_mb.read(cx).value().parse().unwrap_or(8192);
        let disk: i64 = self.sandbox_inputs.disk_mb.read(cx).value().parse().unwrap_or(16384);

        if cpus < 1 {
            self.toast.update(cx, |t, cx| t.show("CPUs must be at least 1", cx));
            return;
        }
        if mem < 2048 {
            self.toast.update(cx, |t, cx| t.show("Memory must be at least 2048 MB", cx));
            return;
        }
        if disk < 4096 {
            self.toast.update(cx, |t, cx| t.show("Disk must be at least 4096 MB", cx));
            return;
        }

        if let Err(e) = self.db.update_sandbox_settings(cpus, mem, disk) {
            eprintln!("Failed to save sandbox settings: {e}");
            return;
        }
        self.toast.update(cx, |t, cx| t.show("Sandbox settings saved", cx));
    }

    pub(super) fn reset_sandbox_defaults(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.sandbox_inputs.cpus.update(cx, |s, cx| s.set_value("2", cx));
        self.sandbox_inputs.memory_mb.update(cx, |s, cx| s.set_value("8192", cx));
        self.sandbox_inputs.disk_mb.update(cx, |s, cx| s.set_value("16384", cx));
        self.save_sandbox_settings(cx);
    }

    pub(super) fn render_sandbox_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let cpus_input = div()
            .w(px(100.0))
            .bg(t::bg_input())
            .rounded(px(6.0))
            .border_1()
            .border_color(t::bg_input())
            .hover(|s| s.border_color(t::border_subtle()))
            .child(self.sandbox_inputs.cpus.clone());

        let mem_input = div()
            .w(px(100.0))
            .bg(t::bg_input())
            .rounded(px(6.0))
            .border_1()
            .border_color(t::bg_input())
            .hover(|s| s.border_color(t::border_subtle()))
            .child(self.sandbox_inputs.memory_mb.clone());

        let disk_input = div()
            .w(px(100.0))
            .bg(t::bg_input())
            .rounded(px(6.0))
            .border_1()
            .border_color(t::bg_input())
            .hover(|s| s.border_color(t::border_subtle()))
            .child(self.sandbox_inputs.disk_mb.clone());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .w_full()
            .child(section_header("Sandbox Defaults"))
            .child(settings_card(vec![
                settings_row("CPUs", "Number of CPU cores per sandbox", cpus_input).into_any_element(),
                settings_row("Memory (MB)", "RAM allocated per sandbox", mem_input).into_any_element(),
                settings_row("Disk (MB)", "Disk space allocated per sandbox", disk_input).into_any_element(),
            ]))
            .child(
                div()
                    .pt_1()
                    .flex()
                    .gap_2()
                    .child(
                            t::button_primary("Save")
                            .id("save-sandbox")
                            .hover(|s| s.bg(t::bg_hover()).text_color(t::text_primary()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.save_sandbox_settings(cx);
                            })),
                    )
                    .child(
                        t::button("Reset to Default")
                            .id("reset-sandbox")
                            .hover(|s| s.bg(t::bg_hover()).text_color(t::text_tertiary()))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reset_sandbox_defaults(window, cx);
                            })),
                    ),
            )
    }
}
