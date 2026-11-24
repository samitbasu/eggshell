//! Prints what the preset makes of shadcn, one `key=value` line per fact,
//! for `cargo xtask shadcn-import` to compare with shadcn's source: the
//! tokens, and the button's, badge's and combobox's metrics and colours, per
//! style and theme. Colours print as egui's `Debug` does, premultiplied:
//! `#RR_GG_BB_AA`.

use eggshell_ui::contract::{Colors, Look, Variant};
use eggshell_ui::owned::{InputPart, MenuPart};
use eggshell_ui::sheet::Key;
use eggshell_ui_shadcn::{Shadcn, ShadcnStyle};
use egui::Theme;

fn colors(prefix: &str, state: &str, c: &Colors) {
    println!(
        "{prefix} state={state} fill={:?} border={:?} text={:?}",
        c.fill, c.border, c.text
    );
}

fn look(prefix: &str, look: &Look) {
    let states = &look.paint.states;
    colors(prefix, "rest", &states.rest);
    colors(prefix, "hover", &states.hover);
    println!("{prefix} state=rest border_width={}", look.border_width);
}

fn main() {
    for style in ShadcnStyle::ALL {
        for theme in [Theme::Light, Theme::Dark] {
            let preset = Shadcn::for_theme(theme).with_style(style);
            let at = format!("style={} theme={theme:?}", style.name());
            for line in format!("{:#?}", preset.tokens).lines() {
                let line = line.trim().trim_end_matches(',');
                if let Some((name, value)) = line.split_once(": ")
                    && value.starts_with('#')
                {
                    println!("token {at} name={name} value={value}");
                }
            }
            let core = preset.style();
            let badge = preset.badge(&core);
            println!(
                "badge {at} size=default height={} min_width=0 padding_x={} gap={} radius={} glyph={} text={}/{}",
                badge.height,
                badge.padding_x,
                badge.gap,
                badge.radius,
                badge.glyph,
                badge.text.size,
                badge.text.line_height
            );
            for variant in badge.variants.keys() {
                look(
                    &format!("look widget=badge {at} variant={variant}"),
                    badge.variants.get(variant),
                );
            }
            let input = core.sheet.resolve(
                InputPart::Root,
                &[Key::Size(core.input().sizes.default_key())],
                &core.tokens,
            );
            let layout = input.style_at(core.tokens.spacing);
            println!(
                "part {at} part=input height={} padding_x={} padding_y={} radius={}",
                layout.size.height.into_raw().value(),
                layout.padding.left.into_raw().value(),
                layout.padding.top.into_raw().value(),
                input.paint_classes().radius.nw
            );
            let menu = core.menu();
            let radius = |part, keys: &[Key]| {
                core.sheet
                    .resolve(part, keys, &core.tokens)
                    .paint_classes()
                    .radius
                    .nw
            };
            println!(
                "part {at} part=content padding={} radius={}",
                menu.padding,
                radius(MenuPart::Content, &[])
            );
            println!(
                "part {at} part=item padding_x={} padding_y={} radius={} gap={}",
                menu.item_padding.x,
                menu.item_padding.y,
                radius(MenuPart::Item, &[Key::Variant(Variant::DEFAULT)]),
                menu.item_gap
            );
        }
    }
}
