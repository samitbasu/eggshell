//! `focus_on_open` and `focus_now` on every focusable owned chain: the
//! control an overlay opens on, and the one the application focuses on the
//! frame it asks, whether the control is new then or already built.

use eggshell_ui::Problem;
use eggshell_ui::overlay::Anchor;
use eggshell_ui::owned::{PaneSize, ToggleGroup};
use eggshell_ui::prim::text_field::Select;
use eggshell_ui::testing::{assert_no_reports, assert_reported};
use eggshell_ui::theme::Tokens;
use eggshell_ui::{Builder, TextStyle, tw};
use egui::{Event, Theme, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use crate::support::{self, Rig, focused_node};

const CHOICES: [(u8, &str); 3] = [(0, "Zero"), (1, "One"), (2, "Two")];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Button,
    Toggle,
    ToggleItem,
    Checkbox,
    Switch,
    RadioGroup,
    Segmented,
    Select,
    Slider,
    Stepper,
    TextField,
    Splitter,
    Dropdown,
}

const ALL: [Control; 13] = [
    Control::Button,
    Control::Toggle,
    Control::ToggleItem,
    Control::Checkbox,
    Control::Switch,
    Control::RadioGroup,
    Control::Segmented,
    Control::Select,
    Control::Slider,
    Control::Stepper,
    Control::TextField,
    Control::Splitter,
    Control::Dropdown,
];

