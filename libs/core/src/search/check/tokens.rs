//! Line syntax of search templates (`tf/search/syntax.py::_tokenize` and
//! `parseLine`): each line becomes a token, quantifier blocks are collected
//! verbatim onto the atom they follow, and malformed lines become syntax
//! errors.

use std::sync::LazyLock;

use regex::Regex;

use crate::search::check::feature_spec::{FeatureMap, esc, parse_feature_vals, unesc};

const OP: &str = r"(?:[.#&|\[\]<>:=\-]+\S*)";
const NAME: &str = r"[A-Za-z0-9_.\-]+";

static WHITE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:%|$)").expect("valid regex"));
static QU_LINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(/where/|/have/|/without/|/with/|/or/|/-/)\s*$").expect("valid regex")
});
static INDENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)").expect("valid regex"));
static OP_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(\s*)({OP})\s*$")).expect("valid regex"));
static REL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^(\s*)({NAME})\s+({OP})\s+({NAME})\s*$")).expect("valid regex")
});
static ATOM_OP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(\s*)({OP})\s+([^ \t=#<>~*]+)(?:(?:\s*\z)|(?:\s+(.*)))$"
    ))
    .expect("valid regex")
});
static ATOM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)([^ \t=#<>~*]+)(?:(?:\s*\z)|(?:\s+(.*)))$").expect("valid regex")
});
static NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("^{NAME}$")).expect("valid regex"));

/// A quantifier keyword (`tf/search/syntax.py`: `QWHERE` … `QEND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuKind {
    Where,
    Have,
    Without,
    With,
    Or,
    End,
}

impl QuKind {
    fn parse(keyword: &str) -> Self {
        match keyword {
            "/where/" => Self::Where,
            "/have/" => Self::Have,
            "/without/" => Self::Without,
            "/with/" => Self::With,
            "/or/" => Self::Or,
            _ => Self::End,
        }
    }

    pub(super) fn keyword(self) -> &'static str {
        match self {
            Self::Where => "/where/",
            Self::Have => "/have/",
            Self::Without => "/without/",
            Self::With => "/with/",
            Self::Or => "/or/",
            Self::End => "/-/",
        }
    }

    fn opens(self) -> bool {
        matches!(self, Self::Where | Self::Without | Self::With)
    }

    fn continues(self) -> bool {
        matches!(self, Self::Have | Self::Or)
    }
}

/// A quantifier block attached to an atom: its keyword, the lines of each of
/// its sub-templates, and the line of the keyword.
#[derive(Debug, Clone)]
pub(super) struct RawQuantifier {
    pub(super) kind: QuKind,
    pub(super) templates: Vec<Vec<String>>,
    pub(super) line: usize,
}

/// A relation operator, with the edge-feature condition an edge operator
/// carries (empty otherwise). The operator text is escaped.
#[derive(Debug, Clone)]
pub(super) struct Operator {
    pub(super) text: String,
    pub(super) edge_features: FeatureMap,
}

impl Operator {
    /// The operator as written in the template.
    pub(super) fn display(&self) -> String {
        unesc(&self.text, false)
    }
}

/// An atom line: `[op] [name:]otype [features]`.
#[derive(Debug, Clone)]
pub(super) struct AtomToken {
    pub(super) indent: usize,
    pub(super) op: Option<Operator>,
    pub(super) name: String,
    pub(super) otype: String,
    /// The line without its indent, for quantifier sub-templates.
    pub(super) src: String,
    pub(super) features: FeatureMap,
    pub(super) quantifiers: Vec<RawQuantifier>,
}

#[derive(Debug, Clone)]
pub(super) enum TokenKind {
    Atom(AtomToken),
    /// A line holding only a relation operator.
    Operator {
        indent: usize,
        op: Operator,
    },
    /// A line of feature conditions for the preceding atom.
    Features(FeatureMap),
    /// A relation line `from op to`.
    Relation {
        from: String,
        op: Operator,
        to: String,
    },
}

#[derive(Debug, Clone)]
pub(super) struct Token {
    pub(super) line: usize,
    pub(super) kind: TokenKind,
}

