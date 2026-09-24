//! User-facing tape references (`#57`) mapped to private Sage lookups.
//!
//! Row numbers are derived from the visible tape order (1-based), but only the
//! most recent successful, reusable expressions are valid references. The
//! kernel stores those values in a private dictionary (`__casette_tape_refs`) so
//! that a prompt like `plot #1, x=-10..10` or `factor #57` compiles to Sage that
//! evaluates against the stored expression without Python comment collisions.

use crate::error::CompileError;
use std::collections::HashMap;

/// A table of valid tape references mapping 1-based row numbers to their reusable Sage expressions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TapeReferenceTable {
    entries: HashMap<usize, String>,
}

impl TapeReferenceTable {
    /// Maximum number of recent successful tape references retained for expansion.
    pub const CAPACITY: usize = 20;

    /// The private Python dictionary identifier used in Sage for tape references.
    pub const VARIABLE_NAME: &'static str = "__casette_tape_refs";

    /// Creates an empty tape reference table.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Creates a tape reference table with explicit entries.
    #[must_use]
    pub fn new(entries: HashMap<usize, String>) -> Self {
        Self { entries }
    }

    /// Returns a reference to the underlying entries map.
    #[must_use]
    pub fn entries(&self) -> &HashMap<usize, String> {
        &self.entries
    }

    /// Inserts or updates an entry for `row_number` and ensures the table does not exceed [`CAPACITY`].
    pub fn insert(&mut self, row_number: usize, expression: String) {
        self.entries.insert(row_number, expression);
        if self.entries.len() > Self::CAPACITY {
            let mut keys: Vec<usize> = self.entries.keys().copied().collect();
            keys.sort_unstable();
            let excess = keys.len() - Self::CAPACITY;
            for key in keys.iter().take(excess) {
                self.entries.remove(key);
            }
        }
    }

    /// Clears all entries from the table.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Expands `#ROW` references in `input` into `__casette_tape_refs[ROW]`.
    ///
    /// References inside single or double quotes are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`CompileError`] if a `#ROW` token is encountered for a row that is
    /// not present in this reference table.
    pub fn expand_references(&self, input: &str) -> Result<String, CompileError> {
        let mut output = String::with_capacity(input.len());
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        let mut quoted_by: Option<char> = None;
        let mut escaped = false;

        while i < chars.len() {
            let ch = chars[i];

            if let Some(quote) = quoted_by {
                output.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == quote {
                    quoted_by = None;
                }
                i += 1;
                continue;
            }

            if ch == '"' || ch == '\'' {
                quoted_by = Some(ch);
                output.push(ch);
                i += 1;
                continue;
            }

            if ch == '#' {
                let digits_start = i + 1;
                let mut digits_end = digits_start;
                while digits_end < chars.len() && chars[digits_end].is_ascii_digit() {
                    digits_end += 1;
                }

                if digits_start < digits_end {
                    let digits_str: String = chars[digits_start..digits_end].iter().collect();
                    if let Ok(row_num) = digits_str.parse::<usize>() {
                        if !self.entries.contains_key(&row_num) {
                            return Err(CompileError::new(
                                format!("`#{digits_str}` is not available on the tape."),
                                Some(i),
                                Some(format!(
                                    "Use one of the last {} successful tape entries.",
                                    Self::CAPACITY
                                )),
                            ));
                        }
                        output.push_str(&format!("{}[{}]", Self::VARIABLE_NAME, row_num));
                        i = digits_end;
                        continue;
                    }
                }
            }

            output.push(ch);
            i += 1;
        }

        Ok(output)
    }

    /// Generates the Python assignment snippet for `row_number` to register its expression
    /// into Sage's global `__casette_tape_refs` dictionary and prune retired rows.
    #[must_use]
    pub fn assignment_code(&self, row_number: usize) -> Option<String> {
        let expr = self.entries.get(&row_number)?;
        let mut retained_keys: Vec<usize> = self.entries.keys().copied().collect();
        retained_keys.sort_unstable();
        let retained_str = retained_keys
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let var = Self::VARIABLE_NAME;
        Some(format!(
            "{var} = globals().get('{var}', {{}})\n\
             {var}[{row_number}] = ({expr})\n\
             for __casette_tape_key in list({var}):\n    \
                 if __casette_tape_key not in {{{retained_str}}}:\n        \
                     del {var}[__casette_tape_key]\n\
             del __casette_tape_key\n"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_valid_references() {
        let mut table = TapeReferenceTable::empty();
        table.insert(1, "sin(x)".into());
        table.insert(2, "cos(x)".into());

        assert_eq!(
            table.expand_references("plot #1, x=-10..10").unwrap(),
            "plot __casette_tape_refs[1], x=-10..10"
        );
        assert_eq!(
            table.expand_references("#1 + #2").unwrap(),
            "__casette_tape_refs[1] + __casette_tape_refs[2]"
        );
    }

    #[test]
    fn test_quotes_preserve_hash() {
        let mut table = TapeReferenceTable::empty();
        table.insert(1, "sin(x)".into());

        assert_eq!(
            table.expand_references("print(\"#1\")").unwrap(),
            "print(\"#1\")"
        );
        assert_eq!(
            table.expand_references("'#1' + #1").unwrap(),
            "'#1' + __casette_tape_refs[1]"
        );
    }

    #[test]
    fn test_missing_reference_errors() {
        let table = TapeReferenceTable::empty();
        let err = table.expand_references("#99 + 1").unwrap_err();
        assert!(err.message.contains("#99"));
        assert!(err.suggestion.unwrap().contains("20"));
    }

    #[test]
    fn test_capacity_pruning() {
        let mut table = TapeReferenceTable::empty();
        for i in 1..=25 {
            table.insert(i, format!("{i}"));
        }
        assert_eq!(table.entries().len(), 20);
        assert!(!table.entries().contains_key(&1));
        assert!(table.entries().contains_key(&6));
        assert!(table.entries().contains_key(&25));
    }
}