impl Control {
    /// The end of the path of the node that takes focus.
    fn focus_node(self) -> &'static str {
        match self {
            Self::ToggleItem => r#""group"=1"#,
            Self::Checkbox => r#""target"/Box"#,
            Self::Switch => r#""target"/Track"#,
            Self::RadioGroup => r#""target"/Circle(1)"#,
            Self::Segmented => r#""target"/1"#,
            Self::Slider => r#""target"/Track/Thumb"#,
            Self::Stepper => "/Field",
            _ => r#""target""#,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Nothing,
    OnOpen,
    Now(Select),
}

struct App {
    control: Control,
    ask: Ask,
    /// Whether the control is built.
    shown: bool,
    text: String,
    count: f64,
}

impl App {
    fn new(control: Control, ask: Ask) -> Self {
        Self {
            control,
            ask,
            shown: true,
            text: "Item 1".to_owned(),
            count: 4.0,
        }
    }
}

fn group(_: &App) -> ToggleGroup<App, u8> {
    ToggleGroup::new("group", 1u8).on_change(|_: &mut App, _| {})
}

/// The control `app` names, under the key `"target"`, asking for focus as
/// `app.ask` says.
fn control(b: &mut Builder<'_, '_, App>, app: &App) {
    macro_rules! ask {
        ($chain:expr) => {{
            let chain = $chain;
            match app.ask {
                Ask::Nothing => chain,
                Ask::OnOpen => chain.focus_on_open(),
                Ask::Now(_) => chain.focus_now(),
            }
        }};
        ($chain:expr, selecting) => {{
            let chain = $chain;
            match app.ask {
                Ask::Nothing => chain,
                Ask::OnOpen => chain.focus_on_open(),
                Ask::Now(select) => chain.focus_now(select),
            }
        }};
    }
    match app.control {
        Control::Button => {
            ask!(b.button("target", "Target")).add();
        }
        Control::Toggle => {
            ask!(b.toggle("target", "Bold", false)).on_change(|_, _| {});
        }
        Control::ToggleItem => {
            let tools = group(app);
            b.div_id("target", tw().flex(), |b| {
                b.toggle_item(&tools, 0, "Zero").add();
                ask!(b.toggle_item(&tools, 1, "One")).add();
            });
        }
        Control::Checkbox => {
            ask!(b.checkbox("target", "Agree", false)).on_change(|_, _| {});
        }
        Control::Switch => {
            ask!(b.switch("target", "Alerts", false)).on_change(|_, _| {});
        }
        Control::RadioGroup => {
            ask!(b.radio_group("target", 1u8, CHOICES)).on_change(|_, _| {});
        }
        Control::Segmented => {
            ask!(b.segmented("target", 1u8, CHOICES)).on_change(|_, _| {});
        }
        Control::Select => {
            ask!(b.select("target", 1u8, CHOICES).named("Pick")).on_change(|_, _| {});
        }
        Control::Slider => {
            ask!(b.slider("target", 4.0, 0.0..=10.0).named("Level")).on_change(|_, _| {});
        }
        Control::Stepper => {
            ask!(
                b.stepper("target", app.count, 0.0..=10.0).named("Count"),
                selecting
            )
            .on_change(|app: &mut App, count| app.count = count);
        }
        Control::TextField => {
            ask!(
                b.text_field("target", app.text.clone())
                    .named("Name")
                    .classes(tw().w_px(160.0)),
                selecting
            )
            .on_change(|app: &mut App, text| app.text = text);
        }
        Control::Splitter => {
            b.div(tw().flex().h_px(40.0).w_px(200.0), |b| {
                b.div(tw().w_px(80.0), |_| {});
                ask!(
                    b.splitter("target")
                        .before(PaneSize::new(80.0))
                        .named("Sidebar")
                )
                .on_resize(|_, _| {});
                b.div(tw().flex_1(), |_| {});
            });
        }
        Control::Dropdown => {
            ask!(b.dropdown_menu("target", "More")).items(|m| {
                m.item("copy", "Copy").on_click(|_: &mut App| {});
            });
        }
    }
}

/// The control in a popover that is open from the start, after a button
/// that would take the popover's focus by default.
fn in_popover(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div_id("anchor", tw().size_px(10.0), |_| {});
    let anchor = b.id_of("anchor");
    b.overlay("pop").anchor(Anchor::Node(anchor)).children(|s| {
        s.div(tw().flex().flex_col().gap(4.0).p(4.0), |s| {
            s.button("first", "First").add();
            control(s, app);
        });
    });
}

/// A text field to hold focus, then the control, if it is shown.
fn on_page(b: &mut Builder<'_, '_, App>, app: &App) {
    b.div(tw().flex().flex_col().items_start().gap(8.0).p(8.0), |b| {
        b.text_field("other", "")
            .named("Other")
            .classes(tw().w_px(160.0))
            .on_change(|_: &mut App, _| {});
        if app.shown {
            control(b, app);
        }
    });
}

fn harness(view: support::View<App>, app: App) -> Harness<'static, Rig<App>> {
    let tokens = Tokens::for_theme(Theme::Light);
    let mut harness = support::harness(
        vec2(500.0, 400.0),
        TextStyle::new(tokens.text_role(), tokens.foreground),
        app,
        view,
    );
    harness.run_steps(6);
    harness
}

