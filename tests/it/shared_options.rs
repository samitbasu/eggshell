//! The options every owned chain shares through `Configure`, `Interactive`
//! and `Respond` (#315), checked chain by chain over one table: the name,
//! merged classes, the tooltip and the refusal that replaces it, enablement
//! and its conditional forms, and the response hook. And the `_with`
//! terminators, whose handlers get the egui context.

use core::time::Duration;

use eggshell_ui::overlay::Shown;
use eggshell_ui::owned::accordion::Single;
use eggshell_ui::owned::options::{Configure, Interactive, Respond};
use eggshell_ui::owned::splitter::PaneSize;
use eggshell_ui::owned::table::Column;
use eggshell_ui::owned::{Enablement, Mark, toggle_group};
use eggshell_ui::prelude::*;
use eggshell_ui::testing::{Driver, Target};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Style, TextStyle, Track};
use egui::{Context, Id, Role, Shape, Theme, vec2};
use egui_kittest::kittest::Queryable as _;

use crate::support;

/// The size two `classes` calls give a control's node between them: taller
/// than any control's content, which a flex item does not shrink below.
const SIZED: (f32, f32) = (173.0, 61.0);

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
    Disclosure,
    Section,
    AccordionItem,
    Tabs,
    CommandList,
    Combobox,
    Autosuggest,
}

