//! Reads the class strings out of shadcn's sources: the `cva` tables and the
//! slot `className`s of a registry `.tsx`, and the `@apply` rules of a style's
//! CSS that its `cn-*` classes stand for. Each class keeps its file and byte
//! span, for the report's diagnostics.

use std::collections::BTreeMap;
use std::ops::Range;

/// A source file, by its index in [`Sources::files`].
pub type FileId = usize;

pub struct File {
    pub name: String,
    pub text: String,
}

#[derive(Default)]
pub struct Sources {
    pub files: Vec<File>,
}

impl Sources {
    pub fn add(&mut self, name: String, text: String) -> FileId {
        self.files.push(File { name, text });
        self.files.len() - 1
    }
}

/// One class as written, where it was written.
#[derive(Clone, Debug)]
pub struct Class {
    pub text: String,
    pub file: FileId,
    pub span: Range<usize>,
}

/// The classes of a string literal at `offset` in `file`, split on
/// whitespace.
fn classes_of(literal: &str, file: FileId, offset: usize) -> Vec<Class> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in literal.char_indices().chain([(literal.len(), ' ')]) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                out.push(Class {
                    text: literal[s..i].to_owned(),
                    file,
                    span: offset + s..offset + i,
                });
                start = None;
            }
            _ => {}
        }
    }
    out
}

/// Where a component's classes come from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// The base classes of a `cva` table.
    Base { table: String },
    /// One value of a `cva` variant: `variant = name`.
    Variant {
        table: String,
        variant: String,
        name: String,
    },
    /// A slot's `className`, in the function that renders it.
    Slot { function: String },
}

/// A component's class strings, by where they come from and the part
/// (`data-slot`) they style.
pub struct Component {
    pub parts: Vec<(Origin, String, Vec<Class>)>,
}