/// A template line classified (TF `parseLine`), on the escaped line.
pub(super) enum ParsedLine {
    Operator {
        indent: String,
        op: String,
    },
    Relation {
        indent: String,
        from: String,
        op: String,
        to: String,
    },
    Atom {
        indent: String,
        op: Option<String>,
        name: String,
        otype: String,
        features: String,
    },
    Features(String),
}

pub(super) fn parse_line(line: &str) -> ParsedLine {
    let esc_line = esc(line);
    if let Some(captures) = OP_LINE_RE.captures(&esc_line)
        && &captures[2] != "."
    {
        return ParsedLine::Operator {
            indent: captures[1].to_string(),
            op: captures[2].to_string(),
        };
    }
    if let Some(captures) = REL_RE.captures(&esc_line) {
        return ParsedLine::Relation {
            indent: captures[1].to_string(),
            from: captures[2].to_string(),
            op: captures[3].to_string(),
            to: captures[4].to_string(),
        };
    }
    let group = |captures: &regex::Captures<'_>, index| {
        captures
            .get(index)
            .map_or_else(String::new, |found: regex::Match<'_>| {
                found.as_str().to_string()
            })
    };
    let with_op = ATOM_OP_RE.captures(&esc_line);
    let mut atom = with_op.as_ref().map(|captures| {
        (
            group(captures, 1),
            Some(group(captures, 2)),
            group(captures, 3),
            group(captures, 4),
        )
    });
    if atom
        .as_ref()
        .is_none_or(|(_, op, _, _)| op.as_deref() == Some("."))
        && let Some(captures) = ATOM_RE.captures(&esc_line)
    {
        atom = Some((
            group(&captures, 1),
            None,
            group(&captures, 2),
            group(&captures, 3),
        ));
    }
    if let Some((indent, op, atom, features)) = atom {
        let (name, otype) = match atom.split_once(':') {
            Some((name, otype)) => (name.to_string(), otype.to_string()),
            None => (String::new(), atom),
        };
        return ParsedLine::Atom {
            indent,
            op,
            name,
            otype,
            features,
        };
    }
    ParsedLine::Features(esc_line)
}

/// Rebuild a template line from its parts (TF `_genLine`).
pub(super) fn gen_line(parsed: &ParsedLine) -> String {
    match parsed {
        ParsedLine::Operator { indent, op } => format!("{indent}{}", unesc(op, false)),
        ParsedLine::Relation {
            indent,
            from,
            op,
            to,
        } => format!("{indent}{from} {} {to}", unesc(op, false)),
        ParsedLine::Atom {
            indent,
            op,
            name,
            otype,
            features,
        } => {
            let op = op
                .as_ref()
                .map_or_else(String::new, |op| format!("{} ", unesc(op, false)));
            let name = if name.is_empty() {
                String::new()
            } else {
                format!("{name}:")
            };
            let mut features = unesc(features, false);
            if !features.is_empty() {
                features.insert(0, ' ');
            }
            format!("{indent}{op}{name}{otype}{features}")
        }
        ParsedLine::Features(features) => unesc(features, false),
    }
}

/// `text` without its first `count` characters (Python `text[count:]`).
pub(super) fn skip_chars(text: &str, count: usize) -> String {
    text.chars().skip(count).collect()
}

/// The open quantifier blocks while tokenizing: the line, keyword and indent
/// of each (TF `curQu`), and where the current block's lines go.
struct QuantifierStack {
    open: Vec<(usize, QuKind, usize)>,
    /// `(token index, quantifier index)` of the outermost open block.
    target: Option<(usize, usize)>,
}