const ALL: [Control; 21] = [
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
    Control::Disclosure,
    Control::Section,
    Control::AccordionItem,
    Control::Tabs,
    Control::CommandList,
    Control::Combobox,
    Control::Autosuggest,
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
            Self::Section => r#""target"/Header"#,
            Self::AccordionItem => r#""item"/Header"#,
            Self::Tabs => r#""target"/List/Tab(1)"#,
            Self::CommandList => "/Search/Query",
            Self::Combobox | Self::Autosuggest => "Field)",
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
            Self::RadioGroup | Self::Segmented | Self::Tabs => "Zero",
            _ => self.label(),
        }
    }

    /// The role of the node named [`label`](Self::label) that takes input,
    /// where another node of the control shares the name: a section's
    /// region, a command list's listbox.
    fn input_role(self) -> Option<Role> {
        match self {
            Self::Section | Self::AccordionItem => Some(Role::Button),
            Self::CommandList => Some(Role::ComboBox),
            _ => None,
        }
    }

    /// The node assistive technology reads as the control named "Target"
    /// (`label`), or as the node disabled with it (`input_label`).
    fn node<'d>(self, d: &'d Driver<App>, label: &'d str) -> egui_kittest::Node<'d> {
        match self.input_role() {
            Some(role) if label == self.label() => d.harness().get_by_role_and_label(role, label),
            _ => d.harness().get_by_label(label),
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
        .classes(tw().w_px(SIZED.0))
        .classes(tw().h_px(SIZED.1))
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
        Control::Disclosure => {
            shared(b.disclosure("target", Shown::Closed), app).on_change(|_, _| {})
        }
        Control::Section => {
            shared(
                b.section("target", "Press", Shown::Open)
                    .on_change(|_, _| {}),
                app,
            )
            .children(|b| b.text(tw(), "Body"));
        }
        Control::AccordionItem => {
            b.accordion("target", Single::collapsible(None::<&str>))
                .on_change(|_, _| {})
                .items(|items| {
                    shared(items.item("item", "Press"), app).children(|b| b.text(tw(), "Body"));
                });
        }
        Control::Tabs => {
            shared(b.tabs("target", 1u8, CHOICES), app)
                .on_change(|_, _| {})
                .panel(|b, _| b.text(tw(), "Panel"));
        }
        Control::CommandList => {
            shared(b.command_list("target"), app).items(|items| {
                items.item("alpha", "Alpha").on_pick(|_| {});
            });
        }
        Control::Combobox => {
            let options = CHOICES.map(|(value, label)| (value, label.to_owned()));
            shared(b.combobox("target", Some(1u8), options), app).on_change(|_, _| {});
        }
        Control::Autosuggest => {
            shared(b.autosuggest("target", "Text", ["Texts".to_owned()]), app).on_change(|_, _| {});
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
    rig(app, view)
}

/// `view` of `app`, with quick tooltips, settled.
fn rig<A: 'static>(app: A, view: support::View<A>) -> Driver<A> {
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

fn painted<A>(d: &Driver<A>, text: &str) -> bool {
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
            !d.harness()
                .query_all_by_label(control.label())
                .collect::<Vec<_>>()
                .is_empty(),
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
            .filter(|(path, _)| path.contains(r#""target""#))
            .collect();
        assert!(!own.is_empty(), "precondition: {control:?} is built");
        assert!(
            own.iter()
                .any(|(_, rect)| (rect.width(), rect.height()) == SIZED),
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
            let disabled = control
                .node(&d, control.input_label())
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
        let description = control
            .node(&d, control.label())
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
        Tabs,
        ChipClick,
        ChipRemove,
        TableSelect,
        Section,
        Accordion,
    }

    const TERMINATORS: [Terminator; 21] = [
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
        Terminator::Tabs,
        Terminator::ChipClick,
        Terminator::ChipRemove,
        Terminator::TableSelect,
        Terminator::Section,
        Terminator::Accordion,
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
                Terminator::Tabs => {
                    b.tabs("target", 0u8, CHOICES)
                        .on_change_with(|app, ctx, _| ran(app, ctx))
                        .panel(|b, _| b.text(tw(), "Panel"));
                }
                Terminator::ChipClick => b.chip("target", "Rust").on_click_with(ran).add(),
                Terminator::ChipRemove => b.chip("target", "Rust").on_remove_with(ran).add(),
                Terminator::TableSelect => {
                    b.table("target")
                        .columns([Column::new(Track::auto())])
                        .selected(None)
                        .on_select_with(|app, ctx, _: u8| ran(app, ctx))
                        .rows(&[1u8], |n| *n, |row, n| row.cell_text(n.to_string()))
                        .add();
                }
                Terminator::Section => {
                    b.section("target", "Press", Shown::Closed)
                        .on_change_with(|app, ctx, _| ran(app, ctx))
                        .children(|_| {});
                }
                Terminator::Accordion => {
                    b.accordion("target", Single::collapsible(None::<&str>))
                        .on_change_with(|app, ctx, _| ran(app, ctx))
                        .items(|items| items.item("item", "Press").children(|_| {}));
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
            Terminator::Tabs => {
                d.click(Target::suffix(r#""target"/List/Tab(2)"#));
            }
            Terminator::ChipRemove => {
                d.click(Target::suffix(r#""target"/"remove""#));
            }
            Terminator::TableSelect => {
                d.click(Target::suffix("Row(1)"));
            }
            Terminator::Section => {
                d.click(Target::suffix(r#""target"/Header"#));
            }
            Terminator::Accordion => {
                d.click(Target::suffix(r#""item"/Header"#));
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
            Terminator::Button
            | Terminator::Toggle
            | Terminator::Node
            | Terminator::Disclosure
            | Terminator::ChipClick => {
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

/// What the converted chains with several parts do with their options
/// beyond the table's checks.
mod parts {
    use super::*;
    use eggshell_ui::owned::choice::Choice;

    struct Picks {
        enablement: Enablement,
        picks: u32,
    }

    fn palette(b: &mut Builder<'_, '_, Picks>, app: &Picks) {
        b.command_list("palette")
            .named("Commands")
            .enablement(app.enablement)
            .items(|items| {
                items
                    .item("alpha", "Alpha")
                    .on_pick(|app: &mut Picks| app.picks += 1);
            });
    }

    #[test]
    fn a_disabled_command_list_takes_no_pick() {
        for (enablement, picks) in [(Enablement::Enabled, 1), (Enablement::Disabled, 0)] {
            let mut d = rig(
                Picks {
                    enablement,
                    picks: 0,
                },
                palette,
            );
            assert!(
                painted(&d, "Alpha"),
                "precondition: {enablement:?} shows its item"
            );
            d.click(Target::Text("Alpha".to_owned()));
            d.frames(4);
            assert_eq!(d.app().picks, picks, "{enablement:?}");
        }
    }

    fn fruit(b: &mut Builder<'_, '_, Picks>, app: &Picks) {
        let options = CHOICES.map(|(value, label)| (value, label.to_owned()));
        b.combobox("fruit", None, options)
            .named("Fruit")
            .tooltip("Tip")
            .enablement(app.enablement)
            .on_change(|_, _: u8| {});
    }

    #[test]
    fn a_disabled_combobox_does_not_open() {
        for (enablement, opens) in [(Enablement::Enabled, true), (Enablement::Disabled, false)] {
            let mut d = rig(
                Picks {
                    enablement,
                    picks: 0,
                },
                fruit,
            );
            assert!(
                !painted(&d, "Zero"),
                "precondition: {enablement:?} starts closed"
            );
            d.click(Target::role(Role::Button, "Show options"));
            d.frames(6);
            assert_eq!(painted(&d, "Zero"), opens, "{enablement:?}");
        }
    }

    #[test]
    fn a_combobox_s_tooltip_shows_over_its_whole_box() {
        let mut d = rig(
            Picks {
                enablement: Enablement::Enabled,
                picks: 0,
            },
            fruit,
        );
        d.hover(Target::role(Role::Button, "Show options"));
        support::rest_for_tooltip(&mut d);
        assert!(painted(&d, "Tip"));
    }

    fn tabs(b: &mut Builder<'_, '_, ()>, _: &()) {
        b.tabs(
            "tabs",
            0u8,
            [
                Choice::new(0u8, "Zero"),
                Choice::new(1u8, "One").tooltip("Own"),
            ],
        )
        .tooltip("Tip")
        .on_change(|_, _| {})
        .panel(|b, _| b.text(tw(), "Panel"));
    }

    #[test]
    fn a_tab_s_own_tooltip_shows_instead_of_the_tabs() {
        for (tab, shown, hidden) in [("Zero", "Tip", "Own"), ("One", "Own", "Tip")] {
            let mut d = rig((), tabs);
            d.hover(Target::role(Role::Tab, tab));
            support::rest_for_tooltip(&mut d);
            assert!(painted(&d, shown), "{tab}: {shown} shows");
            assert!(!painted(&d, hidden), "{tab}: {hidden} does not");
        }
    }
}

/// The chains that take no input of their own, which have only
/// `Configure`'s options: a table, an accordion's column and a dialog.
mod configure_only {
    use super::*;
    use eggshell_ui::overlay::Modal;
    use eggshell_ui::owned::dialog::Dialog;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Chain {
        Table,
        Accordion,
        Dialog,
    }

    /// Taller than a dialog's header and body.
    const BOX: (f32, f32) = (173.0, 141.0);

    const CHAINS: [Chain; 3] = [Chain::Table, Chain::Accordion, Chain::Dialog];

    impl Chain {
        /// The role assistive technology reads the named chain as.
        fn role(self) -> Role {
            match self {
                Self::Table => Role::Table,
                Self::Accordion => Role::Group,
                Self::Dialog => Role::Dialog,
            }
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Naming {
        Named,
        Unnamed,
    }

    struct Page {
        chain: Chain,
        naming: Naming,
    }

    fn configured<C: Configure>(chain: C, naming: Naming) -> C {
        chain
            .when(naming == Naming::Named, |chain| chain.named("Target"))
            .classes(tw().w_px(BOX.0))
            .classes(tw().h_px(BOX.1))
            .when_some(Some("Tip"), |chain, tip| chain.tooltip(tip))
            .when_some(None::<&str>, |chain, tip| chain.tooltip(tip))
    }

    fn view(b: &mut Builder<'_, '_, Page>, app: &Page) {
        b.div(tw().flex().flex_col().items_start().p(8.0), |b| {
            match app.chain {
                Chain::Table => {
                    configured(b.table("target"), app.naming)
                        .columns([Column::new(Track::auto())])
                        .rows(&[1u8], |n| *n, |row, n| row.cell_text(n.to_string()))
                        .add();
                }
                Chain::Accordion => {
                    configured(
                        b.accordion("target", Single::collapsible(None::<&str>)),
                        app.naming,
                    )
                    .on_change(|_, _| {})
                    .items(|items| items.item("item", "Press").children(|_| {}));
                }
                Chain::Dialog => {
                    b.overlay("target")
                        .modal(Modal::Center)
                        .shown(Shown::Open)
                        .on_dismiss(|_| {})
                        .dialog(configured(Dialog::new("Title"), app.naming), |d| {
                            d.body(|s| s.text(tw(), "Body"));
                        });
                }
            }
        });
    }

    fn driver(chain: Chain, naming: Naming) -> Driver<Page> {
        rig(Page { chain, naming }, view)
    }

    /// The rect of the one node both `classes` calls sized.
    fn sized(d: &Driver<Page>) -> Option<egui::Rect> {
        support::nodes(d)
            .into_iter()
            .map(|(_, rect)| rect)
            .find(|rect| (rect.width(), rect.height()) == BOX)
    }

    #[test]
    fn named_names_the_chain_in_its_role() {
        for chain in CHAINS {
            let d = driver(chain, Naming::Named);
            assert!(
                d.harness()
                    .query_by_role_and_label(chain.role(), "Target")
                    .is_some(),
                "{chain:?}"
            );
        }
    }

    #[test]
    fn an_unnamed_accordion_is_no_group_and_an_unnamed_dialog_is_named_by_its_title() {
        let d = driver(Chain::Accordion, Naming::Unnamed);
        assert!(d.harness().query_by_role(Role::Group).is_none());
        let d = driver(Chain::Dialog, Naming::Unnamed);
        assert!(
            d.harness()
                .query_by_role_and_label(Role::Dialog, "Title")
                .is_some()
        );
    }

    #[test]
    fn classes_merge_across_calls_on_the_chain_s_box() {
        for chain in CHAINS {
            let d = driver(chain, Naming::Named);
            assert!(sized(&d).is_some(), "{chain:?}: {:?}", support::nodes(&d));
        }
    }

    #[test]
    fn the_tooltip_shows_over_the_chain_s_box() {
        for chain in CHAINS {
            let mut d = driver(chain, Naming::Named);
            assert!(
                !painted(&d, "Tip"),
                "precondition: {chain:?} shows no tip at rest"
            );
            let at = sized(&d).expect("precondition: the box is sized").center();
            d.hover(Target::Point(at));
            support::rest_for_tooltip(&mut d);
            assert!(painted(&d, "Tip"), "{chain:?}");
        }
    }
}
