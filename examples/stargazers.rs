//! A virtual list of 5,000 people, each a row with an avatar and a name.
//!
//! A port of the stargazers example in hello_egui's fancy example
//! (github.com/lucasmerlin/hello_egui, `fancy-example/src/stargazers.rs`,
//! MIT), rebuilt with `virtual_list`. The original fetches the people who
//! starred a repository from GitHub, with their avatars, and lets them be
//! reordered by dragging; this one makes up people of the same shape, so it
//! needs no network, draws each avatar as an initial, and leaves the
//! reordering out (drag and drop is not part of this port).

#[path = "common/mod.rs"]
mod common;

use eggshell_ui::{Builder, Scope, tw};

type B<'b, 'f> = Builder<'b, 'f, State>;

/// How many people the list has.
pub const PEOPLE: u32 = 5_000;

/// The height of a row: an avatar and the padding around it.
const ROW: f32 = 48.0;

const SYLLABLES: [&str; 12] = [
    "ka", "ri", "to", "mel", "an", "sor", "vi", "lu", "den", "ix", "or", "pa",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    List,
}

/// A person, by the order they starred in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Person(pub u32);

pub struct State {
    pub people: Vec<Person>,
    pub logins: Vec<String>,
}

/// A login made of syllables chosen by `index`, with a number when the
/// syllables alone could repeat.
fn login(index: u32) -> String {
    let mix = u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 16;
    let at = |shift: u32| SYLLABLES[((mix >> shift) % SYLLABLES.len() as u64) as usize];
    let name = format!("{}{}{}", at(0), at(8), at(16));
    if index < 1_000 {
        name
    } else {
        format!("{name}{}", index % 97)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            people: (0..PEOPLE).map(Person).collect(),
            logins: (0..PEOPLE).map(login).collect(),
        }
    }
}

/// ```html
/// <div class="flex items-center gap-2.5 px-2 h-12">
///   <div class="flex items-center justify-center size-8 shrink-0 rounded-full bg-primary text-primary-foreground">K</div>
///   <div class="flex flex-col"><span>karito</span> <span class="text-muted-foreground">github.com/karito</span></div>
/// </div>
/// ```
fn person<'f>(state: &'f State) -> impl FnMut(&mut Scope<'_, 'f, State>, Person) + 'f {
    move |s, Person(index)| {
        let t = common::tokens(s.ctx());
        let login = state.logins[index as usize].as_str();
        let initial = login.chars().next().map(|c| c.to_ascii_uppercase());
        s.div(tw().flex().items_center().gap(2.5).px(2.0).h_full(), |s| {
            s.div(
                tw().flex()
                    .items_center()
                    .justify_center()
                    .size_px(32.0)
                    .shrink_0()
                    .rounded_full()
                    .bg(t.primary)
                    .text(t.primary_foreground),
                |s| s.text(tw(), initial.map(String::from).unwrap_or_default()),
            );
            s.div(tw().flex().flex_col(), |s| {
                s.text(tw(), login);
                s.text(tw().text(t.muted_foreground), format!("github.com/{login}"));
            });
        });
    }
}

/// ```html
/// <div class="flex flex-col gap-2 size-full p-4">
///   <p class="text-muted-foreground">People who starred the repository</p>
///   <div class="grow min-h-0 overflow-y-auto rounded-md border">rows</div>
/// </div>
/// ```
pub fn layout<'f>(b: &mut B<'_, 'f>, state: &'f State) {
    let t = common::tokens(b.ctx());
    b.div(tw().flex().flex_col().gap(2.0).size_full().p(4.0), |b| {
        b.text(
            tw().text(t.muted_foreground),
            format!("{} people starred the repository", state.people.len()),
        );
        b.virtual_list(
            Part::List,
            tw().grow()
                .min_h_0()
                .rounded_md()
                .border(1.0)
                .border_color(t.border),
            &state.people,
        )
        .fixed_px(ROW)
        .items(person(state));
    });
}

fn main() -> eframe::Result {
    common::run_app("stargazers", State::default(), layout)
}
