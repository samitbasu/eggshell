//! The options every owned chain shares through `Configure`, `Interactive`
//! and `Respond` (#315), checked chain by chain over one table: the name,
//! merged classes, the tooltip and the refusal that replaces it, enablement
//! and its conditional forms, and the response hook. And the `_with`
//! terminators, whose handlers get the egui context.

use core::time::Duration;

use eggshell_ui::owned::options::{Configure, Interactive, Respond};
use eggshell_ui::owned::splitter::PaneSize;
use eggshell_ui::owned::{Enablement, Mark, toggle_group};
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Style, TextStyle};
use egui::{Context, Id, Shape, Theme, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;

const CHOICES: [(u8, &str); 3] = [(0, "Zero"), (1, "One"), (2, "Two")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Button,
    Toggle,
    Checkbox,
    Switch,
    RadioGroup,
    Segmented,
    Select,
    Slider,
    Stepper,
    TextField,
    Splitter,
    Custom,
    Number,
    Dropdown,
}

const ALL: [Control; 14] = [
    Control::Button,
    Control::Toggle,
    Control::Checkbox,
    Control::Switch,
    Control::RadioGroup,
    Control::Segmented,
    Control::Select,
    Control::Slider,
    Control::Stepper,
    Control::TextField,
    Control::Splitter,
    Control::Custom,
    Control::Number,
    Control::Dropdown,
];

impl Control {
    /// The end of the path of the node that takes focus and shows the
    /// tooltip.
    fn focus_node(self) -> &'static str {
        match self {
            Self::Checkbox => r#""target"/Box"#,
            Self::Switch => r#""target"/Track"#,
            Self::RadioGroup => r#""target"/Circle(1)"#,
            Self::Segmented => r#""target"/1"#,
            Self::Slider => r#""target"/Track/Thumb"#,
            Self::Stepper => r#""target"/Box/Field"#,
            Self::Number => r#""target"/Readout"#,
            _ => r#""target""#,
        }
    }

    /// What assistive technology reads for the control named "Target".
    fn label(self) -> &'static str {
        match self {
            Self::Number => "Edit Target",
            _ => "Target",
        }
    }

    /// What assistive technology reads for a node that is disabled while
    /// the control is: a group's option, or the control itself.
    fn input_label(self) -> &'static str {
        match self {
            Self::RadioGroup | Self::Segmented => "Zero",
            _ => self.label(),
        }
    }
}

/// How a test configures the control's enablement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gate {
    Open,
    Disabled,
    Refused,
    RefusedThenCleared,
    WhenHolds,
    WhenFails,
    WhenSome,
    WhenNone,
}

impl Gate {
    /// Whether the control ends up taking input.
    fn enabled(self) -> bool {
        matches!(
            self,
            Self::Open | Self::RefusedThenCleared | Self::WhenFails | Self::WhenNone
        )
    }
}

const GATES: [Gate; 8] = [
    Gate::Open,
    Gate::Disabled,
    Gate::Refused,
    Gate::RefusedThenCleared,
    Gate::WhenHolds,
    Gate::WhenFails,
    Gate::WhenSome,
    Gate::WhenNone,
];

struct App {
    control: Control,
    gate: Gate,
    responses: u32,
}

impl App {
    fn new(control: Control, gate: Gate) -> Self {
        Self {
            control,
            gate,
            responses: 0,
        }
    }
}

/// `chain` with every shared option the test sets, written once over the
/// traits for every chain.
fn shared<C: Interactive + Respond<App = App>>(chain: C, app: &App) -> C {
    let chain = chain
        .named("Target")
        .classes(tw().w_px(173.0))
        .classes(tw().h_px(41.0))
        .tooltip("Tip")
        .on_response(|app: &mut App, _| app.responses += 1);
    match app.gate {
        Gate::Open => chain,
        Gate::Disabled => chain.enablement(Enablement::Disabled),
        Gate::Refused => chain.refused("Busy"),
        Gate::RefusedThenCleared => chain.refused("Busy").refused(None::<&str>),
        Gate::WhenHolds => chain.when(true, |chain| chain.enablement(Enablement::Disabled)),
        Gate::WhenFails => chain.when(false, |chain| chain.enablement(Enablement::Disabled)),
        Gate::WhenSome => chain.when_some(Some("Busy"), |chain, why| chain.refused(why)),
        Gate::WhenNone => chain.when_some(None::<&str>, |chain, why| chain.refused(why)),
    }
}

