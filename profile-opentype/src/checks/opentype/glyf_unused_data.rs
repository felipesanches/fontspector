use fontations::skrifa::{raw::TableProvider, Tag};
use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert, Metadata};
use serde_json::json;
use std::cmp::Ordering;

#[check(
    id="opentype/glyf_unused_data",
    rationale="
        This check validates the structural integrity of the glyf table,
        by checking that all glyphs referenced in the loca table are
        actually present in the glyf table and that there is no unused
        data at the end of the glyf table. A failure here indicates a
        problem with the font compiler.
    ",
    proposal="https://github.com/fonttools/fontbakery/issues/4829",  // legacy check
    title="Is there any unused data at the end of the glyf table?"
)]
fn glyf_unused_data(t: &Testable, _context: &Context) -> CheckFnResult {
    let ttf = testfont!(t);
    let glyf = ttf
        .font()
        .table_data(Tag::new(b"glyf"))
        .ok_or(FontspectorError::skip("no-glyf", "No glyf table"))?;
    let loca = ttf
        .font()
        .loca(None)
        .map_err(|_| FontspectorError::General("No loca table".to_string()))?;
    let mut problems = vec![];
    if let Some(last_index) = loca.get_raw(loca.len()) {
        match glyf.len().cmp(&(last_index as usize)) {
            Ordering::Greater => {
                let msg = "Unused data at the end of the glyf table";
                let mut status = Status::fail("unreachable-data", msg);
                status.add_metadata(Metadata::TableProblem {
                    table_tag: "glyf".to_string(),
                    field_name: None,
                    actual: Some(json!(glyf.len())),
                    expected: Some(json!(last_index as usize)),
                    message: msg.to_string(),
                });
                problems.push(status);
            }
            Ordering::Less => {
                let msg = "Missing data at the end of the glyf table";
                let mut status = Status::fail("missing-data", msg);
                status.add_metadata(Metadata::TableProblem {
                    table_tag: "glyf".to_string(),
                    field_name: None,
                    actual: Some(json!(glyf.len())),
                    expected: Some(json!(last_index as usize)),
                    message: msg.to_string(),
                });
                problems.push(status);
            }
            Ordering::Equal => {
                // Pass
            }
        }
        return_result(problems)
    } else {
        Err(FontspectorError::General("Invalid loca table".to_string()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    use super::glyf_unused_data;

    #[test]
    fn test_pass_good_font() {
        let testable = test_able("nunito/Nunito-Regular.ttf");
        let result = run_check(glyf_unused_data, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_fail_extra_data_at_end() {
        // Extend the glyf table with extra bytes beyond what loca references
        let mut testable = test_able("nunito/Nunito-Regular.ttf");
        use fontations::skrifa::{font::FontRef, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let glyf_data = f.table_data(Tag::new(b"glyf")).unwrap();

        // Add extra bytes at the end of glyf
        let mut new_glyf = glyf_data.as_bytes().to_vec();
        new_glyf.extend_from_slice(&[0u8; 100]);

        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag == Tag::new(b"glyf") {
                builder.add_raw(tag, &new_glyf);
            } else if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        testable.contents = builder.build();
        let result = run_check(glyf_unused_data, testable);
        assert_results_contain(
            &result,
            StatusCode::Fail,
            Some("unreachable-data".to_string()),
        );
    }

    #[test]
    fn test_fail_truncated_data() {
        // Truncate the glyf table so it's shorter than what loca references
        let mut testable = test_able("nunito/Nunito-Regular.ttf");
        use fontations::skrifa::{font::FontRef, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let glyf_data = f.table_data(Tag::new(b"glyf")).unwrap();

        // Truncate glyf by 100 bytes
        let glyf_bytes = glyf_data.as_bytes();
        let new_len = glyf_bytes.len().saturating_sub(100);
        let new_glyf = &glyf_bytes[..new_len];

        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag == Tag::new(b"glyf") {
                builder.add_raw(tag, new_glyf);
            } else if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        testable.contents = builder.build();
        let result = run_check(glyf_unused_data, testable);
        assert_results_contain(&result, StatusCode::Fail, Some("missing-data".to_string()));
    }
}
