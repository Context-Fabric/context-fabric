//! Feature conditions as Text-Fabric parses them
//! (`tf/search/syntax.py::parseFeatureVals`), and the template escapes
//! (`tf/search/syntax.py::_esc` / `_unesc`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;

/// The escape sequences of a template line, replaced by the control
/// character with their index while a line is classified (TF `ESCAPES`).
const ESCAPES: [&str; 6] = ["\\\\", "\\ ", "\\t", "\\n", "\\|", "\\="];

/// Escapes that stay escaped inside a regular expression (TF `VAL_ESCAPES`).
const VAL_ESCAPES: [&str; 2] = ["\\|", "\\="];

/// The control character standing for an escaped space (`\ `).
const ESCAPED_SPACE: char = '\u{1}';

const FEATURE_NAME: &str = r"[a-zA-Z0-9\-@_]+";

static TRUE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({FEATURE_NAME})[*]\s*$")).expect("valid regex"));
static NONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({FEATURE_NAME})(#?)\s*$")).expect("valid regex"));
static IDENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({FEATURE_NAME})([=#])(.+)$")).expect("valid regex"));
static COMP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({FEATURE_NAME})([<>])(.*)$")).expect("valid regex"));
static NUM_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-?[0-9]+$").expect("valid regex"));
static RE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({FEATURE_NAME})~(.*)$")).expect("valid regex"));

/// A condition on a feature (TF's `featVals`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FeatureSpec {
    /// `f`: any value (TF `True`).
    Exists,
    /// `f#`: no value (TF `None`).
    Missing,
    /// `f*`: no condition (TF `(None, True)`).
    Any,
    /// `f=a|b` (`equal`) or `f#a|b`: one of / none of the values.
    Values {
        equal: bool,
        values: BTreeSet<String>,
    },
    /// `f>n` (`greater`) or `f<n`.
    Limit { greater: bool, bound: i64 },
    /// `f~pattern`.
    Regex(String),
}

impl FeatureSpec {
    /// How Text-Fabric prints the condition (`str(featVals)`) in the
    /// `superfluous` message. A comparison prints as its operator and bound,
    /// where Text-Fabric prints the address of its comparison function.
    pub(super) fn describe(&self) -> String {
        match self {
            Self::Exists => "True".to_string(),
            Self::Missing => "None".to_string(),
            Self::Any => "(None, True)".to_string(),
            Self::Values { equal, values } => {
                let members: Vec<String> = values.iter().map(|value| python_repr(value)).collect();
                let equal = if *equal { "True" } else { "False" };
                format!("({equal}, frozenset({{{}}}))", members.join(", "))
            }
            Self::Limit { greater, bound } => {
                format!("{}{bound}", if *greater { '>' } else { '<' })
            }
            Self::Regex(pattern) => regex_repr(pattern),
        }
    }
}

/// Feature conditions by feature name; a later condition on the same feature
/// replaces an earlier one (a Python dict).
pub(super) type FeatureMap = BTreeMap<String, FeatureSpec>;

/// Parse one feature condition `featStr` (escaped) into `features`.
///
/// With `as_edge`, `featStr` is a relation operator: only edge operators
/// (`-f…>`, `<f…-`, `<f…>`) carry a condition, on the edge feature.
///
/// Returns TF's syntax error message for a bad condition.
pub(super) fn parse_feature_vals(
    feat_str: &str,
    features: &mut FeatureMap,
    as_edge: bool,
) -> Result<(), String> {
    let feat = if as_edge {
        if !is_edge_operator(feat_str) {
            return Ok(());
        }
        let mut chars = feat_str.chars();
        chars.next();
        chars.next_back();
        chars.as_str().to_string()
    } else {
        feat_str.replace(ESCAPED_SPACE, " ")
    };
    let (name, spec) = parse_condition(&feat)?;
    features.insert(name, spec);
    Ok(())
}

/// Whether `op` is an edge relation (`-f>`, `<f-`, `<f>`), which may carry an
/// edge-feature condition.
pub(super) fn is_edge_operator(op: &str) -> bool {
    let (Some(first), Some(last)) = (op.chars().next(), op.chars().next_back()) else {
        return false;
    };
    matches!((first, last), ('-', '>') | ('<', '-') | ('<', '>'))
}

fn parse_condition(feat: &str) -> Result<(String, FeatureSpec), String> {
    if let Some(captures) = TRUE_RE.captures(feat) {
        return Ok((captures[1].to_string(), FeatureSpec::Any));
    }
    if let Some(captures) = NONE_RE.captures(feat) {
        let spec = if captures[2].is_empty() {
            FeatureSpec::Exists
        } else {
            FeatureSpec::Missing
        };
        return Ok((captures[1].to_string(), spec));
    }
    if let Some(captures) = IDENT_RE.captures(feat) {
        let values = captures[3]
            .split('|')
            .map(|value| unesc(value, false))
            .collect();
        let spec = FeatureSpec::Values {
            equal: &captures[2] == "=",
            values,
        };
        return Ok((captures[1].to_string(), spec));
    }
    if let Some(captures) = COMP_RE.captures(feat) {
        let bound = &captures[3];
        if !NUM_RE.is_match(bound) {
            return Err(format!("Limit is non numeric \"{bound}\""));
        }
        let bound = bound
            .parse::<i64>()
            .map_err(|_| format!("Limit is non numeric \"{bound}\""))?;
        let spec = FeatureSpec::Limit {
            greater: &captures[2] == ">",
            bound,
        };
        return Ok((captures[1].to_string(), spec));
    }
    if let Some(captures) = RE_RE.captures(feat) {
        let pattern = unesc(&captures[2], true);
        if let Err(error) = Regex::new(&pattern) {
            return Err(format!(
                "Wrong regular expression \"{pattern}\": \"{}\"",
                regex_error_text(&error)
            ));
        }
        return Ok((captures[1].to_string(), FeatureSpec::Regex(pattern)));
    }
    Err(format!("Unrecognized feature condition \"{feat}\""))
}

/// The one-line reason of a regex error (its `error: …` line).
pub(super) fn regex_error_text(error: &regex::Error) -> String {
    let text = error.to_string();
    text.lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("error: ").map(str::to_string))
        .unwrap_or(text)
}

/// Replace escape sequences by control characters (TF `_esc`).
pub(super) fn esc(text: &str) -> String {
    let mut text = text.to_string();
    for (index, escape) in ESCAPES.iter().enumerate() {
        text = text.replace(escape, &control(index).to_string());
    }
    text
}

/// Restore escaped characters (TF `_unesc`): the escaped character itself, or,
/// inside a regular expression (`in_re`), `\|` and `\=` kept escaped.
pub(super) fn unesc(text: &str, in_re: bool) -> String {
    let mut text = text.to_string();
    for (index, escape) in ESCAPES.iter().enumerate() {
        let replacement = if in_re && VAL_ESCAPES.contains(escape) {
            (*escape).to_string()
        } else {
            escape[1..].to_string()
        };
        text = text.replace(control(index), &replacement);
    }
    text
}

fn control(index: usize) -> char {
    char::from(u8::try_from(index).expect("few escapes"))
}

/// Python's `repr` of a string.
pub(super) fn python_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut repr = String::with_capacity(text.len() + 2);
    repr.push(quote);
    for ch in text.chars() {
        match ch {
            '\\' => repr.push_str("\\\\"),
            '\n' => repr.push_str("\\n"),
            '\r' => repr.push_str("\\r"),
            '\t' => repr.push_str("\\t"),
            ch if ch == quote => {
                repr.push('\\');
                repr.push(ch);
            }
            ch if (ch as u32) < 0x20 || ch as u32 == 0x7f => {
                repr.push_str(&format!("\\x{:02x}", ch as u32));
            }
            ch => repr.push(ch),
        }
    }
    repr.push(quote);
    repr
}

