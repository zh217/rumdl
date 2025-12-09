use crate::rule::{LintResult, LintWarning, Rule, Severity};
use crate::rule_config_serde::RuleConfig;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static FOOTNOTE_DEF_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*)\[\^([a-zA-Z0-9_-]+)\]:\s*").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct MD063Config {
    /// Check for duplicate footnote definitions (always an error in most parsers)
    #[serde(default = "default_true")]
    pub check_definitions: bool,
    /// Check for duplicate footnote references (allowed by some parsers, but often a mistake)
    #[serde(default = "default_false")]
    pub check_references: bool,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

impl Default for MD063Config {
    fn default() -> Self {
        Self {
            check_definitions: true,
            check_references: false,
        }
    }
}

impl RuleConfig for MD063Config {
    const RULE_NAME: &'static str = "MD063";
}

#[derive(Clone, Default)]
pub struct MD063DuplicateFootnotes {
    config: MD063Config,
}

impl MD063DuplicateFootnotes {
    pub fn new() -> Self {
        Self {
            config: MD063Config::default(),
        }
    }

    pub fn from_config_struct(config: MD063Config) -> Self {
        Self { config }
    }
}

impl Rule for MD063DuplicateFootnotes {
    fn name(&self) -> &'static str {
        "MD063"
    }

    fn description(&self) -> &'static str {
        "Footnotes should not be duplicated"
    }

    fn check(&self, ctx: &crate::lint_context::LintContext) -> LintResult {
        let mut warnings = Vec::new();

        // Check for duplicate definitions
        if self.config.check_definitions {
            let mut seen_definitions: HashMap<String, usize> = HashMap::new();

            for (line_idx, line_info) in ctx.lines.iter().enumerate() {
                if line_info.in_code_block || line_info.in_front_matter {
                    continue;
                }

                let content = line_info.content(ctx.content);
                if let Some(cap) = FOOTNOTE_DEF_REGEX.captures(content) {
                    if let Some(id_match) = cap.get(2) {
                        let id = id_match.as_str();
                        if let Some(&first_line) = seen_definitions.get(id) {
                            warnings.push(LintWarning {
                                message: format!(
                                    "Duplicate footnote definition '[^{}]' (first defined on line {})",
                                    id,
                                    first_line + 1
                                ),
                                line: line_idx + 1,
                                column: line_info.indent + 1,
                                end_line: line_idx + 1,
                                end_column: line_info.indent + 1 + cap.get(0).unwrap().len(),
                                severity: Severity::Error,
                                fix: None,
                                rule_name: Some(self.name().to_string()),
                            });
                        } else {
                            seen_definitions.insert(id.to_string(), line_idx);
                        }
                    }
                }
            }
        }

        // Check for duplicate references
        if self.config.check_references {
            let mut seen_references: HashSet<String> = HashSet::new();

            for footnote_ref in &ctx.footnote_refs {
                // Skip if in code block (already handled by parser usually, but good to be safe)
                if ctx.line_info(footnote_ref.line).is_some_and(|l| l.in_code_block) {
                    continue;
                }

                if seen_references.contains(&footnote_ref.id) {
                    // Calculate column from byte offset
                    let (line, col) = ctx.offset_to_line_col(footnote_ref.byte_offset);
                    let end_col = col + (footnote_ref.byte_end - footnote_ref.byte_offset);

                    warnings.push(LintWarning {
                        message: format!("Duplicate footnote reference '[^{}]'", footnote_ref.id),
                        line,
                        column: col,
                        end_line: line,
                        end_column: end_col,
                        severity: Severity::Warning,
                        fix: None,
                        rule_name: Some(self.name().to_string()),
                    });
                } else {
                    seen_references.insert(footnote_ref.id.clone());
                }
            }
        }

        Ok(warnings)
    }

    fn from_config(config: &crate::config::Config) -> Box<dyn Rule>
    where
        Self: Sized,
    {
        let rule_config = crate::rule_config_serde::load_rule_config::<MD063Config>(config);
        Box::new(Self::from_config_struct(rule_config))
    }

    fn default_config_section(&self) -> Option<(String, toml::Value)> {
        let default_config = MD063Config::default();
        let json_value = serde_json::to_value(&default_config).ok()?;
        let toml_value = crate::rule_config_serde::json_to_toml_value(&json_value)?;
        
        if let toml::Value::Table(table) = toml_value {
            if !table.is_empty() {
                Some((MD063Config::RULE_NAME.to_string(), toml::Value::Table(table)))
            } else {
                None
            }
        } else {
            None
        }
    }
}
