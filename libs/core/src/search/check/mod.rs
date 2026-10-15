//! Template checking: the errors Text-Fabric reports for a bad search
//! template, found before any search work starts.
//!
//! The check follows Text-Fabric's own passes, in its order, and stops after
//! the first pass that fails, as Text-Fabric does:
//!
//! 1. [`tokens`]: line syntax and quantifier structure
//!    (`tf/search/syntax.py::syntax`), reported as `badSyntax`;
//! 2. [`grammar`]: indentation, lonely relations and relation names
//!    (`tf/search/semantics.py::_grammar`), reported as `badSemantics`;
//! 3. [`validation`]: node types, features, feature values and relations
//!    against the corpus (`tf/search/semantics.py::_validation`), reported as
//!    `badSemantics`, then connectedness (`tf/search/graph.py::connectedness`);
//! 4. [`quantifiers`]: every quantifier sub-template, checked the same way
//!    with its line offset in the outer template (`tf/search/spin.py`).
//!
//! Messages carry Text-Fabric's wording and template line numbers.

mod feature_spec;
mod grammar;
mod quantifiers;
mod schema;
mod tokens;
mod validation;

use std::fmt;

pub(crate) use schema::TemplateSchema;

/// Which Text-Fabric pass reported a template error, and so where `S.study`
/// records it on `S.exe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateErrorKind {
    /// A line-level or quantifier-structure error (`S.exe.badSyntax`).
    Syntax,
    /// A grammar or corpus-validation error (`S.exe.badSemantics`).
    Semantics,
    /// A planning error such as a disconnected template; Text-Fabric reports
    /// these without recording them on `badSyntax` / `badSemantics`.
    Plan,
}

/// One error message about a search template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateMessage {
    pub kind: TemplateErrorKind,
    /// The 0-based template line, or `None` for a message about the template
    /// as a whole.
    pub line: Option<usize>,
    pub text: String,
}

impl fmt::Display for TemplateMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "line {line}: {}", self.text),
            None => formatter.write_str(&self.text),
        }
    }
}

/// Every error found in a bad search template, in Text-Fabric's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateErrors {
    messages: Vec<TemplateMessage>,
}

impl TemplateErrors {
    pub(crate) fn new(messages: Vec<TemplateMessage>) -> Self {
        Self { messages }
    }

    /// A single error without a line, for template problems found by the
    /// planner rather than the check.
    pub(crate) fn plan(text: impl Into<String>) -> Self {
        Self::new(vec![TemplateMessage {
            kind: TemplateErrorKind::Plan,
            line: None,
            text: text.into(),
        }])
    }

    pub fn messages(&self) -> &[TemplateMessage] {
        &self.messages
    }

    /// The messages of one kind as `(line, text)` pairs (TF `S.exe.badSyntax`
    /// / `S.exe.badSemantics`).
    pub fn of_kind(&self, kind: TemplateErrorKind) -> Vec<(Option<usize>, String)> {
        self.messages
            .iter()
            .filter(|message| message.kind == kind)
            .map(|message| (message.line, message.text.clone()))
            .collect()
    }
}

impl fmt::Display for TemplateErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, message) in self.messages.iter().enumerate() {
            if index > 0 {
                formatter.write_str("\n")?;
            }
            write!(formatter, "{message}")?;
        }
        Ok(())
    }
}

/// Check `template` against the corpus described by `schema`.
pub(crate) fn check_template(
    template: &str,
    schema: &TemplateSchema<'_>,
) -> Result<(), TemplateErrors> {
    let messages = check_at_offset(template, schema, 0);
    if messages.is_empty() {
        Ok(())
    } else {
        Err(TemplateErrors::new(messages))
    }
}

