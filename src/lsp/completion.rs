use druid::Color;
use serde::{Deserialize, Deserializer};

use crate::theme;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VSCodeCompletionItem {
    label: VSCodeLabel,
    insert_text: VSCodeInsertText,
    #[serde(deserialize_with = "ok_or_none")]
    kind: Option<VSCodeCompletionKind>,
}

impl VSCodeCompletionItem {
    pub fn name(&self) -> String {
        self.label.name()
    }

    pub fn color(&self) -> Color {
        self.kind.map_or(theme::syntax::DEFAULT, |k| k.color())
    }

    pub fn text_to_insert(&self) -> String {
        self.insert_text.value()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum VSCodeLabel {
    Plain(String),
    Detailed(VSCodeDetailedLabel),
}

impl VSCodeLabel {
    pub fn name(&self) -> String {
        match self {
            VSCodeLabel::Plain(s) => s.clone(),
            VSCodeLabel::Detailed(d) => d.label.clone(),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct VSCodeDetailedLabel {
    label: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum VSCodeInsertText {
    Plain(String),
    Snippet(VSCodeSnippetString),
}

impl VSCodeInsertText {
    pub fn value(&self) -> String {
        match self {
            VSCodeInsertText::Plain(s) => s.clone(),
            VSCodeInsertText::Snippet(s) => {
                // remove tab stop syntax
                // TODO: support tab stop syntax (probably as a part of future structural completion)
                let re = regex::Regex::new(r"\$\{\d:(?<inner>.+)\}").unwrap();
                re.replace(&s.value, "$inner").into_owned()
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct VSCodeSnippetString {
    value: String,
}

#[derive(Deserialize, Debug, Clone, Copy)]
enum VSCodeCompletionKind {
    Class,
    Color,
    Constant,
    Constructor,
    Enum,
    EnumMember,
    Event,
    Field,
    File,
    Folder,
    Function,
    Interface,
    Issue,
    Keyword,
    Method,
    Module,
    Operator,
    Property,
    Reference,
    Snippet,
    Struct,
    Text,
    TypeParameter,
    Unit,
    User,
    Value,
    Variable,
}

impl VSCodeCompletionKind {
    pub fn color(self) -> Color {
        use VSCodeCompletionKind::*;

        match self {
            Class | Function | Method => theme::syntax::FUNCTION,
            Constant | Variable | Property => theme::syntax::VARIABLE,
            Keyword => theme::syntax::KEYWORD,
            _ => theme::syntax::DEFAULT,
        }
    }
}

fn ok_or_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(v).ok())
}