/// Tokenize template lines, or return TF's `badSyntax` messages.
pub(super) fn tokenize(lines: &[&str]) -> Result<Vec<Token>, Vec<(Option<usize>, String)>> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut errors: Vec<(Option<usize>, String)> = Vec::new();
    let mut stack = QuantifierStack {
        open: Vec::new(),
        target: None,
    };

    for (i, &line) in lines.iter().enumerate() {
        if WHITE_RE.is_match(line) {
            continue;
        }
        let (indent, line_qu) = match QU_LINE_RE.captures(line) {
            Some(captures) => (
                captures[1].chars().count(),
                Some(QuKind::parse(&captures[2])),
            ),
            None => (
                INDENT_RE
                    .captures(line)
                    .map_or(0, |captures| captures[1].chars().count()),
                None,
            ),
        };
        if stack.open.is_empty() && line_qu.is_none() {
            if let Err(line_errors) = tokenize_line(i, line, &mut tokens) {
                errors.extend(line_errors.into_iter().map(|error| (Some(i), error)));
            }
            continue;
        }
        quantifier_line(
            i,
            line,
            indent,
            line_qu,
            &mut tokens,
            &mut stack,
            &mut errors,
        );
    }
    for (line, kind, _) in &stack.open {
        errors.push((
            Some(*line),
            format!("Quantifier: Unterminated \"{}\"", kind.keyword()),
        ));
    }
    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

/// The index of the atom a quantifier attaches to: the last atom token,
/// skipping feature lines (TF `lastAtomToken`).
fn last_atom_token(tokens: &[Token]) -> Option<usize> {
    for (index, token) in tokens.iter().enumerate().rev() {
        match token.kind {
            TokenKind::Features(_) => continue,
            TokenKind::Atom(_) => return Some(index),
            _ => return None,
        }
    }
    None
}

/// A line inside a quantifier block or a quantifier keyword line (TF's
/// `UBO` … `PEI` cases).
fn quantifier_line(
    i: usize,
    line: &str,
    indent: usize,
    line_qu: Option<QuKind>,
    tokens: &mut [Token],
    stack: &mut QuantifierStack,
    errors: &mut Vec<(Option<usize>, String)>,
) {
    let depth = stack.open.len();
    let Some(kind) = line_qu else {
        // A line inside a block: keep it, stripped of the indent of the
        // outermost open keyword.
        push_block_line(tokens, stack, skip_chars(line, stack.open[0].2));
        return;
    };
    if depth == 0 {
        // A keyword that opens a new outer block.
        let mut good = true;
        if !kind.opens() {
            errors.push((
                Some(i),
                format!("Quantifier: Can not start with \"{}:\"", kind.keyword()),
            ));
            good = false;
        }
        if tokens.is_empty() {
            errors.push((Some(i), "Quantifier: No preceding tokens".to_string()));
            good = false;
        }
        let last_atom = last_atom_token(tokens);
        let follows_atom = last_atom.is_some_and(
            |index| matches!(&tokens[index].kind, TokenKind::Atom(atom) if atom.indent == indent),
        );
        if !tokens.is_empty() && !follows_atom {
            errors.push((
                Some(i),
                "Quantifier: Does not immediately follow an atom at the same level".to_string(),
            ));
            good = false;
        }
        if !good {
            return;
        }
        let atom_index = last_atom.expect("checked above");
        let TokenKind::Atom(atom) = &mut tokens[atom_index].kind else {
            unreachable!("last_atom_token returns atoms");
        };
        atom.quantifiers.push(RawQuantifier {
            kind,
            templates: vec![Vec::new()],
            line: i,
        });
        stack.target = Some((atom_index, atom.quantifiers.len() - 1));
        stack.open.push((i, kind, indent));
        return;
    }
    if kind.opens() {
        // An inner block opens: its lines pass through verbatim.
        push_block_line(tokens, stack, skip_chars(line, stack.open[0].2));
        stack.open.push((i, kind, indent));
        return;
    }
    let (open_line, open_kind, open_indent) = stack.open[depth - 1];
    let wrong_precursor = if kind.continues() {
        (kind == QuKind::Have && open_kind != QuKind::Where)
            || (kind == QuKind::Or && !matches!(open_kind, QuKind::With | QuKind::Or))
    } else {
        open_kind == QuKind::Where
    };
    if wrong_precursor {
        let message = if kind.continues() {
            format!(
                "Quantifier: \"{}\" can not follow \"{}\" on line {open_line}",
                kind.keyword(),
                open_kind.keyword()
            )
        } else {
            format!(
                "Quantifier: \"{}\" : premature end of \"{}\" on line {open_line}",
                kind.keyword(),
                open_kind.keyword()
            )
        };
        errors.push((Some(i), message));
    }
    if open_indent != indent {
        errors.push((
            Some(i),
            format!(
                "Quantifier \"{}\" has not same indentation as \"{}\" on line {open_line}",
                kind.keyword(),
                open_kind.keyword()
            ),
        ));
    }
    if kind.continues() {
        if depth == 1 {
            if let Some(templates) = block_templates(tokens, stack) {
                templates.push(Vec::new());
            }
        } else {
            push_block_line(tokens, stack, skip_chars(line, stack.open[0].2));
        }
        stack.open[depth - 1] = (i, kind, indent);
    } else {
        if depth == 1 {
            stack.target = None;
        } else {
            push_block_line(tokens, stack, skip_chars(line, stack.open[0].2));
        }
        stack.open.pop();
    }
}

