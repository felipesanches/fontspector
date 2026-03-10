use fontations::skrifa::raw::TableProvider;
use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};

#[check(
    id = "opentype/GDEF_spacing_marks",
    rationale = "
        Glyphs in the GDEF mark glyph class should be non-spacing.

        Spacing glyphs in the GDEF mark glyph class may have incorrect anchor
        positioning that was only intended for building composite glyphs during design.
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/2877",
    title = "Check glyphs in mark glyph class are non-spacing."
)]
fn GDEF_spacing_marks(f: &Testable, context: &Context) -> CheckFnResult {
    let font = testfont!(f);
    let hmtx = font.font().hmtx()?;
    let gdef = font
        .font()
        .gdef()
        .map_err(|_| FontspectorError::skip("no-gdef", "GDEF table unreadable or not present"))?;
    let glyph_classdef = gdef.glyph_class_def().ok_or_else(|| {
        FontspectorError::skip("no-glyph-class-def", "GDEF table has no GlyphClassDef")
    })??;
    let nonspacing_mark_glyphs = bullet_list(
        context,
        glyph_classdef
            .iter()
            .filter(|(glyph, class)| *class == 3 && hmtx.advance((*glyph).into()).unwrap_or(0) > 0)
            .map(|(glyph, _)| font.glyph_name_for_id_synthesise(glyph)),
    );
    if !nonspacing_mark_glyphs.is_empty() {
        return Ok(Status::just_one_warn("spacing-mark-glyphs", &format!(
            "The following glyphs seem to be spacing (because they have width > 0 on the hmtx table) so they may be in the GDEF mark glyph class by mistake, or they should have zero width instead:\n\n{nonspacing_mark_glyphs}"
        )));
    }

    Ok(Status::just_one_pass())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use fontspector_checkapi::{
        codetesting::{
            assert_pass, assert_results_contain, assert_skip, remove_table, run_check, test_able,
        },
        StatusCode,
    };

    use super::GDEF_spacing_marks;

    #[test]
    fn test_skip_no_gdef() {
        let testable = test_able("gdef_test.ttf");
        let result = run_check(GDEF_spacing_marks, testable);
        assert_skip(&result);
    }

    #[test]
    fn test_skip_no_gdef_removed() {
        let mut testable = test_able("familysans/FamilySans-Regular.ttf");
        remove_table(&mut testable, b"GDEF");
        let result = run_check(GDEF_spacing_marks, testable);
        assert_skip(&result);
    }

    #[test]
    fn test_pass_existing_font() {
        let testable = test_able("nunito/Nunito-Regular.ttf");
        let result = run_check(GDEF_spacing_marks, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_pass_empty_gdef() {
        let mut testable = test_able("gdef_test.ttf");
        use fontations::skrifa::{font::FontRef, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let gdef_bytes: Vec<u8> = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00,
        ];
        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        builder.add_raw(Tag::new(b"GDEF"), &gdef_bytes);
        testable.contents = builder.build();
        let result = run_check(GDEF_spacing_marks, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_warn_spacing_mark_glyph() {
        // 'A' has non-zero width. Mark it as a mark glyph in GDEF.
        let mut testable = test_able("gdef_test.ttf");
        use fontations::skrifa::{font::FontRef, MetadataProvider, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let a_gid = f.charmap().map('A' as u32).unwrap().to_u32() as u16;

        let gdef_bytes: Vec<u8> = vec![
            0x00,
            0x01,
            0x00,
            0x00,
            0x00,
            0x0C,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x02,
            0x00,
            0x01,
            (a_gid >> 8) as u8,
            (a_gid & 0xFF) as u8,
            (a_gid >> 8) as u8,
            (a_gid & 0xFF) as u8,
            0x00,
            0x03,
        ];
        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        builder.add_raw(Tag::new(b"GDEF"), &gdef_bytes);
        testable.contents = builder.build();
        let result = run_check(GDEF_spacing_marks, testable);
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("spacing-mark-glyphs".to_string()),
        );
    }
}
