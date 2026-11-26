//! Triggers that open one popup at a time and switch between them: a menu
//! bar's dropdowns, a navigation menu's triggers. While one popup is open,
//! the pointer on a neighbouring trigger, or Left and Right, opens that one
//! in its place. [`resolve`] is the rule; each widget keeps its own
//! [`Groups`] and opens and closes its own popups.

use std::collections::HashMap;

use egui::Id;

use crate::nav::{self, Step};
use crate::overlay::Wrap;
use crate::owned::Enablement;

/// What asks to switch from the open popup of a group to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Switch {
    /// The pointer moved onto this trigger.
    Hover(Id),
    /// Left or Right in the popup of the trigger `from`.
    Step { from: Id, step: Step },
}

impl Switch {
    fn member(self) -> Id {
        match self {
            Self::Hover(trigger) | Self::Step { from: trigger, .. } => trigger,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Member {
    pub(crate) trigger: Id,
    pub(crate) enablement: Enablement,
}

/// The members of one group, as the latest pass declared them.
struct Declared {
    pass: u64,
    wrap: Wrap,
    members: Vec<Member>,
}

/// Every group's members, by the group's id. A group's makeup is fixed
/// while one of its popups is open, so the order of the last pass is the
/// order the keys use.
#[derive(Default)]
pub(crate) struct Groups(HashMap<Id, Declared>);

impl Groups {
    /// Notes `member` as the next of the group `group` in the pass `pass`;
    /// the first member noted in a later pass starts the list again.
    pub(crate) fn join(&mut self, (group, wrap): (Id, Wrap), pass: u64, member: Member) {
        let declared = self.0.entry(group).or_insert_with(|| Declared {
            pass,
            wrap,
            members: Vec::new(),
        });
        if declared.pass != pass {
            declared.pass = pass;
            declared.members.clear();
        }
        declared.wrap = wrap;
        match declared
            .members
            .iter_mut()
            .find(|known| known.trigger == member.trigger)
        {
            Some(known) => *known = member,
            None => declared.members.push(member),
        }
    }

    fn of(&self, trigger: Id) -> Option<&Declared> {
        self.0
            .values()
            .find(|declared| declared.members.iter().any(|m| m.trigger == trigger))
    }

    /// The other members of the group `trigger` is in.
    pub(crate) fn siblings(&self, trigger: Id) -> Vec<Id> {
        self.of(trigger)
            .map(|declared| {
                declared
                    .members
                    .iter()
                    .map(|m| m.trigger)
                    .filter(|&other| other != trigger)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The trigger whose popup `switch` opens, given which are open.
    pub(crate) fn target(&self, switch: Switch, is_open: impl Fn(Id) -> bool) -> Option<Id> {
        let declared = self.of(switch.member())?;
        let open = declared
            .members
            .iter()
            .map(|m| m.trigger)
            .find(|&trigger| is_open(trigger));
        resolve(&declared.members, declared.wrap, open, switch)
    }
}

/// The one rule of switching: the trigger among `members` whose popup
/// `switch` opens, while `open`'s is open. The pointer switches only while
/// another is open; a step goes to the neighbouring enabled member of the
/// open one, wrapping as `wrap` says.
pub(crate) fn resolve(
    members: &[Member],
    wrap: Wrap,
    open: Option<Id>,
    switch: Switch,
) -> Option<Id> {
    let open = open?;
    let enabled: Vec<Id> = members
        .iter()
        .filter(|m| m.enablement == Enablement::Enabled)
        .map(|m| m.trigger)
        .collect();
    let to = match switch {
        Switch::Hover(trigger) => Some(trigger).filter(|t| enabled.contains(t)),
        Switch::Step { from, step } if from == open => {
            nav::target(&enabled, Some(from), step, wrap)
        }
        Switch::Step { .. } => None,
    };
    to.filter(|&to| to != open)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> Id {
        Id::NULL.with(name)
    }

    fn member(name: &str, enablement: Enablement) -> Member {
        Member {
            trigger: id(name),
            enablement,
        }
    }

    fn bar(disabled: &[&str]) -> Vec<Member> {
        ["file", "edit", "view", "help"]
            .iter()
            .map(|name| {
                let enablement = if disabled.contains(name) {
                    Enablement::Disabled
                } else {
                    Enablement::Enabled
                };
                member(name, enablement)
            })
            .collect()
    }

    fn step(open: &str, step: Step, wrap: Wrap, disabled: &[&str]) -> Option<Id> {
        let switch = Switch::Step {
            from: id(open),
            step,
        };
        resolve(&bar(disabled), wrap, Some(id(open)), switch)
    }

    #[test]
    fn hover_switches_only_while_another_popup_is_open() {
        let members = bar(&[]);
        let hover = Switch::Hover(id("edit"));
        assert_eq!(resolve(&members, Wrap::Around, None, hover), None);
        assert_eq!(
            resolve(&members, Wrap::Around, Some(id("file")), hover),
            Some(id("edit"))
        );
        assert_eq!(
            resolve(&members, Wrap::Around, Some(id("edit")), hover),
            None,
            "its own trigger"
        );
    }

    #[test]
    fn hover_on_a_disabled_trigger_switches_nothing() {
        let hover = Switch::Hover(id("edit"));
        assert_eq!(
            resolve(&bar(&["edit"]), Wrap::Around, Some(id("file")), hover),
            None
        );
    }

    #[test]
    fn steps_go_to_the_neighbours_and_wrap() {
        assert_eq!(
            step("file", Step::Next, Wrap::Around, &[]),
            Some(id("edit"))
        );
        assert_eq!(
            step("file", Step::Previous, Wrap::Around, &[]),
            Some(id("help"))
        );
        assert_eq!(
            step("help", Step::Next, Wrap::Around, &[]),
            Some(id("file"))
        );
    }

    #[test]
    fn steps_stop_at_the_ends_without_wrap() {
        assert_eq!(step("help", Step::Next, Wrap::Stop, &[]), None);
        assert_eq!(step("file", Step::Previous, Wrap::Stop, &[]), None);
    }

    #[test]
    fn steps_skip_a_disabled_trigger() {
        assert_eq!(
            step("file", Step::Next, Wrap::Around, &["edit"]),
            Some(id("view"))
        );
        assert_eq!(
            step("view", Step::Previous, Wrap::Around, &["edit"]),
            Some(id("file"))
        );
    }

    #[test]
    fn a_step_from_a_popup_that_is_not_open_switches_nothing() {
        let switch = Switch::Step {
            from: id("edit"),
            step: Step::Next,
        };
        assert_eq!(
            resolve(&bar(&[]), Wrap::Around, Some(id("file")), switch),
            None
        );
    }

    #[test]
    fn a_later_pass_declares_the_members_again() {
        let group = (id("bar"), Wrap::Around);
        let mut groups = Groups::default();
        for name in ["file", "edit"] {
            groups.join(group, 1, member(name, Enablement::Enabled));
        }
        groups.join(group, 1, member("file", Enablement::Enabled));
        assert_eq!(groups.siblings(id("file")), [id("edit")], "no duplicate");
        groups.join(group, 2, member("view", Enablement::Enabled));
        assert_eq!(groups.siblings(id("view")), Vec::<Id>::new());
        assert!(groups.siblings(id("file")).is_empty(), "file left the group");
    }

    #[test]
    fn the_target_is_resolved_among_the_group_of_the_switch() {
        let mut groups = Groups::default();
        for name in ["file", "edit"] {
            groups.join((id("bar"), Wrap::Around), 1, member(name, Enablement::Enabled));
        }
        groups.join((id("other"), Wrap::Around), 1, member("tools", Enablement::Enabled));
        let file_open = |trigger| trigger == id("file");
        assert_eq!(
            groups.target(Switch::Hover(id("edit")), file_open),
            Some(id("edit"))
        );
        assert_eq!(
            groups.target(Switch::Hover(id("tools")), file_open),
            None,
            "another group's popup is not this one's"
        );
    }
}
