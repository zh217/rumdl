use crate::rule::{LintResult, LintWarning, Rule, Severity};
use crate::rule_config_serde::RuleConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct MD902Config {
    /// Maximum number of words allowed in a paragraph before requiring a footnote
    #[serde(default = "default_max_words")]
    pub max_words: usize,
    /// Whether to ignore paragraphs inside blockquotes
    #[serde(default = "default_false")]
    pub ignore_blockquotes: bool,
}

fn default_max_words() -> usize {
    200
}

fn default_false() -> bool {
    false
}

impl Default for MD902Config {
    fn default() -> Self {
        Self {
            max_words: 200,
            ignore_blockquotes: false,
        }
    }
}

impl RuleConfig for MD902Config {
    const RULE_NAME: &'static str = "MD902";
}

#[derive(Clone, Default)]
pub struct MD902LongParagraphFootnotes {
    config: MD902Config,
}

impl MD902LongParagraphFootnotes {
    pub fn new() -> Self {
        Self {
            config: MD902Config::default(),
        }
    }

    pub fn from_config_struct(config: MD902Config) -> Self {
        Self { config }
    }

    fn count_words(text: &str) -> usize {
        text.split_whitespace().count()
    }

    fn has_footnote_reference(text: &str) -> bool {
        // Simple check for [^...] pattern
        // We don't need full parser accuracy here, just a heuristic
        text.contains("[^")
    }
}

impl Rule for MD902LongParagraphFootnotes {
    fn name(&self) -> &'static str {
        "MD902"
    }

    fn description(&self) -> &'static str {
        "Long paragraphs should have footnotes"
    }

    fn check(&self, ctx: &crate::lint_context::LintContext) -> LintResult {
        let mut warnings = Vec::new();
        let mut current_paragraph = String::new();
        let mut paragraph_start_line = 0;
        let mut paragraph_end_line = 0;

        for (i, line_info) in ctx.lines.iter().enumerate() {
            let line_num = i + 1;

            // Determine if this line breaks a paragraph
            let is_break = line_info.is_blank
                || line_info.in_code_block
                || line_info.in_front_matter
                || line_info.in_html_block
                || line_info.heading.is_some()
                || line_info.list_item.is_some()
                || (self.config.ignore_blockquotes && line_info.blockquote.is_some());

            if is_break {
                // Process the accumulated paragraph if any
                if !current_paragraph.is_empty() {
                    let word_count = Self::count_words(&current_paragraph);
                    if word_count > self.config.max_words && !Self::has_footnote_reference(&current_paragraph) {
                        warnings.push(LintWarning {
                            message: format!(
                                "Paragraph has {} words (limit: {}) but no footnote reference",
                                word_count, self.config.max_words
                            ),
                            line: paragraph_start_line,
                            column: 1,
                            end_line: paragraph_end_line,
                            end_column: ctx.lines[paragraph_end_line - 1].content(ctx.content).chars().count() + 1,
                            severity: Severity::Warning,
                            fix: None,
                            rule_name: Some(self.name().to_string()),
                        });
                    }
                    current_paragraph.clear();
                }
            } else {
                // Accumulate line content
                if current_paragraph.is_empty() {
                    paragraph_start_line = line_num;
                }
                paragraph_end_line = line_num;

                let content = line_info.content(ctx.content).trim();
                if !current_paragraph.is_empty() {
                    current_paragraph.push(' ');
                }
                current_paragraph.push_str(content);
            }
        }

        // Check last paragraph
        if !current_paragraph.is_empty() {
            let word_count = Self::count_words(&current_paragraph);
            if word_count > self.config.max_words && !Self::has_footnote_reference(&current_paragraph) {
                warnings.push(LintWarning {
                    message: format!(
                        "Paragraph has {} words (limit: {}) but no footnote reference",
                        word_count, self.config.max_words
                    ),
                    line: paragraph_start_line,
                    column: 1,
                    end_line: paragraph_end_line,
                    end_column: ctx.lines[paragraph_end_line - 1].content(ctx.content).chars().count() + 1,
                    severity: Severity::Warning,
                    fix: None,
                    rule_name: Some(self.name().to_string()),
                });
            }
        }

        Ok(warnings)
    }

    fn fix(&self, ctx: &crate::lint_context::LintContext) -> Result<String, crate::rule::LintError> {
        // Cannot auto-fix missing citations.
        Ok(ctx.content.to_string())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn from_config(config: &crate::config::Config) -> Box<dyn Rule>
    where
        Self: Sized,
    {
        let rule_config = config
            .rules
            .get(MD902Config::RULE_NAME)
            .and_then(|rc| serde_json::to_value(&rc.values).ok())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        Box::new(Self::from_config_struct(rule_config))
    }
}
