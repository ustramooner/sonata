//! Per-voice text rules: ordered regex replacements applied to the text before it is phonemized.
//!
//! Some languages need words rearranged before espeak-ng sees them (Tetun reads "5mm" as "milímetru 5", and writes times and dates in a
//! different order from how it says them). espeak-ng cannot reorder words, so a voice can ship a rules file next to its model:
//!
//! ```json
//! { "tables": { "months": { "3": "Marsu" } },
//!   "rules":  [ { "pattern": "(\\d+)/0?(\\d+)", "replace": "loron $1, fulan {{2|months}}", "ignore_case": false } ] }
//! ```
//!
//! * rules run in order; each one replaces every match of `pattern` (fancy-regex syntax, so lookaround and backreferences work)
//! * in `replace`, `$1`..`$9` insert a capture group and `{{N|table}}` inserts the value of table `table` for the text of group N
//!   (tried as written, then with leading zeros removed, then lowercased). When the key is missing the match is left unchanged, so a
//!   lookup also works as a validity check.

use fancy_regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct RulesFile {
    #[serde(default)]
    tables: HashMap<String, HashMap<String, String>>,
    rules: Vec<RuleSpec>,
}

#[derive(Deserialize)]
struct RuleSpec {
    pattern: String,
    replace: String,
    #[serde(default)]
    ignore_case: bool,
}

enum Part {
    Text(String),
    Group(usize),
    Lookup(usize, String),
}

struct Rule {
    regex: Regex,
    replacement: Vec<Part>,
}

pub struct TextRules {
    tables: HashMap<String, HashMap<String, String>>,
    rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct RulesError(pub String);

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RulesError {}

fn parse_replacement(template: &str, tables: &HashMap<String, HashMap<String, String>>) -> Result<Vec<Part>, RulesError> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        if let Some(inner) = rest.strip_prefix("{{") {
            let end = inner.find("}}").ok_or_else(|| RulesError(format!("unclosed {{{{ in replacement: {template}")))?;
            let (group, table) = inner[..end]
                .split_once('|')
                .ok_or_else(|| RulesError(format!("lookup needs N|table: {{{{{}}}}}", &inner[..end])))?;
            let group: usize = group.trim().parse().map_err(|_| RulesError(format!("bad group number in {{{{{}}}}}", &inner[..end])))?;
            let table = table.trim().to_string();
            if !tables.contains_key(&table) {
                return Err(RulesError(format!("unknown table {table}")));
            }
            if !text.is_empty() {
                parts.push(Part::Text(std::mem::take(&mut text)));
            }
            parts.push(Part::Lookup(group, table));
            rest = &inner[end + 2..];
        } else if let Some(after) = rest.strip_prefix('$') {
            match after.chars().next().and_then(|c| c.to_digit(10)) {
                Some(n) => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    parts.push(Part::Group(n as usize));
                    rest = &after[1..];
                }
                None => {
                    text.push('$');
                    rest = after;
                }
            }
        } else {
            let c = rest.chars().next().unwrap();
            text.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    Ok(parts)
}

impl TextRules {
    pub fn from_json(json: &str) -> Result<Self, RulesError> {
        let file: RulesFile = serde_json::from_str(json).map_err(|e| RulesError(format!("invalid text rules: {e}")))?;
        let mut rules = Vec::with_capacity(file.rules.len());
        for spec in &file.rules {
            let pattern = if spec.ignore_case { format!("(?i){}", spec.pattern) } else { spec.pattern.clone() };
            let regex = Regex::new(&pattern).map_err(|e| RulesError(format!("bad pattern {}: {e}", spec.pattern)))?;
            rules.push(Rule { regex, replacement: parse_replacement(&spec.replace, &file.tables)? });
        }
        Ok(Self { tables: file.tables, rules })
    }

    pub fn from_path(path: &Path) -> Result<Self, RulesError> {
        let json = std::fs::read_to_string(path).map_err(|e| RulesError(format!("cannot read {}: {e}", path.display())))?;
        Self::from_json(&json)
    }

    /// The rules file that belongs to a Piper voice config: `name.onnx.json` -> `name.textrules.json`, or `<config>.textrules.json`.
    pub fn path_for_config(config_path: &Path) -> Vec<PathBuf> {
        let name = config_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut found = Vec::new();
        if let Some(stem) = name.strip_suffix(".onnx.json") {
            found.push(config_path.with_file_name(format!("{stem}.textrules.json")));
        }
        found.push(config_path.with_file_name(format!("{name}.textrules.json")));
        found
    }

    /// Load the rules next to a voice config, if the voice has any. A broken rules file is an error; a missing one is not.
    pub fn for_config(config_path: &Path) -> Result<Option<Self>, RulesError> {
        for path in Self::path_for_config(config_path) {
            if path.exists() {
                return Self::from_path(&path).map(Some);
            }
        }
        Ok(None)
    }

    fn lookup(&self, table: &str, key: &str) -> Option<&String> {
        let t = self.tables.get(table)?;
        t.get(key)
            .or_else(|| t.get(key.trim_start_matches('0')))
            .or_else(|| t.get(&key.to_lowercase()))
    }

    pub fn apply(&self, text: &str) -> String {
        let mut text = text.to_string();
        for rule in &self.rules {
            let mut out = String::with_capacity(text.len());
            let mut last = 0;
            for caps in rule.regex.captures_iter(&text).flatten() {
                let whole = caps.get(0).unwrap();
                let mut replaced = String::new();
                let mut valid = true;
                for part in &rule.replacement {
                    match part {
                        Part::Text(t) => replaced.push_str(t),
                        Part::Group(n) => replaced.push_str(caps.get(*n).map_or("", |m| m.as_str())),
                        Part::Lookup(n, table) => match caps.get(*n).and_then(|m| self.lookup(table, m.as_str())) {
                            Some(v) => replaced.push_str(v),
                            None => {
                                valid = false;
                                break;
                            }
                        },
                    }
                }
                out.push_str(&text[last..whole.start()]);
                out.push_str(if valid { &replaced } else { whole.as_str() });
                last = whole.end();
            }
            out.push_str(&text[last..]);
            text = out;
        }
        text
    }
}