fn focused_on(harness: &Harness<'_, Rig<App>>, control: Control) -> bool {
    focused_node(harness)
        .is_some_and(|path| path.contains(r#""target""#) && path.ends_with(control.focus_node()))
}

fn focus_other(harness: &mut Harness<'_, Rig<App>>) {
    harness.get_by_label("Other").focus();
    harness.run_steps(4);
    assert!(
        focused_node(harness).is_some_and(|path| path.ends_with(r#""other""#)),
        "precondition: the other field has focus, not {:?}",
        focused_node(harness)
    );
}

#[test]
fn without_the_option_a_popover_opens_on_its_first_focusable() {
    for control in ALL {
        let harness = harness(in_popover, App::new(control, Ask::Nothing));
        assert!(
            focused_node(&harness).is_some_and(|path| path.ends_with(r#""first""#)),
            "{control:?}: {:?}",
            focused_node(&harness)
        );
    }
}

#[test]
fn a_popover_opens_on_the_control_marked_focus_on_open() {
    for control in ALL {
        let harness = harness(in_popover, App::new(control, Ask::OnOpen));
        assert!(
            focused_on(&harness, control),
            "{control:?}: {:?}",
            focused_node(&harness)
        );
        assert_no_reports(&harness.ctx);
    }
}

#[test]
fn of_two_controls_marked_focus_on_open_the_first_takes_focus_and_it_is_reported() {
    fn two(b: &mut Builder<'_, '_, App>, _: &App) {
        b.div_id("anchor", tw().size_px(10.0), |_| {});
        let anchor = b.id_of("anchor");
        b.overlay("pop").anchor(Anchor::Node(anchor)).children(|s| {
            s.button("first", "First").add();
            s.button("target", "Target").focus_on_open().add();
            s.checkbox("second", "Second", false)
                .focus_on_open()
                .on_change(|_, _| {});
        });
    }
    let harness = harness(two, App::new(Control::Button, Ask::Nothing));
    assert_reported(&harness.ctx, Problem::FocusOnOpenTwice);
    assert!(
        focused_on(&harness, Control::Button),
        "{:?}",
        focused_node(&harness)
    );
}

#[test]
fn focus_on_open_on_the_page_takes_no_focus() {
    for control in ALL {
        let mut harness = harness(on_page, App::new(control, Ask::OnOpen));
        focus_other(&mut harness);
        harness.state_mut().app.shown = false;
        harness.run_steps(4);
        harness.state_mut().app.shown = true;
        harness.run_steps(6);
        assert!(!focused_on(&harness, control), "{control:?}");
    }
}

#[test]
fn focus_now_focuses_a_control_already_built() {
    for control in ALL {
        let mut harness = harness(on_page, App::new(control, Ask::Nothing));
        focus_other(&mut harness);
        harness.state_mut().app.ask = Ask::Now(Select::None);
        harness.run_steps(4);
        assert!(
            focused_on(&harness, control),
            "{control:?}: {:?}",
            focused_node(&harness)
        );
    }
}

#[test]
fn focus_now_focuses_a_control_built_that_frame() {
    for control in ALL {
        let mut app = App::new(control, Ask::Now(Select::None));
        app.shown = false;
        let mut harness = harness(on_page, app);
        focus_other(&mut harness);
        harness.state_mut().app.shown = true;
        harness.run_steps(4);
        assert!(
            focused_on(&harness, control),
            "{control:?}: {:?}",
            focused_node(&harness)
        );
    }
}

#[test]
fn once_the_app_drops_focus_now_focus_is_left_where_the_user_puts_it() {
    for control in ALL {
        let mut harness = harness(on_page, App::new(control, Ask::Now(Select::None)));
        assert!(focused_on(&harness, control), "precondition: {control:?}");
        harness.state_mut().app.ask = Ask::Nothing;
        harness.run_steps(2);
        focus_other(&mut harness);
        harness.run_steps(6);
        assert!(!focused_on(&harness, control), "{control:?}");
    }
}

fn typed_over(control: Control, select: Select, text: &str) -> Harness<'static, Rig<App>> {
    let mut harness = harness(on_page, App::new(control, Ask::Nothing));
    focus_other(&mut harness);
    harness.state_mut().app.ask = Ask::Now(select);
    harness.run_steps(4);
    assert!(focused_on(&harness, control), "precondition: {control:?}");
    harness.state_mut().app.ask = Ask::Nothing;
    harness.run_steps(2);
    harness.event(Event::Text(text.into()));
    harness.run_steps(4);
    harness
}

#[test]
fn focus_now_selecting_all_lets_typing_replace_a_field_s_text() {
    let harness = typed_over(Control::TextField, Select::All, "Pump");
    assert_eq!(harness.state().app.text, "Pump");
}

#[test]
fn focus_now_selecting_nothing_types_beside_the_text() {
    let harness = typed_over(Control::TextField, Select::None, "x");
    let text = &harness.state().app.text;
    assert_ne!(text, "x", "the text was kept");
    assert!(text.contains('x') && text.contains("Item 1"), "{text:?}");
}

#[test]
fn focus_now_selecting_all_lets_typing_replace_a_stepper_s_number() {
    let mut harness = typed_over(Control::Stepper, Select::All, "7");
    harness.key_press(egui::Key::Enter);
    harness.run_steps(4);
    assert_eq!(harness.state().app.count, 7.0);
}