/// Python's `str` of a compiled regular expression.
pub(super) fn regex_repr(pattern: &str) -> String {
    format!("re.compile({})", python_repr(pattern))
}

/// Whether Python's `int()` accepts `text` (surrounding whitespace, a sign,
/// and `_` between digits allowed).
pub(super) fn is_python_int(text: &str) -> bool {
    let text = text.trim();
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    !digits.is_empty()
        && !digits.starts_with('_')
        && !digits.ends_with('_')
        && !digits.contains("__")
        && digits.chars().all(|ch| ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(feat: &str) -> Result<(String, FeatureSpec), String> {
        let mut features = FeatureMap::new();
        parse_feature_vals(&esc(feat), &mut features, false)?;
        Ok(features.into_iter().next().expect("one feature"))
    }

    #[test]
    fn conditions_parse_as_in_text_fabric() {
        assert_eq!(parse("sp").unwrap().1, FeatureSpec::Exists);
        assert_eq!(parse("sp#").unwrap().1, FeatureSpec::Missing);
        assert_eq!(parse("sp*").unwrap().1, FeatureSpec::Any);
        assert_eq!(
            parse(r"sp=a|b\|c").unwrap().1,
            FeatureSpec::Values {
                equal: true,
                values: ["a", "b|c"].into_iter().map(String::from).collect()
            }
        );
        assert_eq!(
            parse("n<-3").unwrap().1,
            FeatureSpec::Limit {
                greater: false,
                bound: -3
            }
        );
        assert_eq!(
            parse(r"sp~a\|b").unwrap().1,
            FeatureSpec::Regex(r"a\|b".into())
        );
    }

    #[test]
    fn bad_conditions_report_text_fabric_messages() {
        assert_eq!(
            parse("strong>abc").unwrap_err(),
            "Limit is non numeric \"abc\""
        );
        assert_eq!(
            parse("sp=").unwrap_err(),
            "Unrecognized feature condition \"sp=\""
        );
        assert_eq!(
            parse("=x").unwrap_err(),
            "Unrecognized feature condition \"=x\""
        );
        assert!(
            parse("sp~[")
                .unwrap_err()
                .starts_with("Wrong regular expression \"[\": \"")
        );
    }

    #[test]
    fn edge_operators_carry_conditions() {
        let mut features = FeatureMap::new();
        parse_feature_vals("-parent=3>", &mut features, true).unwrap();
        assert_eq!(features.len(), 1);
        let mut none = FeatureMap::new();
        parse_feature_vals("<:", &mut none, true).unwrap();
        assert!(none.is_empty());
        assert_eq!(
            parse_feature_vals("<>", &mut none, true).unwrap_err(),
            "Unrecognized feature condition \"\""
        );
    }

    #[test]
    fn python_reprs() {
        assert_eq!(python_repr("a"), "'a'");
        assert_eq!(python_repr("it's"), "\"it's\"");
        assert_eq!(regex_repr("abc"), "re.compile('abc')");
        assert_eq!(
            FeatureSpec::Values {
                equal: true,
                values: ["3".to_string()].into()
            }
            .describe(),
            "(True, frozenset({'3'}))"
        );
        assert!(is_python_int(" -1_000 "));
        assert!(!is_python_int("abc"));
        assert!(!is_python_int("1__0"));
    }
}
