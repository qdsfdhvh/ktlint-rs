use anyhow::{bail, Result};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
    pub owner: &'static str,
    pub range: Range<usize>,
    pub replacement: String,
}

impl TextEdit {
    pub(crate) fn new(
        owner: &'static str,
        range: Range<usize>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            owner,
            range,
            replacement: replacement.into(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct EditSet {
    edits: Vec<TextEdit>,
}

impl EditSet {
    pub(crate) fn new(edits: Vec<TextEdit>) -> Self {
        Self { edits }
    }

    pub(crate) fn apply(mut self, source: &str) -> Result<String> {
        self.edits
            .sort_by_key(|edit| (edit.range.start, edit.range.end));
        let mut previous_end = 0usize;
        let mut previous_start = None;
        for (index, edit) in self.edits.iter().enumerate() {
            if edit.owner.is_empty() {
                bail!("formatter edit has no rule/pass owner");
            }
            if edit.range.start > edit.range.end || edit.range.end > source.len() {
                bail!("{} produced an out-of-bounds edit", edit.owner);
            }
            if !source.is_char_boundary(edit.range.start)
                || !source.is_char_boundary(edit.range.end)
            {
                bail!("{} produced an edit outside UTF-8 boundaries", edit.owner);
            }
            if index > 0
                && (edit.range.start < previous_end || previous_start == Some(edit.range.start))
            {
                let previous = &self.edits[index - 1];
                bail!(
                    "overlapping formatter edits from {} and {}",
                    previous.owner,
                    edit.owner
                );
            }
            previous_end = edit.range.end;
            previous_start = Some(edit.range.start);
        }

        let mut output = source.to_string();
        for edit in self.edits.into_iter().rev() {
            output.replace_range(edit.range, &edit.replacement);
        }
        Ok(output)
    }
}

/// Convert a deterministic whole-pass transformation into its smallest single edit.
/// Future CST rules can return multiple edits directly through [`EditSet`].
pub(super) fn minimal_edit(
    owner: &'static str,
    source: &str,
    transformed: &str,
) -> Option<TextEdit> {
    if source == transformed {
        return None;
    }

    let mut prefix = 0usize;
    for ((left_offset, left), (right_offset, right)) in
        source.char_indices().zip(transformed.char_indices())
    {
        if left != right {
            break;
        }
        prefix = left_offset + left.len_utf8();
        debug_assert_eq!(prefix, right_offset + right.len_utf8());
    }

    let source_tail = &source[prefix..];
    let transformed_tail = &transformed[prefix..];
    let mut suffix = 0usize;
    for (left, right) in source_tail
        .chars()
        .rev()
        .zip(transformed_tail.chars().rev())
    {
        if left != right {
            break;
        }
        let width = left.len_utf8();
        if suffix + width > source_tail.len() || suffix + width > transformed_tail.len() {
            break;
        }
        suffix += width;
    }

    Some(TextEdit::new(
        owner,
        prefix..source.len() - suffix,
        &transformed[prefix..transformed.len() - suffix],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_non_overlapping_edits_from_right_to_left() {
        let edits = EditSet::new(vec![
            TextEdit::new("left", 0..1, "A"),
            TextEdit::new("right", 2..3, "C"),
        ]);
        assert_eq!(edits.apply("abc").unwrap(), "AbC");
    }

    #[test]
    fn rejects_overlapping_edits() {
        let edits = EditSet::new(vec![
            TextEdit::new("first", 0..2, "x"),
            TextEdit::new("second", 1..3, "y"),
        ]);
        assert!(edits
            .apply("abc")
            .unwrap_err()
            .to_string()
            .contains("overlapping"));
    }

    #[test]
    fn rejects_multiple_edits_at_the_same_boundary() {
        let edits = EditSet::new(vec![
            TextEdit::new("first", 1..1, "x"),
            TextEdit::new("second", 1..1, "y"),
        ]);
        assert!(edits
            .apply("ab")
            .unwrap_err()
            .to_string()
            .contains("overlapping"));
    }

    #[test]
    fn rejects_non_utf8_boundary() {
        let edits = EditSet::new(vec![TextEdit::new("bad", 1..2, "x")]);
        assert!(edits.apply("é").is_err());
    }

    #[test]
    fn minimal_edit_preserves_unchanged_unicode_prefix_and_suffix() {
        let edit = minimal_edit("rule", "α before ω", "α after ω").unwrap();
        assert_eq!(
            EditSet::new(vec![edit]).apply("α before ω").unwrap(),
            "α after ω"
        );
    }
}
