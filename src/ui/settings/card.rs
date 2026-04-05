use gpui::*;
use crate::ui::theme as t;

/// Section header above a card.
pub fn section_header(title: &str) -> impl IntoElement {
    div()
        .pb(px(10.0))
        .child(
            div()
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .text_color(t::text_secondary())
                .child(title.to_string()),
        )
}

/// Wrap rows in a rounded card with subtle border and dividers between rows.
pub fn settings_card(rows: Vec<AnyElement>) -> impl IntoElement {
    let count = rows.len();
    let mut card = div()
        .rounded(px(8.0))
        .border_1()
        .border_color(t::border_subtle())
        .bg(t::bg_elevated())
        .flex()
        .flex_col();

    for (i, row) in rows.into_iter().enumerate() {
        card = card.child(row);
        if i < count - 1 {
            card = card.child(
                div()
                    .mx_4()
                    .h(px(1.0))
                    .bg(t::border_subtle()),
            );
        }
    }
    card
}

/// A single row inside a settings card: label+description on the left, control on the right.
pub fn settings_row(
    title: &str,
    description: &str,
    control: impl IntoElement,
) -> impl IntoElement {
    div()
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t::text_secondary())
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(t::text_ghost())
                        .child(description.to_string()),
                ),
        )
        .child(control)
}
