use std::collections::HashSet;

use fontations::skrifa::{raw::TableProvider, MetadataProvider};
use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert};

#[check(
    id = "opentype/varfont/STAT_axis_record_for_each_axis",
    rationale = "
        According to the OpenType spec, there must be an Axis Record
        for every axis defined in the fvar table.

        https://docs.microsoft.com/en-us/typography/opentype/spec/stat#axis-records
    ",
    title = "All fvar axes have a correspondent Axis Record on STAT table?",
    proposal = "https://github.com/fonttools/fontbakery/pull/3017"
)]
fn STAT_axis_record_for_each_axis(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    skip!(!f.is_variable_font(), "not-variable", "Not a variable font");
    let fvar_axis_tags: HashSet<_> = f
        .font()
        .axes()
        .iter()
        .map(|axis| axis.tag().to_string())
        .collect();
    let stat_axis_tags: HashSet<_> = f
        .font()
        .stat()
        .map_err(|_| FontspectorError::skip("no-stat", "No STAT table"))?
        .design_axes()?
        .iter()
        .map(|axis_record| axis_record.axis_tag().to_string())
        .collect();
    let missing_axes: Vec<&str> = fvar_axis_tags
        .difference(&stat_axis_tags)
        .map(|x| x.as_ref())
        .collect();
    Ok(if missing_axes.is_empty() {
        Status::just_one_pass()
    } else {
        Status::just_one_fail(
            "missing-axis-records",
            &format!(
                "STAT table is missing Axis Records for the following axes:\n\n{}",
                bullet_list(context, &missing_axes)
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, assert_skip, run_check, test_able},
        StatusCode,
    };

    use super::STAT_axis_record_for_each_axis;

    #[test]
    fn test_pass_good_font() {
        let testable = test_able("cabinvf/Cabin[wdth,wght].ttf");
        let result = run_check(STAT_axis_record_for_each_axis, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_fail_missing_axis_records() {
        let mut testable = test_able("cabinvf/Cabin[wdth,wght].ttf");
        use fontations::skrifa::{font::FontRef, raw::TableProvider, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let stat_data = f.table_data(Tag::new(b"STAT")).unwrap();
        let mut new_stat = stat_data.as_bytes().to_vec();

        // STAT has axes: wdth(0), wght(1), ital(2). fvar has wght and wdth.
        // To make the check fail, overwrite the wght axis tag with something
        // unrecognized so STAT no longer covers fvar's wght axis.
        // Each axis record is designAxisSize bytes. Find the offset.
        let axis_size = u16::from_be_bytes([new_stat[4], new_stat[5]]) as usize;
        let axis_offset =
            u32::from_be_bytes([new_stat[8], new_stat[9], new_stat[10], new_stat[11]]) as usize;
        // Axis 1 (wght) starts at axis_offset + axis_size * 1
        // First 4 bytes of an axis record are the tag
        let wght_tag_offset = axis_offset + axis_size;
        new_stat[wght_tag_offset..wght_tag_offset + 4].copy_from_slice(b"XXXX");

        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag == Tag::new(b"STAT") {
                builder.add_raw(tag, &new_stat);
            } else if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        testable.contents = builder.build();
        let result = run_check(STAT_axis_record_for_each_axis, testable);
        assert_results_contain(
            &result,
            StatusCode::Fail,
            Some("missing-axis-records".to_string()),
        );
    }

    #[test]
    fn test_skip_static_font() {
        let testable = test_able("source-sans-pro/TTF/SourceSansPro-Bold.ttf");
        let result = run_check(STAT_axis_record_for_each_axis, testable);
        assert_skip(&result);
    }
}
