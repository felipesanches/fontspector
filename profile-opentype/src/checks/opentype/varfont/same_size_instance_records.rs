use std::collections::HashSet;

use fontations::skrifa::MetadataProvider;
use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert};

#[check(
    id = "opentype/varfont/same_size_instance_records",
    title = "Validates that all of the instance records in a given font have the same size",
    rationale = "According to the 'fvar' documentation in OpenType spec v1.9
        https://docs.microsoft.com/en-us/typography/opentype/spec/fvar

        All of the instance records in a given font must be the same size, with
        all either including or omitting the postScriptNameID field. [...]
        If the value is 0xFFFF, then the value is ignored, and no PostScript name
        equivalent is provided for the instance.",
    proposal = "https://github.com/fonttools/fontbakery/issues/3705"
)]
fn same_size_instance_records(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    skip!(!f.is_variable_font(), "not-variable", "Not a variable font");
    skip!(
        f.font().named_instances().is_empty(),
        "no-instance-records",
        "Font has no instance records."
    );
    let has_or_hasnt_postscriptname: HashSet<bool> = f
        .font()
        .named_instances()
        .iter()
        .map(|ni| ni.postscript_name_id().is_none())
        .collect();
    Ok(if has_or_hasnt_postscriptname.len() > 1 {
        Status::just_one_fail(
            "different-size-instance-records",
            "Instance records don't all have the same size.",
        )
    } else {
        Status::just_one_pass()
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    use super::same_size_instance_records;

    #[test]
    fn test_pass_good_font() {
        let testable = test_able("cabinvf/Cabin[wdth,wght].ttf");
        let result = run_check(same_size_instance_records, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_pass_all_with_postscript_name() {
        use fontations::{
            skrifa::raw::TableProvider,
            write::{from_obj::ToOwnedTable, tables::fvar::Fvar, FontBuilder},
        };
        use fontspector_checkapi::{FileTypeConvert, TTF};

        let mut testable = test_able("cabinvf/Cabin[wdth,wght].ttf");
        let f = TTF.from_testable(&testable).unwrap();
        let mut fvar: Fvar = f.font().fvar().unwrap().to_owned_table();
        // Set postScriptNameID for all instances
        for (i, inst) in fvar.axis_instance_arrays.instances.iter_mut().enumerate() {
            inst.post_script_name_id = Some(fontations::write::types::NameId::new(256 + i as u16));
        }
        let new_bytes = FontBuilder::new()
            .add_table(&fvar)
            .unwrap()
            .copy_missing_tables(f.font())
            .build();
        testable.contents = new_bytes;
        let result = run_check(same_size_instance_records, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_fail_different_size_records() {
        // Rebuild fvar with postScriptNameID field: set one instance to a real
        // value and the rest to 0xFFFF. The check sees Some vs None mismatch.
        let mut testable = test_able("cabinvf/Cabin[wdth,wght].ttf");
        use fontations::skrifa::{font::FontRef, raw::TableProvider, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let fvar_data = f.table_data(Tag::new(b"fvar")).unwrap();
        let fvar_bytes = fvar_data.as_bytes();

        // fvar header: 8 x uint16 = 16 bytes
        let axes_offset = u16::from_be_bytes([fvar_bytes[4], fvar_bytes[5]]) as usize;
        let axis_count = u16::from_be_bytes([fvar_bytes[8], fvar_bytes[9]]) as usize;
        let axis_size = u16::from_be_bytes([fvar_bytes[10], fvar_bytes[11]]) as usize;
        let instance_count = u16::from_be_bytes([fvar_bytes[12], fvar_bytes[13]]) as usize;
        let old_instance_size = u16::from_be_bytes([fvar_bytes[14], fvar_bytes[15]]) as usize;
        let new_instance_size = old_instance_size + 2; // Add postScriptNameID

        // Build new fvar: header + axes + new instances (with postScriptNameID)
        let mut new_fvar = Vec::new();
        // Copy header (16 bytes), update instanceSize
        new_fvar.extend_from_slice(&fvar_bytes[..16]);
        new_fvar[14] = (new_instance_size >> 8) as u8;
        new_fvar[15] = (new_instance_size & 0xFF) as u8;

        // Copy axes
        let axes_end = axes_offset + axis_count * axis_size;
        new_fvar.extend_from_slice(&fvar_bytes[axes_offset..axes_end]);

        // Copy instances, appending postScriptNameID to each
        let inst_start = axes_end;
        for i in 0..instance_count {
            let off = inst_start + i * old_instance_size;
            new_fvar.extend_from_slice(&fvar_bytes[off..off + old_instance_size]);
            if i == 0 {
                // First instance: set postScriptNameID to 256
                new_fvar.extend_from_slice(&[0x01, 0x00]);
            } else {
                // Others: 0xFFFF (no PostScript name)
                new_fvar.extend_from_slice(&[0xFF, 0xFF]);
            }
        }

        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag == Tag::new(b"fvar") {
                builder.add_raw(tag, &new_fvar);
            } else if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        testable.contents = builder.build();
        let result = run_check(same_size_instance_records, testable);
        assert_results_contain(
            &result,
            StatusCode::Fail,
            Some("different-size-instance-records".to_string()),
        );
    }
}
