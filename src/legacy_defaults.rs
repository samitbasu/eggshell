// #497 stage 3: delete, with the XDefaults writers this feeds.
//! The Defaults-layer rules the widgets' own writers make from egui's
//! default visuals, which a preset's look is still written over.

use std::time::Duration;

use egui::{Color32, FontFamily, vec2};

use crate::contract::{Colors, Keyed, Look, Metrics, Paint, Size, States};
use crate::sheet::Sheet;
use crate::{Opacity, TextSpec, Tracking, Weight};

pub(crate) fn write(sheet: &mut Sheet) {
    let egui = &egui::Style::default();
    let (v, s) = (&egui.visuals, &egui.spacing);
    let colors = |w: &egui::style::WidgetVisuals| Colors {
        fill: w.weak_bg_fill,
        border: w.bg_stroke.color,
        text: w.fg_stroke.color,
    };
    let w = &v.widgets;
    let (inactive, hovered, active) = (colors(&w.inactive), colors(&w.hovered), colors(&w.active));
    let focus_border = v.selection.stroke.color;
    let faded = Opacity::new(v.disabled_alpha);
    let surface = v.panel_fill;
    let states = |rest: Colors, hover: Colors, pressed: Colors| States {
        rest,
        hover,
        pressed,
        focused: Colors {
            border: focus_border,
            ..rest
        },
        disabled: rest.faded_over(faded, surface),
    };
    let button_states = states(inactive, hovered, active);
    let radius = w.inactive.corner_radius.nw;
    let border_width = w.inactive.bg_stroke.width;
    let font = egui
        .text_styles
        .get(&egui::TextStyle::Button)
        .cloned()
        .unwrap_or_default();
    let text = TextSpec {
        size: font.size,
        line_height: (font.size * 1.25).round(),
        family: FontFamily::Proportional,
        weight: Weight::Normal,
    };
    let metrics = Metrics {
        height: s.interact_size.y,
        min_width: 0.0,
        padding_x: s.button_padding.x,
        gap: s.item_spacing.x,
        radius,
        glyph: text.size,
        text: None,
    };
    let clear = Color32::TRANSPARENT;
    let selected = Colors {
        fill: v.selection.bg_fill,
        border: v.selection.bg_fill,
        text: v.selection.stroke.color,
    };
    let choice_border = border_width.max(1.0);
    let choice_off = Paint::new(states(
        Colors {
            fill: v.extreme_bg_color,
            border: w.inactive.bg_stroke.color,
            text: v.text_color(),
        },
        hovered,
        hovered,
    ));
    let choice_on = Paint::new(states(selected, selected, selected));
    let transition = Duration::from_millis(100);
    crate::owned::select_defaults(
        sheet,
        crate::owned::SelectDefaults {
            trigger: Look::new(button_states).bordered(border_width.max(1.0)),
            radius: metrics.radius,
            transition,
        },
    );
    crate::owned::menu_defaults(
        sheet,
        crate::owned::MenuDefaults {
            popup: Look::new(States::still(
                Colors {
                    fill: v.window_fill,
                    border: v.window_stroke.color,
                    text: v.text_color(),
                },
                v.window_fill,
            ))
            .bordered(v.window_stroke.width.max(1.0))
            .shadowed(v.popup_shadow),
            radius,
            min_width: s.combo_width,
            item: Look::new(States::new(
                Colors {
                    fill: clear,
                    border: clear,
                    text: v.text_color(),
                },
                selected,
                v.window_fill,
            )),
            item_radius: radius,
            destructive: States::new(
                Colors {
                    fill: clear,
                    border: clear,
                    text: v.error_fg_color,
                },
                Colors {
                    fill: v.error_fg_color.gamma_multiply(0.15),
                    border: clear,
                    text: v.error_fg_color,
                },
                v.window_fill,
            ),
            separator: w.noninteractive.bg_stroke.color,
            separator_margin: s.item_spacing.y,
            heading: v.weak_text_color(),
            heading_weight: Weight::Normal,
            shortcut: v.weak_text_color(),
            shortcut_tracking: Tracking::ZERO,
        },
    );
    crate::owned::input_defaults(
        sheet,
        crate::owned::InputDefaults {
            look: Look::new(States {
                rest: Colors {
                    fill: v.text_edit_bg_color(),
                    border: w.inactive.bg_stroke.color,
                    text: w.inactive.fg_stroke.color,
                },
                hover: Colors {
                    fill: v.text_edit_bg_color(),
                    border: w.hovered.bg_stroke.color,
                    text: w.inactive.fg_stroke.color,
                },
                pressed: Colors {
                    fill: v.text_edit_bg_color(),
                    border: focus_border,
                    text: w.inactive.fg_stroke.color,
                },
                focused: Colors {
                    fill: v.text_edit_bg_color(),
                    border: focus_border,
                    text: w.inactive.fg_stroke.color,
                },
                disabled: Colors {
                    fill: v.text_edit_bg_color(),
                    border: w.inactive.bg_stroke.color,
                    text: w.inactive.fg_stroke.color,
                }
                .faded_over(faded, surface),
            })
            .bordered(border_width),
            radius,
            padding: s.button_padding,
            height: s.interact_size.y,
        },
    );
    crate::owned::tabs_defaults(
        sheet,
        crate::owned::TabsDefaults {
            selected: Paint::new(States::still(
                Colors {
                    fill: w.active.weak_bg_fill,
                    border: clear,
                    text: v.strong_text_color(),
                },
                surface,
            )),
            unselected: Paint::new(States::new(
                Colors {
                    fill: clear,
                    border: clear,
                    text: v.weak_text_color(),
                },
                hovered,
                surface,
            )),
            cell_border_width: 0.0,
            cell_radius: radius,
            transition,
        },
    );
    crate::owned::choice_defaults(
        sheet,
        crate::owned::ChoiceDefaults {
            checkbox: crate::owned::ChoicePaint {
                off: choice_off,
                on: choice_on,
                border_width: choice_border,
                transition,
            },
            checkbox_radius: 2,
            radio: crate::owned::ChoicePaint {
                off: choice_off,
                on: choice_on,
                border_width: choice_border,
                transition,
            },
            switch: crate::owned::ChoicePaint {
                off: choice_off,
                on: choice_on,
                border_width: choice_border,
                transition,
            },
        },
    );
    crate::owned::table_defaults(sheet, crate::owned::TableDefaults::from_egui(egui));
    crate::owned::tooltip_defaults(
        sheet,
        crate::owned::TooltipDefaults {
            popup: Look::new(States::still(
                Colors {
                    fill: v.window_fill,
                    border: v.window_stroke.color,
                    text: v.text_color(),
                },
                v.window_fill,
            ))
            .bordered(v.window_stroke.width.max(1.0))
            .shadowed(v.popup_shadow),
            radius,
            padding: vec2(f32::from(s.menu_margin.left), f32::from(s.menu_margin.top)),
            max_width: s.tooltip_width,
        },
    );
    crate::owned::button_defaults(
        sheet,
        crate::owned::ButtonDefaults {
            look: Look::new(button_states).bordered(border_width),
            sizes: Keyed::new(Size::DEFAULT, metrics),
            transition,
            press_shift: 0.0,
        },
    );
}