/// The messages for `template`, whose line 0 is line `offset` of the
/// outermost template (TF `SearchExe.offset`).
fn check_at_offset(
    template: &str,
    schema: &TemplateSchema<'_>,
    offset: usize,
) -> Vec<TemplateMessage> {
    let lines: Vec<&str> = template.split('\n').collect();
    let tokens = match tokens::tokenize(&lines) {
        Ok(tokens) => tokens,
        Err(errors) => return at_offset(TemplateErrorKind::Syntax, errors, offset),
    };
    let query = match grammar::analyze(tokens) {
        Ok(query) => query,
        Err(errors) => return at_offset(TemplateErrorKind::Semantics, errors, offset),
    };
    let validated = validation::validate(&query, schema, offset);
    if !validated.is_empty() {
        return validated;
    }
    quantifiers::check(&query, schema, offset)
}

/// Text-Fabric's `(line, message)` pairs of one kind, with lines shifted by
/// `offset`.
fn at_offset(
    kind: TemplateErrorKind,
    errors: Vec<(Option<usize>, String)>,
    offset: usize,
) -> Vec<TemplateMessage> {
    errors
        .into_iter()
        .map(|(line, text)| TemplateMessage {
            kind,
            line: line.map(|line| line + offset),
            text,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::schema::{EdgeSchema, NodeTypes, ValueType};
    use super::*;

    const VALID_TYPES: &str = "Valid object types are: book, chapter, verse, sentence, group, \
                               clause, wg, phrase, subphrase, word";

    /// N1904's node types, with a few of its features.
    fn schema() -> TemplateSchema<'static> {
        TemplateSchema {
            node_types: NodeTypes::listed(
                [
                    "book",
                    "chapter",
                    "verse",
                    "sentence",
                    "group",
                    "clause",
                    "wg",
                    "phrase",
                    "subphrase",
                    "word",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
            ),
            node_features: HashMap::from([
                ("otype", ValueType::Str),
                ("sp", ValueType::Str),
                ("strong", ValueType::Int),
            ]),
            edge_features: HashMap::from([
                (
                    "parent",
                    EdgeSchema {
                        value_type: ValueType::Int,
                        has_values: false,
                        searchable: true,
                    },
                ),
                (
                    "oslots",
                    EdgeSchema {
                        value_type: ValueType::Int,
                        has_values: false,
                        searchable: false,
                    },
                ),
            ]),
            set_names: None,
        }
    }

    /// The messages for `template` as `(kind, line, text)`.
    fn errors(template: &str) -> Vec<(TemplateErrorKind, Option<usize>, String)> {
        match check_template(template, &schema()) {
            Ok(()) => Vec::new(),
            Err(errors) => errors
                .messages()
                .iter()
                .map(|message| (message.kind, message.line, message.text.clone()))
                .collect(),
        }
    }

    fn syntax(line: usize, text: &str) -> (TemplateErrorKind, Option<usize>, String) {
        (TemplateErrorKind::Syntax, Some(line), text.to_string())
    }

    fn semantics(line: Option<usize>, text: &str) -> (TemplateErrorKind, Option<usize>, String) {
        (TemplateErrorKind::Semantics, line, text.to_string())
    }

    #[test]
    fn good_templates_pass() {
        for template in [
            "word",
            "word sp=verb",
            "word\nsp=verb",
            "word\n\nsp=verb",
            "a.b:word",
            "word sp",
            "word sp#",
            "word sp*",
            "phrase\n  word",
            "sentence\n  word\n  <",
            "word\n<3: word",
            "word\n-parent> word",
            "word\n< word\n  phrase",
            "word sp=a|b",
            "x:word\ny:word\nx .sp. y",
            "x:word\ny:word\nx .strong<strong. y",
            "word\n/without/\n  sp=verb\n/-/",
            "clause\n/where/\n  phrase\n/have/\n  word\n/-/",
            "phrase\n/with/\n  word sp=verb\n/or/\n  word sp=noun\n/-/",
            "sentence\n/without/\n  p:phrase\n  .. [[ p\n/-/",
        ] {
            assert_eq!(errors(template), Vec::new(), "{template:?}");
        }
    }

    #[test]
    fn line_syntax_errors() {
        assert_eq!(
            errors("word strong>abc"),
            vec![syntax(0, "Limit is non numeric \"abc\"")]
        );
        assert_eq!(errors("a$:word"), vec![syntax(0, "Illegal name: \"a$\"")]);
        assert_eq!(
            errors("x:word\ny:word\nx <> y"),
            vec![syntax(2, "Unrecognized feature condition \"\"")]
        );
        assert_eq!(
            errors("word sp= =x"),
            vec![
                syntax(0, "Unrecognized feature condition \"sp=\""),
                syntax(0, "Unrecognized feature condition \"=x\""),
            ]
        );
        let regex = errors("word sp~[");
        assert_eq!(regex.len(), 1);
        assert!(regex[0].2.starts_with("Wrong regular expression \"[\": \""));
    }

    #[test]
    fn quantifier_structure_errors() {
        assert_eq!(
            errors("word\n  /with/\n  word"),
            vec![syntax(
                1,
                "Quantifier: Does not immediately follow an atom at the same level"
            )]
        );
        assert_eq!(
            errors("/with/\nword"),
            vec![syntax(0, "Quantifier: No preceding tokens")]
        );
        assert_eq!(
            errors("word\n/have/\n/-/"),
            vec![
                syntax(1, "Quantifier: Can not start with \"/have/:\""),
                syntax(2, "Quantifier: Can not start with \"/-/:\""),
            ]
        );
        assert_eq!(
            errors("word\n/where/\n  word\n/-/"),
            vec![syntax(
                3,
                "Quantifier: \"/-/\" : premature end of \"/where/\" on line 1"
            )]
        );
        assert_eq!(
            errors("word\n/where/\n  word\n/have/\n  word"),
            vec![syntax(3, "Quantifier: Unterminated \"/have/\"")]
        );
        assert_eq!(
            errors("word\n/with/\nphrase\n /or/\nphrase\n/-/"),
            vec![
                syntax(
                    3,
                    "Quantifier \"/or/\" has not same indentation as \"/with/\" on line 1"
                ),
                syntax(
                    5,
                    "Quantifier \"/-/\" has not same indentation as \"/or/\" on line 3"
                ),
            ]
        );
        assert_eq!(
            errors("word\n/without/\nphrase\n/or/\nphrase\n/-/"),
            vec![syntax(
                3,
                "Quantifier: \"/or/\" can not follow \"/without/\" on line 1"
            )]
        );
    }

    #[test]
    fn grammar_errors() {
        assert_eq!(
            errors("  word"),
            vec![semantics(Some(0), "Unexpected indent: 2, expected 0")]
        );
        assert_eq!(
            errors("sentence\n  word\n    phrase\n   clause"),
            vec![semantics(
                Some(3),
                "Unexpected indent: 3, expected one of 0, 2"
            )]
        );
        assert_eq!(
            errors("<\nword"),
            vec![semantics(
                Some(0),
                "Lonely relation: not allowed at outermost level"
            )]
        );
        assert_eq!(
            errors("sentence\n  <\n  word"),
            vec![semantics(
                Some(1),
                "Lonely relation: not allowed as first child"
            )]
        );
        assert_eq!(
            errors("x:word\nx < z"),
            vec![semantics(Some(1), "Relation with undefined name: \"z\"")]
        );
    }

    #[test]
    fn validation_errors() {
        assert_eq!(
            errors("wordd"),
            vec![
                semantics(Some(0), "Unknown object type: \"wordd\""),
                semantics(None, VALID_TYPES),
            ]
        );
        assert_eq!(
            errors("word\n< nosuch"),
            vec![
                semantics(Some(1), "Unknown object type: \"nosuch\""),
                semantics(None, VALID_TYPES),
            ]
        );
        assert_eq!(
            errors("word nosuchfeature=1"),
            vec![semantics(
                None,
                "Missing feature \"nosuchfeature\" in line(s) 0"
            )]
        );
        assert_eq!(
            errors("word strong=abc"),
            vec![
                semantics(None, "Feature \"strong\" has wrong values:"),
                semantics(None, "    \"abc\" is not a number: line(s) 0"),
            ]
        );
        assert_eq!(
            errors("word strong~abc"),
            vec![
                semantics(None, "Feature \"strong\" has wrong values:"),
                semantics(None, "    \"re.compile('abc')\" is not a number: line(s) 0"),
            ]
        );
        assert_eq!(
            errors("word\n-parent=3> word"),
            vec![
                semantics(None, "Feature \"parent\" cannot have values:"),
                semantics(
                    None,
                    "    \"(True, frozenset({'3'}))\" superfluous: line(s) 1"
                ),
            ]
        );
        assert_eq!(
            errors("x:word\ny:word\nx .strong<sp. y"),
            vec![
                semantics(None, "Feature \"sp\" has wrong type:"),
                semantics(None, "    \"str\" is the wrong type: line(s) 2"),
            ]
        );
        assert_eq!(
            errors("x:word\ny:word\nx .nosuch. y"),
            vec![semantics(None, "Missing feature \"nosuch\" in line(s) 2")]
        );
    }

    #[test]
    fn unknown_relations_list_the_allowed_ones() {
        let legend = format!("Allowed relations:\n{}", crate::search::RELATIONS_LEGEND);
        for (template, op) in [
            ("word\n<x: word", "<x:"),
            ("word\n<03: word", "<03:"),
            ("x:word\n.. word", ".."),
            ("word\n-oslots> word", "-oslots>"),
        ] {
            assert_eq!(
                errors(template),
                vec![
                    semantics(Some(1), &format!("Unknown relation: \"{op}\"")),
                    semantics(None, &legend),
                ],
                "{template:?}"
            );
        }
        assert_eq!(
            errors("word\n<nosuch- word"),
            vec![
                semantics(Some(1), "Unknown relation: \"<nosuch-\""),
                semantics(None, &legend),
                semantics(None, "Missing feature \"nosuch\" in line(s) 1"),
            ]
        );
    }

    #[test]
    fn disconnected_and_empty_templates() {
        let plan = |text: &str| (TemplateErrorKind::Plan, None, text.to_string());
        assert_eq!(
            errors("sp=verb"),
            vec![plan(
                "Search without instructions. Tell me what to look for."
            )]
        );
        assert_eq!(
            errors("x:word\ny:word"),
            vec![
                plan("More than one connected components (2):"),
                plan(
                    "Either run the subqueries one by one, or connect the components by a relation"
                ),
            ]
        );
    }

    #[test]
    fn quantifier_sub_templates_report_under_their_keyword() {
        assert_eq!(
            errors("word\n/without/\n  wordd\n/-/"),
            vec![
                semantics(Some(1), "Error under /without/:"),
                semantics(Some(2), "Unknown object type: \"wordd\""),
                semantics(None, VALID_TYPES),
            ]
        );
        assert_eq!(
            errors("word\n/with/\n  phrase\n/or/\n  wordd\n/-/"),
            vec![
                semantics(Some(3), "Error under /or/:"),
                semantics(Some(4), "Unknown object type: \"wordd\""),
                semantics(None, VALID_TYPES),
            ]
        );
        assert_eq!(
            errors("word\n/where/\n  phrase\n/have/\n  wordd\n/-/"),
            vec![
                semantics(Some(2), "Error under /have/:"),
                semantics(Some(4), "Unknown object type: \"wordd\""),
                semantics(None, VALID_TYPES),
            ]
        );
        // A sub-template that is not connected to the quantified atom.
        assert_eq!(
            errors("word\n/with/\nphrase\n/-/")[0],
            (
                TemplateErrorKind::Plan,
                Some(1),
                "Error under /with/:".to_string()
            )
        );
    }

    #[test]
    fn messages_display_with_their_line() {
        let errors = check_template("wordd", &schema()).unwrap_err();
        assert_eq!(
            errors.to_string(),
            format!("line 0: Unknown object type: \"wordd\"\n{VALID_TYPES}")
        );
        assert_eq!(errors.of_kind(TemplateErrorKind::Syntax), Vec::new());
        assert_eq!(errors.of_kind(TemplateErrorKind::Semantics).len(), 2);
    }
}
