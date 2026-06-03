use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use crate::error::{CfError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalRange {
    Single(u32),
    Range(u32, u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FitemizeValue {
    Str(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    Items(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetValue {
    Str(String),
    Int(i64),
    Items(Vec<String>),
    Set(BTreeSet<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlattenFeatureSpec {
    Name(String),
    Values(SetValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Projection<T> {
    Values(BTreeSet<T>),
    Tuples(BTreeSet<Vec<T>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SilentInput {
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliFlagValue {
    Bool(bool),
    Ternary(i8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliFlagSpec {
    pub description: String,
    pub default: CliFlagValue,
    pub n_values: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliReadResult {
    pub good: bool,
    pub tasks: BTreeMap<String, bool>,
    pub params: BTreeMap<String, String>,
    pub flags: BTreeMap<String, CliFlagValue>,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedFormat {
    pub template: String,
    pub rendered_template: String,
    pub features: Vec<(Vec<String>, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirContext {
    pub home_dir: String,
    pub cur_dir: String,
    pub parent_dir: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttrDict {
    data: BTreeMap<String, serde_json::Value>,
}

impl AttrDict {
    pub fn new(data: BTreeMap<String, serde_json::Value>) -> Self {
        Self { data }
    }

    pub fn empty() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.data.insert(key.into(), value);
    }

    pub fn update(&mut self, values: BTreeMap<String, serde_json::Value>) {
        self.data.extend(values);
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &serde_json::Value> {
        self.data.values()
    }

    pub fn items(&self) -> impl Iterator<Item = (&String, &serde_json::Value)> {
        self.data.iter()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn deepdict(&self) -> serde_json::Value {
        serde_json::Value::Object(
            self.data
                .iter()
                .map(|(key, value)| (key.clone(), deepdict(value)))
                .collect(),
        )
    }
}

pub const VERBOSE: &str = "verbose";
pub const AUTO: &str = "auto";
pub const TERSE: &str = "terse";
pub const DEEP: &str = "deep";
pub const SILENT_D: &str = AUTO;

pub const LOG_LEVEL_DEBUG: u32 = 10;
pub const LOG_LEVEL_INFO: u32 = 20;
pub const LOG_LEVEL_WARNING: u32 = 30;
pub const LOG_LEVEL_ERROR: u32 = 40;
pub const LOCATIONS: &[&str] = &["~/text-fabric-data"];

#[allow(non_snake_case)]
pub fn silentConvert(value: Option<SilentInput>) -> String {
    match value {
        None => SILENT_D.to_string(),
        Some(SilentInput::Bool(false)) => VERBOSE.to_string(),
        Some(SilentInput::Bool(true)) => DEEP.to_string(),
        Some(SilentInput::Str(value))
            if matches!(value.as_str(), VERBOSE | AUTO | TERSE | DEEP) =>
        {
            value
        }
        Some(SilentInput::Str(_)) => SILENT_D.to_string(),
    }
}

pub fn level_map() -> BTreeMap<String, u32> {
    BTreeMap::from([
        (VERBOSE.to_string(), LOG_LEVEL_DEBUG),
        (AUTO.to_string(), LOG_LEVEL_INFO),
        (TERSE.to_string(), LOG_LEVEL_WARNING),
        (DEEP.to_string(), LOG_LEVEL_ERROR),
    ])
}

#[allow(non_snake_case)]
pub fn LEVEL_MAP() -> BTreeMap<String, u32> {
    level_map()
}

pub fn logging_level(silent: Option<SilentInput>) -> u32 {
    *level_map()
        .get(&silentConvert(silent))
        .unwrap_or(&LOG_LEVEL_INFO)
}

pub fn configure_logging(silent: Option<SilentInput>) -> u32 {
    logging_level(silent)
}

pub fn set_logging_level(silent: Option<SilentInput>) -> u32 {
    logging_level(silent)
}

pub fn utcnow() -> SystemTime {
    SystemTime::now()
}

pub fn var(env_var: &str) -> Option<String> {
    env::var(env_var).ok()
}

pub const WARN32: &str = "WARNING: you are not running a 64-bit implementation of Python.\nYou may run into memory problems if you load a big data set.\nConsider installing a 64-bit Python.\n";
pub const MSG64: &str = "Running on 64-bit Python";

pub fn check32() -> (bool, String, String) {
    let on32 = usize::BITS < 64;
    if on32 {
        (true, WARN32.to_string(), String::new())
    } else {
        (false, String::new(), MSG64.to_string())
    }
}

pub fn console_message(messages: &[&str], newline: bool) -> String {
    let mut message = messages.join(" ");
    message = if message.is_empty() {
        String::new()
    } else {
        unexpanduser(&message)
    };
    if let Some(stripped) = message.strip_prefix('\n') {
        message = stripped.to_string();
    }
    if let Some(stripped) = message.strip_suffix('\n') {
        message = stripped.to_string();
    }
    if newline {
        message.push('\n');
    }
    message
}

pub fn console(messages: &[&str], error: bool, newline: bool) -> Result<()> {
    let message = console_message(messages, newline);
    if error {
        let mut stderr = io::stderr().lock();
        stderr.write_all(message.as_bytes())?;
        stderr.flush()?;
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(message.as_bytes())?;
        stdout.flush()?;
    }
    Ok(())
}

pub fn deepdict(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(deepdict).collect())
        }
        serde_json::Value::Object(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), deepdict(value)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[allow(non_snake_case)]
pub fn deepAttrDict(value: &serde_json::Value, prefer_tuples: bool) -> serde_json::Value {
    let _ = prefer_tuples;
    deepdict(value)
}

pub fn deep_attr_dict(value: &serde_json::Value, prefer_tuples: bool) -> serde_json::Value {
    deepAttrDict(value, prefer_tuples)
}

#[allow(non_snake_case)]
pub fn isIterable(value: &serde_json::Value) -> bool {
    matches!(
        value,
        serde_json::Value::Array(_) | serde_json::Value::Object(_)
    )
}

pub fn is_iterable(value: &serde_json::Value) -> bool {
    isIterable(value)
}

pub fn read_args(
    command: &str,
    description: &str,
    possible_tasks: &BTreeMap<String, String>,
    possible_params: &BTreeMap<String, (String, String)>,
    possible_flags: &BTreeMap<String, CliFlagSpec>,
    not_in_all: &BTreeSet<String>,
    args: &[String],
) -> CliReadResult {
    let mut task_args = BTreeSet::new();
    let mut help_tasks = Vec::new();
    for (task, help) in possible_tasks {
        help_tasks.push(format!("\t{task}:\n\t\t{help}\n"));
        task_args.insert(task.clone());
    }
    let not_in_all_rep = if not_in_all.is_empty() {
        String::new()
    } else {
        format!(
            " except {}",
            not_in_all.iter().cloned().collect::<Vec<_>>().join(", ")
        )
    };
    help_tasks.push(format!("all:\n\t\tall tasks{not_in_all_rep}"));
    task_args.insert("all".to_string());

    let mut param_defaults = BTreeMap::new();
    let mut help_params = Vec::new();
    for (param, (help, default)) in possible_params {
        help_params.push(format!("\t{param}={default}:\n\t\t{help}\n"));
        param_defaults.insert(param.clone(), default.clone());
    }

    let mut flag_defaults = BTreeMap::new();
    let mut flag_args = BTreeMap::new();
    let mut help_flags = Vec::new();
    for (flag, spec) in possible_flags {
        help_flags.push(format!(
            "\t{flag}={}:\n\t\t{}\n",
            cli_flag_value_rep(&spec.default),
            spec.description
        ));
        match spec.n_values {
            2 => {
                flag_defaults.insert(flag.clone(), spec.default.clone());
                flag_args.insert(format!("-{flag}"), CliFlagValue::Bool(false));
                flag_args.insert(format!("+{flag}"), CliFlagValue::Bool(true));
                help_flags.push(format!("\t\t-{flag}: no {flag}"));
                help_flags.push(format!("\t\t+{flag}: yes {flag}"));
            }
            3 => {
                flag_defaults.insert(flag.clone(), spec.default.clone());
                flag_args.insert(format!("-{flag}"), CliFlagValue::Ternary(-1));
                flag_args.insert(format!("+{flag}"), CliFlagValue::Ternary(0));
                flag_args.insert(format!("++{flag}"), CliFlagValue::Ternary(1));
                help_flags.push(format!("\t\t-{flag}: no {flag}"));
                help_flags.push(format!("\t\t+{flag}: a bit {flag}"));
                help_flags.push(format!("\t\t++{flag}: more {flag}"));
            }
            _ => {}
        }
    }

    let help_text = format!(
        "{command} [tasks/params/flags] [--help]\n\n{description}\n\n--help: show this text and exit\n\n\
tasks:\n{}\n\nparameters:\n{}\n\nflags:\n{}",
        help_tasks.join(""),
        help_params.join(""),
        help_flags.join("\n")
    );

    let arg_set = args.iter().cloned().collect::<BTreeSet<_>>();
    if arg_set.is_empty() || arg_set.contains("--help") || arg_set.contains("-h") {
        let mut messages = vec![help_text];
        if arg_set.is_empty() {
            messages.push("No task specified".to_string());
        }
        return CliReadResult {
            good: true,
            tasks: BTreeMap::new(),
            params: BTreeMap::new(),
            flags: BTreeMap::new(),
            messages,
        };
    }

    let possible_args = task_args
        .iter()
        .cloned()
        .chain(flag_args.keys().cloned())
        .collect::<BTreeSet<_>>();
    let illegal_args = arg_set
        .iter()
        .filter(|arg| {
            !possible_args.contains(*arg)
                && !param_defaults
                    .contains_key(arg.split_once('=').map(|(key, _)| key).unwrap_or(arg))
        })
        .cloned()
        .collect::<Vec<_>>();
    if !illegal_args.is_empty() {
        let mut messages = vec![help_text];
        messages.extend(
            illegal_args
                .into_iter()
                .map(|arg| format!("Illegal argument `{arg}`")),
        );
        return CliReadResult {
            good: false,
            tasks: BTreeMap::new(),
            params: BTreeMap::new(),
            flags: BTreeMap::new(),
            messages,
        };
    }

    let mut tasks = BTreeMap::new();
    let mut params = BTreeMap::new();
    let mut flags = BTreeMap::new();
    for arg in arg_set {
        if task_args.contains(&arg) {
            tasks.insert(arg, true);
        } else if let Some(value) = flag_args.get(&arg) {
            flags.insert(
                arg.trim_start_matches(['+', '-']).to_string(),
                value.clone(),
            );
        } else {
            let (param, value) = arg.split_once('=').unwrap_or((&arg, ""));
            let value = if value.is_empty() {
                param_defaults.get(param).cloned().unwrap_or_default()
            } else {
                value.to_string()
            };
            params.insert(param.to_string(), value);
        }
    }

    for (flag, default) in flag_defaults {
        flags.entry(flag).or_insert(default);
    }
    for (param, default) in param_defaults {
        params.entry(param).or_insert(default);
    }

    if tasks.get("all").copied().unwrap_or(false) {
        tasks = possible_tasks
            .keys()
            .filter(|task| task.as_str() != "all" && !not_in_all.contains(*task))
            .map(|task| (task.clone(), true))
            .collect();
    } else {
        tasks.remove("all");
    }

    CliReadResult {
        good: true,
        tasks,
        params,
        flags,
        messages: Vec::new(),
    }
}

#[allow(non_snake_case)]
pub fn readArgs(
    command: &str,
    description: &str,
    possible_tasks: &BTreeMap<String, String>,
    possible_params: &BTreeMap<String, (String, String)>,
    possible_flags: &BTreeMap<String, CliFlagSpec>,
    not_in_all: &BTreeSet<String>,
    args: &[String],
) -> CliReadResult {
    read_args(
        command,
        description,
        possible_tasks,
        possible_params,
        possible_flags,
        not_in_all,
        args,
    )
}

fn cli_flag_value_rep(value: &CliFlagValue) -> String {
    match value {
        CliFlagValue::Bool(value) => value.to_string(),
        CliFlagValue::Ternary(value) => value.to_string(),
    }
}

pub fn version_sort(version: &str) -> Vec<(u64, String, String)> {
    version
        .split('.')
        .map(|part| {
            let digit_len = part
                .char_indices()
                .take_while(|(_, ch)| ch.is_ascii_digit())
                .map(|(index, ch)| index + ch.len_utf8())
                .last()
                .unwrap_or(0);
            let (digits, rest) = part.split_at(digit_len);
            let alpha_len = rest
                .char_indices()
                .take_while(|(_, ch)| !ch.is_ascii_digit())
                .map(|(index, ch)| index + ch.len_utf8())
                .last()
                .unwrap_or(0);
            let (alpha, tail) = rest.split_at(alpha_len);
            (
                digits.parse::<u64>().unwrap_or(0),
                alpha.to_string(),
                tail.to_string(),
            )
        })
        .collect()
}

#[allow(non_snake_case)]
pub fn versionSort(version: &str) -> Vec<(u64, String, String)> {
    version_sort(version)
}

pub fn nbytes(bytes: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes;
    for (index, unit) in units.iter().enumerate() {
        if value < 1024.0 || index == units.len() - 1 {
            return if index == 0 {
                format!("{value:>5.0}{unit}")
            } else {
                format!("{value:>5.1}{unit}")
            };
        }
        value /= 1024.0;
    }
    String::new()
}

pub fn itemize(value: Option<&str>, sep: Option<&str>) -> Vec<String> {
    let Some(value) = value else {
        return Vec::new();
    };
    if value.is_empty() {
        return Vec::new();
    }
    match sep {
        Some(sep) => value.trim().split(sep).map(ToString::to_string).collect(),
        None => value.split_whitespace().map(ToString::to_string).collect(),
    }
}

pub fn fitemize(value: Option<FitemizeValue>) -> Vec<String> {
    match value {
        None => Vec::new(),
        Some(FitemizeValue::Str(value)) => split_helper_items(&value),
        Some(FitemizeValue::Bool(value)) => {
            vec![if value { "True" } else { "False" }.to_string()]
        }
        Some(FitemizeValue::Int(value)) => vec![value.to_string()],
        Some(FitemizeValue::Float(value)) => vec![value.to_string()],
        Some(FitemizeValue::Items(values)) => values,
    }
}

pub fn project<T>(rows: impl IntoIterator<Item = Vec<T>>, max_dimension: usize) -> Projection<T>
where
    T: Clone + Ord,
{
    if max_dimension == 1 {
        return Projection::Values(
            rows.into_iter()
                .filter_map(|row| row.first().cloned())
                .collect(),
        );
    }
    Projection::Tuples(
        rows.into_iter()
            .map(|row| row.into_iter().take(max_dimension).collect::<Vec<_>>())
            .collect(),
    )
}

pub fn set_from_value(value: Option<SetValue>, as_int: bool) -> BTreeSet<String> {
    match value {
        None => BTreeSet::new(),
        Some(SetValue::Set(values)) => values,
        Some(SetValue::Str(value)) => split_helper_items(&value)
            .into_iter()
            .filter_map(|part| {
                if as_int {
                    part.chars().all(|ch| ch.is_ascii_digit()).then_some(part)
                } else if part.is_empty() {
                    None
                } else {
                    Some(part)
                }
            })
            .collect(),
        Some(SetValue::Items(values)) => values
            .into_iter()
            .filter_map(|part| {
                if as_int {
                    part.chars().all(|ch| ch.is_ascii_digit()).then_some(part)
                } else if part.is_empty() {
                    None
                } else {
                    Some(part)
                }
            })
            .collect(),
        Some(SetValue::Int(value)) => BTreeSet::from([value.to_string()]),
    }
}

#[allow(non_snake_case)]
pub fn setFromValue(value: Option<SetValue>, as_int: bool) -> BTreeSet<String> {
    set_from_value(value, as_int)
}

pub fn set_from_str(value: Option<&str>) -> BTreeSet<String> {
    value
        .map(split_helper_items)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

#[allow(non_snake_case)]
pub fn setFromStr(value: Option<&str>) -> BTreeSet<String> {
    set_from_str(value)
}

pub fn flatten_to_set(features: &[FlattenFeatureSpec]) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    for feature in features {
        match feature {
            FlattenFeatureSpec::Name(name) => {
                result.insert(name.clone());
            }
            FlattenFeatureSpec::Values(value) => {
                result.extend(set_from_value(Some(value.clone()), false));
            }
        }
    }
    result
}

#[allow(non_snake_case)]
pub fn flattenToSet(features: &[FlattenFeatureSpec]) -> BTreeSet<String> {
    flatten_to_set(features)
}

pub fn merge_dict_of_sets<K, V>(
    source: &mut BTreeMap<K, BTreeSet<V>>,
    overrides: &BTreeMap<K, BTreeSet<V>>,
) where
    K: Clone + Ord,
    V: Clone + Ord,
{
    for (key, values) in overrides {
        source
            .entry(key.clone())
            .or_default()
            .extend(values.iter().cloned());
    }
}

#[allow(non_snake_case)]
pub fn mergeDictOfSets<K, V>(
    source: &mut BTreeMap<K, BTreeSet<V>>,
    overrides: &BTreeMap<K, BTreeSet<V>>,
) where
    K: Clone + Ord,
    V: Clone + Ord,
{
    merge_dict_of_sets(source, overrides);
}

pub fn merge_dict(
    source: &mut BTreeMap<String, serde_json::Value>,
    overrides: &BTreeMap<String, serde_json::Value>,
) {
    for (key, value) in overrides {
        match (source.get_mut(key), value) {
            (Some(serde_json::Value::Object(existing)), serde_json::Value::Object(incoming)) => {
                let mut existing_map = existing
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<BTreeMap<_, _>>();
                let incoming_map = incoming
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<BTreeMap<_, _>>();
                merge_dict(&mut existing_map, &incoming_map);
                *existing = existing_map.into_iter().collect();
            }
            _ => {
                source.insert(key.clone(), value.clone());
            }
        }
    }
}

#[allow(non_snake_case)]
pub fn mergeDict(
    source: &mut BTreeMap<String, serde_json::Value>,
    overrides: &BTreeMap<String, serde_json::Value>,
) {
    merge_dict(source, overrides);
}

pub fn format_meta(
    feature_meta: &BTreeMap<String, BTreeMap<String, String>>,
) -> BTreeMap<String, BTreeMap<String, String>> {
    feature_meta
        .iter()
        .map(|(feature, meta)| {
            let formatted = meta
                .iter()
                .filter_map(|(key, value)| {
                    if key == "eg" && meta.contains_key("desc") {
                        return None;
                    }
                    if key == "desc" {
                        let example = meta
                            .get("eg")
                            .filter(|example| !example.is_empty())
                            .map(|example| format!(" ({example})"))
                            .unwrap_or_default();
                        return Some(("description".to_string(), format!("{value}{example}")));
                    }
                    Some((key.clone(), value.clone()))
                })
                .collect::<BTreeMap<_, _>>();
            (feature.clone(), formatted)
        })
        .collect()
}

#[allow(non_snake_case)]
pub fn formatMeta(
    feature_meta: &BTreeMap<String, BTreeMap<String, String>>,
) -> BTreeMap<String, BTreeMap<String, String>> {
    format_meta(feature_meta)
}

pub fn collect_formats(
    config: &BTreeMap<String, String>,
) -> (BTreeMap<String, CollectedFormat>, Vec<String>) {
    let mut feature_set = BTreeSet::new();
    let mut formats = BTreeMap::new();

    for (name, template) in config {
        let Some(format_name) = name.strip_prefix("fmt:") else {
            continue;
        };
        let mut rendered_template = String::new();
        let mut features = Vec::new();
        let mut rest = template.as_str();

        while let Some(start) = rest.find('{') {
            rendered_template.push_str(&rest[..start]);
            let after_start = &rest[start + 1..];
            let Some(end) = after_start.find('}') else {
                rendered_template.push_str(&rest[start..]);
                rest = "";
                break;
            };
            let placeholder = &after_start[..end];
            let (feature_part, default) = placeholder
                .split_once(':')
                .map(|(feature_part, default)| (feature_part, default))
                .unwrap_or((placeholder, ""));
            let alternatives = feature_part
                .split('/')
                .filter(|feature| !feature.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            for feature in &alternatives {
                feature_set.insert(feature.clone());
            }
            features.push((alternatives, default.to_string()));
            rendered_template.push_str("{}");
            rest = &after_start[end + 1..];
        }
        rendered_template.push_str(rest);

        formats.insert(
            format_name.to_string(),
            CollectedFormat {
                template: template.clone(),
                rendered_template,
                features,
            },
        );
    }

    (formats, feature_set.into_iter().collect())
}

#[allow(non_snake_case)]
pub fn collectFormats(
    config: &BTreeMap<String, String>,
) -> (BTreeMap<String, CollectedFormat>, Vec<String>) {
    collect_formats(config)
}

pub fn make_examples(nodes: &[u32]) -> String {
    if nodes.len() <= 10 {
        format!(
            "{:>7} x: {}",
            nodes.len(),
            nodes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!(
            "{:>7} x: {} ... {}",
            nodes.len(),
            nodes[0..5]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            nodes[nodes.len() - 5..]
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[allow(non_snake_case)]
pub fn makeExamples(nodes: &[u32]) -> String {
    make_examples(nodes)
}

pub fn deep_size_json(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Null => std::mem::size_of::<serde_json::Value>(),
        serde_json::Value::Bool(_) => std::mem::size_of::<serde_json::Value>(),
        serde_json::Value::Number(_) => {
            std::mem::size_of::<serde_json::Value>() + std::mem::size_of::<serde_json::Number>()
        }
        serde_json::Value::String(value) => std::mem::size_of::<serde_json::Value>() + value.len(),
        serde_json::Value::Array(values) => {
            std::mem::size_of::<serde_json::Value>()
                + values.iter().map(deep_size_json).sum::<usize>()
        }
        serde_json::Value::Object(values) => {
            std::mem::size_of::<serde_json::Value>()
                + values
                    .iter()
                    .map(|(key, value)| key.len() + deep_size_json(value))
                    .sum::<usize>()
        }
    }
}

#[allow(non_snake_case)]
pub fn deepSizeJson(value: &serde_json::Value) -> usize {
    deep_size_json(value)
}

#[allow(non_snake_case)]
pub fn deepSize(value: &serde_json::Value) -> usize {
    deep_size_json(value)
}

pub fn normpath(path: Option<&str>) -> Option<String> {
    let path = path?;
    let normalized = normalize_path_string(path);
    if normalized.is_empty() {
        Some(".".to_string())
    } else {
        Some(normalized)
    }
}

pub fn abspath(path: &str) -> String {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    normpath(Some(&absolute.to_string_lossy())).unwrap_or_default()
}

pub fn expanduser(path: &str) -> String {
    let normalized = normpath(Some(path)).unwrap_or_default();
    if normalized == "~" {
        return home_dir();
    }
    if let Some(rest) = normalized.strip_prefix("~/") {
        return format!("{}/{}", home_dir(), rest);
    }
    normalized
}

pub fn unexpanduser(path: &str) -> String {
    let normalized = normpath(Some(path)).unwrap_or_default();
    let home = home_dir();
    if normalized == home {
        "~".to_string()
    } else if let Some(rest) = normalized.strip_prefix(&format!("{home}/")) {
        format!("~/{rest}")
    } else {
        normalized
    }
}

#[allow(non_snake_case)]
pub fn setDir() -> DirContext {
    let cur_dir = getCwd().unwrap_or_else(|_| ".".to_string());
    DirContext {
        home_dir: expanduser("~"),
        parent_dir: dirNm(&cur_dir),
        cur_dir,
    }
}

#[allow(non_snake_case)]
pub fn expandDir(context: &DirContext, dir_name: &str) -> String {
    let expanded = if let Some(rest) = dir_name.strip_prefix('~') {
        format!("{}{}", context.home_dir, rest)
    } else if let Some(rest) = dir_name.strip_prefix("..") {
        format!("{}{}", context.parent_dir, rest)
    } else if let Some(rest) = dir_name.strip_prefix("./") {
        format!("{}/{}", context.cur_dir, rest)
    } else if dir_name == "." {
        context.cur_dir.clone()
    } else if Path::new(dir_name).is_absolute() {
        dir_name.to_string()
    } else {
        format!("{}/{}", context.cur_dir, dir_name)
    };
    normpath(Some(&expanded)).unwrap_or_default()
}

#[allow(non_snake_case)]
pub fn prefixSlash(path: Option<&str>) -> Option<String> {
    let path = path?;
    if path.is_empty() || path.starts_with('/') {
        Some(path.to_string())
    } else {
        Some(format!("/{path}"))
    }
}

#[allow(non_snake_case)]
pub fn dirNm(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|parent| {
            let normalized = normalize_path_string(&parent.to_string_lossy());
            if normalized == "." {
                String::new()
            } else {
                normalized
            }
        })
        .unwrap_or_default()
}

#[allow(non_snake_case)]
pub fn fileNm(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|file| file.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[allow(non_snake_case)]
pub fn extNm(path: &str) -> String {
    fileNm(path)
        .rsplit_once('.')
        .map_or_else(|| fileNm(path), |(_, extension)| extension.to_string())
}

#[allow(non_snake_case)]
pub fn stripExt(path: &str) -> String {
    let directory = dirNm(path);
    let file = fileNm(path);
    let stem = file
        .rsplit_once('.')
        .map_or(file.as_str(), |(stem, _)| stem);
    if directory.is_empty() {
        stem.to_string()
    } else {
        format!("{directory}/{stem}")
    }
}

#[allow(non_snake_case)]
pub fn replaceExt(path: &str, new_ext: &str) -> String {
    format!("{}.{}", stripExt(path), new_ext)
}

#[allow(non_snake_case)]
pub fn splitPath(path: &str) -> (String, String) {
    (dirNm(path), fileNm(path))
}

#[allow(non_snake_case)]
pub fn splitExt(path: &str) -> (String, String) {
    let Some((stem, extension)) = path.rsplit_once('.') else {
        return (path.to_string(), String::new());
    };
    if stem.is_empty() || extension.contains('/') {
        (path.to_string(), String::new())
    } else {
        (stem.to_string(), format!(".{extension}"))
    }
}

#[allow(non_snake_case)]
pub fn scanDir(path: &str) -> Result<Vec<String>> {
    if !dirExists(Some(path)) {
        return Ok(Vec::new());
    }
    let mut entries = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().to_string()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort();
    Ok(entries)
}

#[allow(non_snake_case)]
pub fn backendRep(backend: Option<&str>, kind: &str, default: Option<&str>) -> Option<String> {
    let raw = backend.unwrap_or("").to_ascii_lowercase();
    let backend = match raw.as_str() {
        "" | "github" | "github.com" => "github".to_string(),
        "gitlab" | "gitlab.com" => "gitlab".to_string(),
        _ => raw,
    };
    match kind {
        "norm" => Some(backend),
        "tech" => Some(if backend == "github" {
            "github".to_string()
        } else {
            "gitlab".to_string()
        }),
        "name" => Some(match backend.as_str() {
            "github" => "GitHub".to_string(),
            "gitlab" => "GitLab".to_string(),
            _ => backend,
        }),
        "machine" => Some(match backend.as_str() {
            "github" => "github.com".to_string(),
            "gitlab" => "gitlab.com".to_string(),
            _ => backend,
        }),
        "rep" | "spec" => {
            if default
                .and_then(|default| backendRep(Some(default), "norm", None))
                .is_some_and(|default| default == backend)
            {
                Some(String::new())
            } else {
                Some(format!("<{backend}>"))
            }
        }
        "clone" => Some(format!("{}/{}", home_dir(), backend)),
        "cache" => Some(format!("{}/text-fabric-data/{}", home_dir(), backend)),
        "url" => Some(match backend.as_str() {
            "github" => "https://github.com".to_string(),
            "gitlab" => "https://gitlab.com".to_string(),
            _ => format!("https://{backend}"),
        }),
        "urlnb" => Some(format!("https://nbviewer.org/github/{backend}")),
        "pages" => Some(match backend.as_str() {
            "github" => "github.io".to_string(),
            "gitlab" => "gitlab.io".to_string(),
            _ => {
                let tail = backend
                    .split_once('.')
                    .map(|(_, tail)| tail)
                    .unwrap_or(&backend);
                format!("pages.{tail}")
            }
        }),
        _ => None,
    }
}

#[allow(non_snake_case)]
pub fn isFile(path: &str) -> bool {
    Path::new(path).is_file()
}

#[allow(non_snake_case)]
pub fn isDir(path: &str) -> bool {
    Path::new(path).is_dir()
}

#[allow(non_snake_case)]
pub fn fileExists(path: &str) -> bool {
    isFile(path)
}

#[allow(non_snake_case)]
pub fn dirExists(path: Option<&str>) -> bool {
    match path {
        None => false,
        Some("") => true,
        Some(path) => Path::new(path).is_dir(),
    }
}

#[allow(non_snake_case)]
pub fn fileMake(path: &str, force: bool) -> Result<()> {
    if Path::new(path).is_dir() || (!force && fileExists(path)) {
        return Ok(());
    }
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::File::create(path)?;
    Ok(())
}

#[allow(non_snake_case)]
pub fn fileRemove(path: &str) -> Result<()> {
    if fileExists(path) {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[allow(non_snake_case)]
pub fn fileCopy(source: &str, target: &str) -> Result<()> {
    if source == target {
        return Ok(());
    }
    if fileExists(source) {
        fileRemove(target)?;
        if let Some(parent) = Path::new(target).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::copy(source, target)?;
    }
    Ok(())
}

#[allow(non_snake_case)]
pub fn fileMove(source: &str, target: &str) -> Result<()> {
    if fileExists(source) {
        fileRemove(target)?;
    }
    fs::rename(source, target)?;
    Ok(())
}

#[allow(non_snake_case)]
pub fn dirMake(path: &str) -> Result<()> {
    if !dirExists(Some(path)) {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

#[allow(non_snake_case)]
pub fn dirRemove(path: &str) -> Result<()> {
    if dirExists(Some(path)) {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[allow(non_snake_case)]
pub fn dirCopy(source: &str, target: &str, noclobber: bool) -> Result<bool> {
    if !dirExists(Some(source)) {
        return Ok(false);
    }
    if dirExists(Some(target)) {
        if noclobber {
            return Ok(false);
        }
        dirRemove(target)?;
    }
    copy_dir_recursive(Path::new(source), Path::new(target))?;
    Ok(true)
}

#[allow(non_snake_case)]
pub fn dirMove(source: &str, target: &str) -> Result<bool> {
    if !dirExists(Some(source)) || dirExists(Some(target)) {
        return Ok(false);
    }
    fs::rename(source, target)?;
    Ok(true)
}

#[allow(non_snake_case)]
pub fn dirContents(path: &str) -> Result<(Vec<String>, Vec<String>)> {
    if !dirExists(Some(path)) {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.file_type()?.is_file() {
            files.push(name);
        } else if entry.file_type()?.is_dir() {
            dirs.push(name);
        }
    }
    Ok((files, dirs))
}

#[allow(non_snake_case)]
pub fn dirAllFiles(path: &str, ignore: Option<&BTreeSet<String>>) -> Result<Vec<String>> {
    if fileExists(path) {
        return Ok(vec![path.to_string()]);
    }
    if !dirExists(Some(path)) {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_all_files(Path::new(path), ignore, &mut files)?;
    files.sort();
    Ok(files)
}

#[allow(non_snake_case)]
pub fn dirEmpty(path: &str) -> bool {
    let normalized = normpath(Some(path)).unwrap_or_default();
    !Path::new(&normalized).exists()
        || fs::read_dir(&normalized)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false)
}

#[allow(non_snake_case)]
pub fn getCwd() -> Result<String> {
    Ok(normpath(Some(&env::current_dir()?.to_string_lossy())).unwrap_or_default())
}

#[allow(non_snake_case)]
pub fn chDir(path: &str) -> Result<()> {
    env::set_current_dir(path)?;
    Ok(())
}

#[allow(non_snake_case)]
pub fn readJson(text: Option<&str>, as_file: Option<&str>) -> Result<serde_json::Value> {
    let Some(path) = as_file else {
        return serde_json::from_str(text.unwrap_or("null")).map_err(CfError::from);
    };
    if !fileExists(path) {
        return Ok(serde_json::json!({}));
    }
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(CfError::from)
}

#[allow(non_snake_case)]
pub fn writeJson(data: &serde_json::Value, as_file: Option<&str>) -> Result<Option<String>> {
    let dumped = serde_json::to_string_pretty(data).map_err(CfError::from)?;
    if let Some(path) = as_file {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, &dumped)?;
        Ok(None)
    } else {
        Ok(Some(dumped))
    }
}

#[allow(non_snake_case)]
pub fn readYaml(text: Option<&str>, as_file: Option<&str>) -> Result<serde_yaml::Value> {
    let Some(path) = as_file else {
        return serde_yaml::from_str(text.unwrap_or("null")).map_err(CfError::from);
    };
    if !fileExists(path) {
        return Ok(serde_yaml::Value::Mapping(Default::default()));
    }
    let text = fs::read_to_string(path)?;
    serde_yaml::from_str(&text).map_err(CfError::from)
}

#[allow(non_snake_case)]
pub fn writeYaml(data: &serde_yaml::Value, as_file: Option<&str>) -> Result<Option<String>> {
    let dumped = serde_yaml::to_string(data).map_err(CfError::from)?;
    if let Some(path) = as_file {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, &dumped)?;
        Ok(None)
    } else {
        Ok(Some(dumped))
    }
}

pub fn is_int(value: &str) -> bool {
    value.parse::<i64>().is_ok()
}

#[allow(non_snake_case)]
pub fn isInt(value: &str) -> bool {
    is_int(value)
}

pub fn math_esc(value: Option<&str>) -> String {
    value.unwrap_or("").replace('$', "<span>$</span>")
}

#[allow(non_snake_case)]
pub fn mathEsc(value: Option<&str>) -> String {
    math_esc(value)
}

pub fn md_esc(value: Option<&str>, math: bool) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let escaped = value
        .replace('!', "&#33;")
        .replace('#', "&#35;")
        .replace('*', "&#42;")
        .replace('[', "&#91;")
        .replace('_', "&#95;")
        .replace('|', "&#124;")
        .replace('~', "&#126;");
    if math {
        escaped
    } else {
        escaped.replace('$', "<span>$</span>")
    }
}

#[allow(non_snake_case)]
pub fn mdEsc(value: Option<&str>, math: bool) -> String {
    md_esc(value, math)
}

pub fn html_esc(value: Option<&str>, math: bool) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let escaped = value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    if math {
        escaped
    } else {
        escaped.replace('$', "<span>$</span>")
    }
}

#[allow(non_snake_case)]
pub fn htmlEsc(value: Option<&str>, math: bool) -> String {
    html_esc(value, math)
}

pub fn xml_esc(value: Option<&str>) -> String {
    value
        .unwrap_or("")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

#[allow(non_snake_case)]
pub fn xmlEsc(value: Option<&str>) -> String {
    xml_esc(value)
}

pub fn mdhtml_esc(value: Option<&str>, math: bool) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let escaped = value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;");
    if math {
        escaped
    } else {
        escaped.replace('$', "<span>$</span>")
    }
}

#[allow(non_snake_case)]
pub fn mdhtmlEsc(value: Option<&str>, math: bool) -> String {
    mdhtml_esc(value, math)
}

pub fn tsv_esc(value: &str) -> String {
    if value.starts_with('"') || value.starts_with('\'') {
        format!("\\{value}")
    } else {
        value.to_string()
    }
}

#[allow(non_snake_case)]
pub fn tsvEsc(value: &str) -> String {
    tsv_esc(value)
}

pub fn pandas_esc(value: &str) -> String {
    value.replace('\t', " ").replace('"', "\u{1}\"")
}

#[allow(non_snake_case)]
pub fn pandasEsc(value: &str) -> String {
    pandas_esc(value)
}

pub fn camel(name: Option<&str>) -> Option<String> {
    let name = name?;
    if name.is_empty() {
        return Some(String::new());
    }
    let mut result = String::new();
    for (index, part) in name.split('_').enumerate() {
        if index == 0 {
            result.push_str(part);
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.push_str(chars.as_str());
        }
    }
    Some(result)
}

pub fn clean_name(name: &str) -> String {
    let mut clean = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if clean
        .chars()
        .next()
        .is_none_or(|ch| !ch.is_ascii_alphabetic())
    {
        clean.insert(0, 'x');
    }
    match clean.as_str() {
        "database" => "dbase".to_string(),
        "default" => "dfault".to_string(),
        "first" => "frst".to_string(),
        "focus" => "fcus".to_string(),
        "gap" => "gp".to_string(),
        "last" => "lst".to_string(),
        "notexist" => "notexst".to_string(),
        "object" => "objct".to_string(),
        "retrieve" => "retriev".to_string(),
        "noretrieve" => "noretriev".to_string(),
        "type" => "typ".to_string(),
        "as" => "as_".to_string(),
        "or" => "or_".to_string(),
        _ => clean,
    }
}

#[allow(non_snake_case)]
pub fn cleanName(name: &str) -> String {
    clean_name(name)
}

pub fn is_clean(name: Option<&str>) -> bool {
    let Some(name) = name else {
        return false;
    };
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == '_')
}

#[allow(non_snake_case)]
pub fn isClean(name: Option<&str>) -> bool {
    is_clean(name)
}

pub fn set_from_spec(spec: &str) -> Result<BTreeSet<u32>> {
    let mut covered = BTreeSet::new();
    for part in spec.split(',') {
        if part.is_empty() {
            return Err(CfError::InvalidSpec(spec.to_string()));
        }
        let bounds = part.split('-').collect::<Vec<_>>();
        match bounds.as_slice() {
            [single] => {
                covered.insert(parse_spec_bound(single, spec)?);
            }
            [start, end] => {
                let mut start = parse_spec_bound(start, spec)?;
                let mut end = parse_spec_bound(end, spec)?;
                if end < start {
                    std::mem::swap(&mut start, &mut end);
                }
                covered.extend(start..=end);
            }
            _ => return Err(CfError::InvalidSpec(spec.to_string())),
        }
    }
    Ok(covered)
}

#[allow(non_snake_case)]
pub fn setFromSpec(spec: &str) -> Result<BTreeSet<u32>> {
    set_from_spec(spec)
}

pub fn ranges_from_set(nodes: &BTreeSet<u32>) -> Vec<(u32, u32)> {
    ranges_from_sorted_iter(nodes.iter().copied())
}

#[allow(non_snake_case)]
pub fn rangesFromSet(nodes: &BTreeSet<u32>) -> Vec<(u32, u32)> {
    ranges_from_set(nodes)
}

pub fn ranges_from_list(nodes: &[u32]) -> Vec<(u32, u32)> {
    ranges_from_sorted_iter(nodes.iter().copied())
}

#[allow(non_snake_case)]
pub fn rangesFromList(nodes: &[u32]) -> Vec<(u32, u32)> {
    ranges_from_list(nodes)
}

pub fn spec_from_ranges(ranges: &[(u32, u32)]) -> String {
    ranges
        .iter()
        .map(|(start, end)| {
            if start == end {
                start.to_string()
            } else {
                format!("{start}-{end}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[allow(non_snake_case)]
pub fn specFromRanges(ranges: &[(u32, u32)]) -> String {
    spec_from_ranges(ranges)
}

pub fn spec_from_ranges_logical(ranges: &[(u32, u32)]) -> Vec<LogicalRange> {
    ranges
        .iter()
        .map(|(start, end)| {
            if start == end {
                LogicalRange::Single(*start)
            } else {
                LogicalRange::Range(*start, *end)
            }
        })
        .collect()
}

#[allow(non_snake_case)]
pub fn specFromRangesLogical(ranges: &[(u32, u32)]) -> Vec<LogicalRange> {
    spec_from_ranges_logical(ranges)
}

pub fn make_index<K, V>(data: &BTreeMap<K, V>) -> BTreeMap<V, BTreeSet<K>>
where
    K: Copy + Ord,
    V: Clone + Ord,
{
    let mut inverse: BTreeMap<V, BTreeSet<K>> = BTreeMap::new();
    for (node, value) in data {
        inverse.entry(value.clone()).or_default().insert(*node);
    }
    inverse
}

#[allow(non_snake_case)]
pub fn makeIndex<K, V>(data: &BTreeMap<K, V>) -> BTreeMap<V, BTreeSet<K>>
where
    K: Copy + Ord,
    V: Clone + Ord,
{
    make_index(data)
}

pub fn make_inverse<K, V>(data: &BTreeMap<K, BTreeSet<V>>) -> BTreeMap<V, BTreeSet<K>>
where
    K: Copy + Ord,
    V: Copy + Ord,
{
    let mut inverse: BTreeMap<V, BTreeSet<K>> = BTreeMap::new();
    for (source, targets) in data {
        for target in targets {
            inverse.entry(*target).or_default().insert(*source);
        }
    }
    inverse
}

#[allow(non_snake_case)]
pub fn makeInverse<K, V>(data: &BTreeMap<K, BTreeSet<V>>) -> BTreeMap<V, BTreeSet<K>>
where
    K: Copy + Ord,
    V: Copy + Ord,
{
    make_inverse(data)
}

pub fn make_inverse_val<K, V, T>(data: &BTreeMap<K, BTreeMap<V, T>>) -> BTreeMap<V, BTreeMap<K, T>>
where
    K: Copy + Ord,
    V: Copy + Ord,
    T: Clone,
{
    let mut inverse: BTreeMap<V, BTreeMap<K, T>> = BTreeMap::new();
    for (source, targets) in data {
        for (target, value) in targets {
            inverse
                .entry(*target)
                .or_default()
                .insert(*source, value.clone());
        }
    }
    inverse
}

#[allow(non_snake_case)]
pub fn makeInverseVal<K, V, T>(data: &BTreeMap<K, BTreeMap<V, T>>) -> BTreeMap<V, BTreeMap<K, T>>
where
    K: Copy + Ord,
    V: Copy + Ord,
    T: Clone,
{
    make_inverse_val(data)
}

fn parse_spec_bound(bound: &str, spec: &str) -> Result<u32> {
    bound
        .parse::<u32>()
        .map_err(|_| CfError::InvalidSpec(spec.to_string()))
}

fn ranges_from_sorted_iter(nodes: impl IntoIterator<Item = u32>) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    let mut current: Option<(u32, u32)> = None;
    for node in nodes {
        match current {
            None => current = Some((node, node)),
            Some((start, end)) if node == end + 1 => current = Some((start, node)),
            Some(range) => {
                ranges.push(range);
                current = Some((node, node));
            }
        }
    }
    if let Some(range) = current {
        ranges.push(range);
    }
    ranges
}

fn split_helper_items(value: &str) -> Vec<String> {
    value
        .trim_matches(|ch| matches!(ch, '\n' | '\t' | ' ' | ','))
        .split(|ch| matches!(ch, '\n' | '\t' | ' ' | ','))
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn home_dir() -> String {
    env::var("HOME")
        .map(|home| normalize_path_string(&home))
        .unwrap_or_else(|_| "~".to_string())
}

fn normalize_path_string(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    let path = Path::new(&replaced);
    let mut parts = Vec::new();
    let mut prefix = String::new();
    let mut absolute = false;
    for component in path.components() {
        match component {
            Component::Prefix(raw_prefix) => {
                prefix = raw_prefix.as_os_str().to_string_lossy().replace('\\', "/");
            }
            Component::RootDir => absolute = true,
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.last().is_some_and(|part| part != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..".to_string());
                }
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
        }
    }
    let mut result = String::new();
    if !prefix.is_empty() {
        result.push_str(&prefix);
        if absolute {
            result.push('/');
        }
    } else if absolute {
        result.push('/');
    }
    result.push_str(&parts.join("/"));
    if result.is_empty() {
        ".".to_string()
    } else {
        result
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if entry.file_type()?.is_file() {
            fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}

fn collect_all_files(
    path: &Path,
    ignore: Option<&BTreeSet<String>>,
    files: &mut Vec<String>,
) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry.file_type()?.is_file() {
            files.push(normalize_path_string(&entry_path.to_string_lossy()));
        } else if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if ignore.is_some_and(|ignored| ignored.contains(&name)) {
                continue;
            }
            collect_all_files(&entry_path, ignore, files)?;
        }
    }
    Ok(())
}