fn block_templates<'t>(
    tokens: &'t mut [Token],
    stack: &QuantifierStack,
) -> Option<&'t mut Vec<Vec<String>>> {
    let (token, quantifier) = stack.target?;
    match &mut tokens[token].kind {
        TokenKind::Atom(atom) => Some(&mut atom.quantifiers[quantifier].templates),
        _ => None,
    }
}

fn push_block_line(tokens: &mut [Token], stack: &QuantifierStack, line: String) {
    if let Some(templates) = block_templates(tokens, stack)
        && let Some(current) = templates.last_mut()
    {
        current.push(line);
    }
}

/// Tokenize a line outside quantifier blocks, or return its syntax errors.
fn tokenize_line(i: usize, line: &str, tokens: &mut Vec<Token>) -> Result<(), Vec<String>> {
    match parse_line(line) {
        ParsedLine::Operator { indent, op } => {
            let op = operator(op).map_err(|error| vec![error])?;
            tokens.push(Token {
                line: i,
                kind: TokenKind::Operator {
                    indent: indent.chars().count(),
                    op,
                },
            });
        }
        ParsedLine::Relation { from, op, to, .. } => {
            let op = operator(op).map_err(|error| vec![error])?;
            tokens.push(Token {
                line: i,
                kind: TokenKind::Relation { from, op, to },
            });
        }
        ParsedLine::Atom {
            indent,
            op,
            name,
            otype,
            features,
        } => {
            let mut errors = Vec::new();
            if !name.is_empty() && !NAME_RE.is_match(&name) {
                errors.push(format!("Illegal name: \"{name}\""));
            }
            let features = match read_features(&features) {
                Ok(features) => Some(features),
                Err(feature_errors) => {
                    errors.extend(feature_errors);
                    None
                }
            };
            let op = match (op, &features) {
                (Some(op), Some(_)) => match operator(op) {
                    Ok(op) => Some(op),
                    Err(error) => {
                        errors.push(error);
                        None
                    }
                },
                _ => None,
            };
            if !errors.is_empty() {
                return Err(errors);
            }
            tokens.push(Token {
                line: i,
                kind: TokenKind::Atom(AtomToken {
                    indent: indent.chars().count(),
                    op,
                    name,
                    otype,
                    src: line.trim_start().to_string(),
                    features: features.expect("no errors"),
                    quantifiers: Vec::new(),
                }),
            });
        }
        ParsedLine::Features(features) => {
            let features = read_features(&features)?;
            tokens.push(Token {
                line: i,
                kind: TokenKind::Features(features),
            });
        }
    }
    Ok(())
}

fn operator(text: String) -> Result<Operator, String> {
    let mut edge_features = FeatureMap::new();
    parse_feature_vals(&text, &mut edge_features, true)?;
    Ok(Operator {
        text,
        edge_features,
    })
}

/// Parse the whitespace-separated feature conditions of a line (TF
/// `readFeatures`), or return the message of every bad condition.
fn read_features(text: &str) -> Result<FeatureMap, Vec<String>> {
    let mut features = FeatureMap::new();
    let mut errors = Vec::new();
    for feat in text.split_whitespace() {
        if let Err(error) = parse_feature_vals(feat, &mut features, false) {
            errors.push(error);
        }
    }
    if errors.is_empty() {
        Ok(features)
    } else {
        Err(errors)
    }
}
