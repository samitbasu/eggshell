//! Text classes. They are inherited by descendants (see [`TextClasses`]).

use egui::{Color32, FontFamily};

use super::Tw;
use crate::text::{
    FontStyle, Numerals, TextAlign, TextCase, TextClasses, TextDecoration, TextSpec, Tracking,
    UserSelect, WhiteSpace,
};

macro_rules! text_flags {
    ($($name:ident, $doc:literal, |$t:ident| $set:expr;)*) => {$(
        #[doc = $doc]
        pub fn $name(self) -> Self {
            self.texted(|$t| $set)
        }
    )*};
}

impl Tw {
    fn texted(mut self, set: impl FnOnce(&mut TextClasses)) -> Self {
        set(&mut self.text);
        self
    }

    /// The text role: size, line height and family.
    pub fn role(self, spec: TextSpec) -> Self {
        self.texted(|t| {
            t.size = Some(spec.size);
            t.line_height = Some(spec.line_height);
            t.family = Some(spec.family);
        })
    }

    /// `text-<color>`.
    pub fn text(self, color: impl Into<Color32>) -> Self {
        self.texted(|t| t.color = Some(color.into()))
    }

    /// `font-<family>`: the family only, keeping the inherited size.
    pub fn font(self, family: FontFamily) -> Self {
        self.texted(|t| t.family = Some(family))
    }

    /// `tracking-*`.
    pub fn tracking(self, tracking: Tracking) -> Self {
        self.texted(|t| t.tracking = Some(tracking))
    }

    text_flags! {
        italic, "`italic`.", |t| t.font_style = Some(FontStyle::Italic);
        not_italic, "`not-italic`.", |t| t.font_style = Some(FontStyle::Normal);
        underline, "`underline`.", |t| t.decoration = Some(TextDecoration::Underline);
        no_underline, "`no-underline`.", |t| t.decoration = Some(TextDecoration::None);
        whitespace_nowrap, "`whitespace-nowrap`.", |t| t.white_space = Some(WhiteSpace::NoWrap);
        whitespace_normal, "`whitespace-normal`.", |t| t.white_space = Some(WhiteSpace::Normal);
        text_left, "`text-left`.", |t| t.align = Some(TextAlign::Left);
        text_center, "`text-center`.", |t| t.align = Some(TextAlign::Center);
        text_right, "`text-right`.", |t| t.align = Some(TextAlign::Right);
        uppercase, "`uppercase`.", |t| t.case = Some(TextCase::Upper);
        capitalize, "`capitalize`.", |t| t.case = Some(TextCase::Capitalize);
        normal_case, "`normal-case`.", |t| t.case = Some(TextCase::Normal);
        select_text, "`select-text`: selectable even inside a node that senses the pointer.", |t| t.select = Some(UserSelect::Text);
        select_none, "`select-none`.", |t| t.select = Some(UserSelect::None);
        tabular_nums, "`tabular-nums`: every digit as wide as the widest, so a number's width does not depend on its digits.", |t| t.numerals = Some(Numerals::Tabular);
        proportional_nums, "`proportional-nums`.", |t| t.numerals = Some(Numerals::Proportional);
    }

    /// `truncate`: no wrapping, an ellipsis at the end, and `overflow-hidden`
    /// so the node may shrink below its text.
    pub fn truncate(self) -> Self {
        self.texted(|t| t.white_space = Some(WhiteSpace::Truncate))
            .overflow_x_hidden()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TextStyle;
    use crate::tw::tw;
    use taffy::Overflow;

    fn base() -> TextStyle {
        TextStyle::new(
            TextSpec {
                size: 14.0,
                line_height: 18.0,
                family: FontFamily::Proportional,
            },
            Color32::BLACK,
        )
    }

    fn resolved(tw: Tw) -> TextStyle {
        tw.text.over(&base())
    }

    #[test]
    fn role_sets_size_line_height_and_family() {
        let spec = TextSpec {
            size: 48.0,
            line_height: 60.0,
            family: FontFamily::Monospace,
        };
        assert_eq!(resolved(tw().role(spec.clone())).spec, spec);
    }

    #[test]
    fn font_changes_only_the_family() {
        let got = resolved(tw().font(FontFamily::Monospace));
        assert_eq!(got.spec.family, FontFamily::Monospace);
        assert_eq!((got.spec.size, got.spec.line_height), (14.0, 18.0));
    }

    #[test]
    fn font_after_role_wins_and_role_after_font_wins() {
        let role = TextSpec {
            size: 20.0,
            line_height: 24.0,
            family: FontFamily::Monospace,
        };
        let name = FontFamily::Name("Bold".into());
        assert_eq!(
            resolved(tw().role(role.clone()).font(name.clone()))
                .spec
                .family,
            name
        );
        assert_eq!(
            resolved(tw().font(name).role(role.clone())).spec.family,
            role.family
        );
    }

    #[test]
    fn flags_and_values() {
        assert_eq!(resolved(tw().text(Color32::RED)).color, Color32::RED);
        assert_eq!(resolved(tw().italic()).font_style, FontStyle::Italic);
        assert_eq!(
            resolved(tw().italic().not_italic()).font_style,
            FontStyle::Normal
        );
        assert_eq!(
            resolved(tw().underline()).decoration,
            TextDecoration::Underline
        );
        assert_eq!(
            resolved(tw().underline().no_underline()).decoration,
            TextDecoration::None
        );
        assert_eq!(
            resolved(tw().whitespace_nowrap()).white_space,
            WhiteSpace::NoWrap
        );
        assert_eq!(
            resolved(tw().whitespace_nowrap().whitespace_normal()).white_space,
            WhiteSpace::Normal
        );
        assert_eq!(resolved(tw().text_center()).align, TextAlign::Center);
        assert_eq!(resolved(tw().text_right()).align, TextAlign::Right);
        assert_eq!(
            resolved(tw().text_center().text_left()).align,
            TextAlign::Left
        );
        assert_eq!(resolved(tw().uppercase()).case, TextCase::Upper);
        assert_eq!(resolved(tw().capitalize()).case, TextCase::Capitalize);
        assert_eq!(
            resolved(tw().uppercase().normal_case()).case,
            TextCase::Normal
        );
        assert_eq!(
            resolved(tw().tracking(Tracking::points(0.5))).tracking,
            Tracking::points(0.5)
        );
    }

    #[test]
    fn numerals_inherit_and_the_later_class_wins() {
        assert_eq!(resolved(tw()).numerals, Numerals::Proportional);
        assert_eq!(resolved(tw().tabular_nums()).numerals, Numerals::Tabular);
        assert_eq!(
            resolved(tw().tabular_nums().proportional_nums()).numerals,
            Numerals::Proportional
        );
        assert_eq!(
            resolved(tw().proportional_nums().tabular_nums()).numerals,
            Numerals::Tabular
        );
    }

    #[test]
    fn truncate_also_lets_the_node_shrink() {
        let truncated = tw().truncate();
        assert_eq!(
            resolved(truncated.clone()).white_space,
            WhiteSpace::Truncate
        );
        assert_eq!(truncated.style().overflow.x, Overflow::Hidden);
        assert_eq!(truncated.style().overflow.y, Overflow::Visible);
    }

    #[test]
    fn text_classes_leave_layout_alone() {
        let styled = tw()
            .role(base().spec)
            .text(Color32::RED)
            .italic()
            .text_center();
        assert_eq!(*styled.style(), *tw().style());
    }
}