fn control(b: &mut Builder<'_, '_, App>, app: &App) {
    match app.control {
        Control::Button => shared(b.button("target", "Press"), app).add(),
        Control::Toggle => shared(b.toggle("target", "Bold", false), app).on_change(|_, _| {}),
        Control::Checkbox => {
            shared(b.checkbox_unlabelled("target", false), app).on_change(|_, _| {});
        }
        Control::Switch => {
            shared(b.switch_unlabelled("target", false), app).on_change(|_, _| {});
        }
        Control::RadioGroup => {
            shared(b.radio_group("target", 1u8, CHOICES), app).on_change(|_, _| {});
        }
        Control::Segmented => {
            shared(b.segmented("target", 1u8, CHOICES), app).on_change(|_, _| {});
        }
        Control::Select => shared(b.select("target", 1u8, CHOICES), app).on_change(|_, _| {}),
        Control::Slider => {
            shared(b.slider("target", 4.0, 0.0..=10.0), app).on_change(|_, _| {});
        }
        Control::Stepper => {
            shared(b.stepper("target", 4.0, 0.0..=10.0), app).on_change(|_, _| {});
        }
        Control::TextField => {
            shared(b.text_field("target", "Text"), app).on_change(|_, _| {});
        }
        Control::Splitter => {
            b.div(tw().flex().h_px(60.0).w_px(400.0), |b| {
                b.div(tw().w_px(80.0), |_| {});
                shared(b.splitter("target").before(PaneSize::new(80.0)), app).on_resize(|_, _| {});
                b.div(tw().flex_1(), |_| {});
            });
        }
        Control::Custom => {
            shared(b.custom("target", tw().sense_click()), app).ui(|_, _| {});
        }
        Control::Number => {
            shared(b.number("target", 4.0, 0.0..=10.0), app)
                .editable(|text| text.parse().ok())
                .on_change(|_, _| {});
        }
        Control::Dropdown => {
            shared(b.dropdown_menu("target", "More"), app).items(|m| {
                m.item("copy", "Copy").on_click(|_| {});
            });
        }
    }
}

fn view(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(8.0).p(8.0), |b| {
        control(b, app);
    });
}

fn quick_tooltips(style: &mut Style) {
    let mut tooltip = style.tooltip.clone();
    tooltip.delay = Duration::from_millis(100);
    style.tooltip = tooltip;
}

fn driver(app: App) -> Driver<App> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut d = support::driver(
        vec2(500.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    )
    .style(&eggshell_ui::SharedStyle::new(support::shadcn_with(
        quick_tooltips,
    )))
    .build();
    d.frames(6);
    d
}

fn painted(d: &Driver<App>, text: &str) -> bool {
    support::shapes(d).iter().any(|shape| match shape {
        Shape::Text(shape) => shape.galley.text() == text,
        _ => false,
    })
}

#[test]
fn named_is_the_accessible_name_of_every_chain() {
    for control in ALL {
        let d = driver(App::new(control, Gate::Open));
        assert!(
            d.harness().query_by_label(control.label()).is_some(),
            "{control:?}"
        );
    }
}