/// The part a class list styles: its `cn-*` class without the `cn-`, else
/// the `data-slot` of the element it is written on (`at` lies inside it),
/// else nothing.
fn slot_of(classes: &[Class], text: &str, at: usize) -> String {
    if let Some(cn) = classes.iter().find_map(|c| c.text.strip_prefix("cn-")) {
        return cn.to_owned();
    }
    let open = text[..at].rfind('<').unwrap_or(0);
    let close = at + text[at..].find('>').unwrap_or(text.len() - at);
    text[open..close]
        .split("data-slot=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_default()
        .to_owned()
}

/// A JS string literal's content and the byte offset of its first character.
fn string_at(text: &str, at: usize) -> Option<(&str, usize, usize)> {
    let quote = text[at..].chars().next()?;
    if !matches!(quote, '"' | '\'' | '`') {
        return None;
    }
    let start = at + 1;
    let end = start + text[start..].find(quote)?;
    Some((&text[start..end], start, end + 1))
}

fn skip_space(text: &str, mut at: usize) -> usize {
    while let Some(c) = text[at..].chars().next() {
        if c.is_whitespace() {
            at += c.len_utf8();
        } else if text[at..].starts_with("//") {
            at += text[at..].find('\n').unwrap_or(text.len() - at);
        } else {
            break;
        }
    }
    at
}

/// A JS literal, as far as a `cva` call writes one.
enum Value<'a> {
    Str(&'a str, usize),
    Object(Vec<(String, Value<'a>)>),
    Other,
}

/// Parses the literal at `at`; returns it and the offset after it.
fn value(text: &str, at: usize) -> (Value<'_>, usize) {
    let at = skip_space(text, at);
    if let Some((s, start, end)) = string_at(text, at) {
        return (Value::Str(s, start), end);
    }
    if text[at..].starts_with('{') {
        let mut entries = Vec::new();
        let mut at = at + 1;
        loop {
            at = skip_space(text, at);
            if text[at..].starts_with('}') {
                return (Value::Object(entries), at + 1);
            }
            let (key, after) = match string_at(text, at) {
                Some((s, _, end)) => (s.to_owned(), end),
                None => {
                    let len = text[at..]
                        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                        .unwrap_or(0);
                    if len == 0 {
                        return (Value::Other, at);
                    }
                    (text[at..at + len].to_owned(), at + len)
                }
            };
            at = skip_space(text, after);
            if !text[at..].starts_with(':') {
                return (Value::Other, at);
            }
            let (entry, after) = value(text, at + 1);
            entries.push((key, entry));
            at = skip_space(text, after);
            if text[at..].starts_with(',') {
                at += 1;
            }
        }
    }
    // Anything else (an identifier, a call) ends at the next `,` `}` or `)`.
    let end = text[at..]
        .find([',', '}', ')'])
        .map_or(text.len(), |i| at + i);
    (Value::Other, end)
}

/// The name of the function whose body holds `at`: the last
/// `function Name` before it.
fn function_at(text: &str, at: usize) -> String {
    text[..at]
        .rfind("function ")
        .map(|i| {
            text[i + 9..]
                .split(|c: char| !c.is_alphanumeric())
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .unwrap_or_else(|| "(top level)".to_owned())
}

/// The `cva` tables and slot `className`s of a registry component.
pub fn component(text: &str, file: FileId) -> Component {
    let mut parts = Vec::new();
    let mut search = 0;
    while let Some(i) = text[search..].find("cva(") {
        let at = search + i + 4;
        let table = text[..search + i]
            .rfind("const ")
            .map(|c| {
                text[c + 6..]
                    .split(|ch: char| !ch.is_alphanumeric())
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
            .unwrap_or_default();
        let (base, after) = value(text, at);
        let mut slot = String::new();
        if let Value::Str(s, offset) = base {
            let classes = classes_of(s, file, offset);
            slot = slot_of(&classes, "", 0);
            parts.push((
                Origin::Base {
                    table: table.clone(),
                },
                slot.clone(),
                classes,
            ));
        }
        let comma = skip_space(text, after);
        let (config, end) = value(text, comma + usize::from(text[comma..].starts_with(',')));
        if let Value::Object(entries) = config {
            for (key, entry) in entries {
                if key != "variants" {
                    continue;
                }
                let Value::Object(variants) = entry else {
                    continue;
                };
                for (variant, values) in variants {
                    let Value::Object(values) = values else {
                        continue;
                    };
                    for (name, classes) in values {
                        if let Value::Str(s, offset) = classes {
                            parts.push((
                                Origin::Variant {
                                    table: table.clone(),
                                    variant: variant.clone(),
                                    name,
                                },
                                slot.clone(),
                                classes_of(s, file, offset),
                            ));
                        }
                    }
                }
            }
        }
        search = end.max(at);
    }
    let mut search = 0;
    while let Some(i) = text[search..].find("className=") {
        let at = search + i + "className=".len();
        let function = function_at(text, at);
        let mut classes = Vec::new();
        let end = if let Some((s, start, end)) = string_at(text, at) {
            classes.extend(classes_of(s, file, start));
            end
        } else if text[at..].starts_with("{cn(") {
            // Every string literal among cn()'s arguments, to its closing paren.
            let mut depth = 0usize;
            let mut j = at + 3;
            while let Some(c) = text[j..].chars().next() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    '"' | '\'' | '`' => {
                        if let Some((s, start, end)) = string_at(text, j) {
                            classes.extend(classes_of(s, file, start));
                            j = end;
                            continue;
                        }
                    }
                    _ => {}
                }
                j += c.len_utf8();
            }
            j
        } else {
            at
        };
        if !classes.is_empty() {
            let slot = slot_of(&classes, text, at);
            parts.push((Origin::Slot { function }, slot, classes));
        }
        search = end.max(at);
    }
    Component { parts }
}

/// A style's `@apply` classes, by the `.cn-*` rule that applies them.
pub type Rules = BTreeMap<String, Vec<Class>>;

/// The `@apply` classes of each `.cn-*` rule of a style's CSS.
pub fn apply_rules(text: &str, file: FileId) -> Rules {
    let mut rules = BTreeMap::new();
    let mut search = 0;
    while let Some(i) = text[search..].find(".cn-") {
        let at = search + i + 1;
        let name_len = text[at..]
            .find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(0);
        let name = &text[at..at + name_len];
        let after = skip_space(text, at + name_len);
        search = at + name_len;
        if !text[after..].starts_with('{') {
            continue;
        }
        let body_end = text[after..].find('}').map_or(text.len(), |j| after + j);
        let body = &text[after..body_end];
        if let Some(a) = body.find("@apply") {
            let start = after + a + "@apply".len();
            let end = text[start..body_end]
                .find(';')
                .map_or(body_end, |j| start + j);
            rules
                .entry(name.to_owned())
                .or_insert_with(Vec::new)
                .extend(classes_of(&text[start..end], file, start));
        }
        search = body_end;
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cva_table_gives_its_base_and_each_variant_with_spans() {
        let text = r#"const buttonVariants = cva(
  "cn-button inline-flex",
  {
    variants: {
      variant: { default: "cn-button-variant-default", ghost: "hover:bg-muted" },
      size: { sm: "h-8 px-2.5" },
    },
    defaultVariants: { variant: "default" },
  }
)"#;
        let c = component(text, 0);
        let origins: Vec<_> = c.parts.iter().map(|(o, _, _)| o.clone()).collect();
        assert_eq!(origins.len(), 4, "{origins:?}");
        assert!(c.parts.iter().all(|(_, slot, _)| slot == "button"));
        let (_, _, sm) = &c.parts[3];
        assert_eq!(sm[1].text, "px-2.5");
        assert_eq!(&text[sm[1].span.clone()], "px-2.5");
        assert!(
            matches!(&c.parts[3].0, Origin::Variant { variant, name, .. } if variant == "size" && name == "sm")
        );
    }

    #[test]
    fn a_slots_class_name_is_read_from_cn_and_from_a_plain_string() {
        let text = r#"function ComboboxTrigger() {
  return <X className={cn("cn-combobox-trigger", className)}>
    <Icon className="pointer-events-none size-4" />
}"#;
        let c = component(text, 0);
        let names: Vec<_> = c
            .parts
            .iter()
            .flat_map(|(_, _, cs)| cs.iter().map(|c| c.text.as_str()))
            .collect();
        assert_eq!(
            names,
            ["cn-combobox-trigger", "pointer-events-none", "size-4"]
        );
        assert!(
            matches!(&c.parts[0].0, Origin::Slot { function } if function == "ComboboxTrigger")
        );
    }

    #[test]
    fn each_class_list_names_its_part_by_its_cn_class_else_its_data_slot() {
        let text = r#"function SelectTrigger() {
  return <Trigger data-slot="select-trigger" className={cn("cn-select-trigger", className)}>
    <Icon className="cn-select-trigger-icon pointer-events-none" />
  </Trigger>
}
function SelectLabel() {
  return <Label data-slot="select-label" className={cn("px-2", className)} />
}"#;
        let slots: Vec<_> = component(text, 0)
            .parts
            .into_iter()
            .map(|(_, slot, _)| slot)
            .collect();
        assert_eq!(
            slots,
            ["select-trigger", "select-trigger-icon", "select-label"]
        );
    }

    #[test]
    fn a_css_rule_gives_its_apply_classes_with_spans() {
        let text = ".style-vega {\n  .cn-button-size-sm {\n    @apply h-8 gap-1;\n  }\n}";
        let rules = apply_rules(text, 0);
        let sm = &rules["cn-button-size-sm"];
        assert_eq!(sm.len(), 2);
        assert_eq!(&text[sm[1].span.clone()], "gap-1");
    }
}