#[test]
fn classes_merge_across_calls_on_every_chain() {
    for control in ALL {
        let d = driver(App::new(control, Gate::Open));
        let nodes = support::nodes(&d);
        let own: Vec<_> = nodes
            .iter()
            .filter(|(path, _)| path.starts_with(r#"/"target""#))
            .collect();
        assert!(!own.is_empty(), "precondition: {control:?} is built");
        assert!(
            own.iter()
                .any(|(_, rect)| (rect.width(), rect.height()) == (173.0, 41.0)),
            "{control:?}: both calls' classes hold on one node: {own:?}"
        );
    }
}

#[test]
fn enablement_refused_when_and_when_some_gate_every_chain() {
    use egui_kittest::kittest::NodeT as _;
    for control in ALL {
        for gate in GATES {
            let d = driver(App::new(control, gate));
            let disabled = d
                .harness()
                .get_by_label(control.input_label())
                .accesskit_node()
                .is_disabled();
            assert_eq!(disabled, !gate.enabled(), "{control:?} {gate:?}");
        }
    }
}

#[test]
fn a_refusal_replaces_the_tooltip_on_every_chain() {
    for control in ALL {
        for (gate, shown, hidden) in [(Gate::Open, "Tip", "Busy"), (Gate::Refused, "Busy", "Tip")] {
            let mut d = driver(App::new(control, gate));
            d.hover(Target::suffix(control.focus_node()));
            support::rest_for_tooltip(&mut d);
            assert!(painted(&d, shown), "{control:?} {gate:?}: {shown} shows");
            assert!(
                !painted(&d, hidden),
                "{control:?} {gate:?}: {hidden} does not"
            );
        }
    }
}

#[test]
fn the_refusal_is_the_accessible_description_of_every_chain() {
    use egui_kittest::kittest::NodeT as _;
    for control in ALL {
        let d = driver(App::new(control, Gate::Refused));
        let description = d
            .harness()
            .get_by_label(control.label())
            .accesskit_node()
            .description();
        assert_eq!(description.as_deref(), Some("Busy"), "{control:?}");
    }
}

#[test]
fn on_response_runs_each_frame_on_every_chain() {
    for control in ALL {
        let d = driver(App::new(control, Gate::Open));
        assert!(d.app().responses > 0, "{control:?}");
    }
}

/// The `_with` terminators: each handler is given the egui context, and
/// what it writes there is the application's context.
mod with {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Terminator {
        Button,
        Toggle,
        ToggleItem,
        Checkbox,
        Switch,
        RadioGroup,
        Segmented,
        TextField,
        Slider,
        Stepper,
        Splitter,
        MenuItem,
        MenuCheckbox,
        Node,
        Disclosure,
    }

    const TERMINATORS: [Terminator; 15] = [
        Terminator::Button,
        Terminator::Toggle,
        Terminator::ToggleItem,
        Terminator::Checkbox,
        Terminator::Switch,
        Terminator::RadioGroup,
        Terminator::Segmented,
        Terminator::TextField,
        Terminator::Slider,
        Terminator::Stepper,
        Terminator::Splitter,
        Terminator::MenuItem,
        Terminator::MenuCheckbox,
        Terminator::Node,
        Terminator::Disclosure,
    ];

    fn mark() -> Id {
        Id::unique("with-terminator")
    }

    /// What every handler does: counts, and leaves a mark in the context.
    fn ran(app: &mut Runs, ctx: &Context) {
        app.runs += 1;
        ctx.data_mut(|data| data.insert_temp(mark(), app.runs));
    }

    struct Runs {
        terminator: Terminator,
        runs: u32,
    }

    fn view(b: &mut Builder<'_, '_, Runs>, app: &Runs) {
        b.div(
            tw().flex().flex_col().items_start().gap(8.0).p(8.0),
            |b| match app.terminator {
                Terminator::Button => b.button("target", "Go").on_click_with(ran),
                Terminator::Toggle => {
                    b.toggle("target", "Bold", false)
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::ToggleItem => {
                    let group = toggle_group::Group::new("group", 0u8)
                        .on_change_with(|app: &mut Runs, ctx, _| ran(app, ctx));
                    b.div_id("target", tw().flex(), |b| {
                        b.toggle_item(&group, 0, "Zero").add();
                        b.toggle_item(&group, 1, "One").add();
                    });
                }
                Terminator::Checkbox => {
                    b.checkbox("target", "Agree", false)
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::Switch => {
                    b.switch("target", "Alerts", false)
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::RadioGroup => {
                    b.radio_group("target", 0u8, CHOICES)
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::Segmented => {
                    b.segmented("target", 0u8, CHOICES)
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::TextField => {
                    b.text_field("target", "")
                        .named("Field")
                        .classes(tw().w_px(160.0))
                        .focus_now()
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::Slider => {
                    b.slider("target", 4.0, 0.0..=10.0)
                        .named("Level")
                        .classes(tw().w_px(160.0))
                        .focus_now()
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::Stepper => {
                    b.stepper("target", 4.0, 0.0..=10.0)
                        .named("Count")
                        .focus_now()
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
                Terminator::Splitter => {
                    b.div(tw().flex().h_px(40.0).w_px(300.0), |b| {
                        b.div(tw().w_px(80.0), |_| {});
                        b.splitter("target")
                            .before(PaneSize::new(80.0))
                            .named("Sidebar")
                            .focus_now()
                            .on_resize_with(|app, ctx, _| ran(app, ctx));
                        b.div(tw().flex_1(), |_| {});
                    });
                }
                Terminator::MenuItem => {
                    b.dropdown_menu("menu", "More").items(|m| {
                        m.item("target", "Copy").on_click_with(ran);
                    });
                }
                Terminator::MenuCheckbox => {
                    b.dropdown_menu("menu", "More").items(|m| {
                        m.checkbox_item("target", "Wrap", Mark::Off)
                            .on_change_with(|app, ctx, _| ran(app, ctx));
                    });
                }
                Terminator::Node => {
                    b.div_id("target", tw().size_px(40.0).sense_click(), |_| {})
                        .on_click_with(ran);
                }
                Terminator::Disclosure => {
                    b.disclosure("target", eggshell_ui::overlay::Shown::Closed)
                        .named("More")
                        .on_change_with(|app, ctx, _| ran(app, ctx));
                }
            },
        );
    }

    /// Drives `terminator`'s control the way a user changes it.
    fn drive(d: &mut Driver<Runs>, terminator: Terminator) {
        match terminator {
            Terminator::ToggleItem => {
                d.click(Target::suffix(r#""group"=1"#));
            }
            Terminator::Checkbox => {
                d.click(Target::suffix(r#""target"/Box"#));
            }
            Terminator::Switch => {
                d.click(Target::suffix(r#""target"/Track"#));
            }
            Terminator::RadioGroup => {
                d.click(Target::suffix(r#""target"/Circle(2)"#));
            }
            Terminator::Segmented => {
                d.click(Target::suffix(r#""target"/2"#));
            }
            Terminator::TextField => {
                d.type_text("x");
            }
            Terminator::Slider | Terminator::Splitter => {
                d.press(egui::Key::ArrowRight);
            }
            Terminator::Stepper => {
                d.press(egui::Key::ArrowUp);
            }
            Terminator::MenuItem | Terminator::MenuCheckbox => {
                d.click(Target::suffix(r#""menu""#));
                d.frames(4);
                d.click(Target::suffix(r#""target""#));
            }
            Terminator::Button | Terminator::Toggle | Terminator::Node | Terminator::Disclosure => {
                d.click(Target::suffix(r#""target""#));
            }
        }
        d.frames(4);
    }

    #[test]
    fn every_with_terminator_hands_its_handler_the_context() {
        for terminator in TERMINATORS {
            let tokens = Tokens::for_theme(Theme::Light);
            let mut d = support::driver(
                vec2(500.0, 400.0),
                TextStyle::new(tokens.text_role(), tokens.foreground),
                Runs {
                    terminator,
                    runs: 0,
                },
                view,
            )
            .build();
            d.frames(6);
            assert_eq!(d.app().runs, 0, "precondition: {terminator:?} has not run");
            drive(&mut d, terminator);
            assert_eq!(d.app().runs, 1, "{terminator:?} ran once");
            assert_eq!(
                d.ctx().data(|data| data.get_temp::<u32>(mark())),
                Some(1),
                "{terminator:?} wrote to the application's context"
            );
        }
    }

    #[test]
    fn a_node_s_on_response_runs_every_frame() {
        struct Count(u32);
        fn view(b: &mut Builder<'_, '_, Count>, _: &Count) {
            b.div_id("node", tw().size_px(20.0), |_| {})
                .on_response(|count: &mut Count, _| count.0 += 1);
        }
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(100.0, 100.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            Count(0),
            view,
        )
        .build();
        d.frames(3);
        let first = d.app().0;
        d.frames(3);
        assert!(first > 0 && d.app().0 > first, "{first} then {}", d.app().0);
    }
}

/// The options of the chains outside the table: a menu's items, and a field
/// box, which passes its enablement to what it holds.
mod others {
    use super::*;

    struct Menu {
        responses: u32,
    }

    fn menu(b: &mut Builder<'_, '_, Menu>, _: &Menu) {
        b.dropdown_menu("menu", "More").items(|m| {
            m.item("copy", "Copy")
                .named("Copy the text")
                .refused("Nothing selected")
                .on_response(|app: &mut Menu, _| app.responses += 1)
                .on_click(|_| {});
        });
    }

    #[test]
    fn a_menu_item_takes_the_shared_options() {
        use egui_kittest::kittest::NodeT as _;
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(400.0, 300.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            Menu { responses: 0 },
            menu,
        )
        .build();
        d.frames(4);
        d.click(Target::suffix(r#""menu""#));
        d.frames(6);
        let item = d.harness().get_by_label("Copy the text");
        assert!(item.accesskit_node().is_disabled());
        assert_eq!(
            item.accesskit_node().description().as_deref(),
            Some("Nothing selected")
        );
        assert!(d.app().responses > 0);
    }

    fn boxed(b: &mut Builder<'_, '_, ()>, _: &()) {
        b.field_box("box")
            .refused("Read only")
            .when(true, |chain| chain.classes(tw().w_px(200.0)))
            .children(|s| {
                s.text_field("field", "Text")
                    .named("Inside")
                    .classes(tw().flex_1())
                    .on_change(|_, _| {});
            });
    }

    #[test]
    fn a_refused_field_box_disables_what_it_holds() {
        use egui_kittest::kittest::NodeT as _;
        let tokens = Tokens::for_theme(Theme::Light);
        let mut d = support::driver(
            vec2(400.0, 200.0),
            TextStyle::new(tokens.text_role(), tokens.foreground),
            (),
            boxed,
        )
        .build();
        d.frames(4);
        assert!(
            d.harness()
                .get_by_label("Inside")
                .accesskit_node()
                .is_disabled()
        );
        assert_eq!(support::rect_of(&d, r#"/"box""#).width(), 200.0);
    }
}
