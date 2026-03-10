use crate::checks::opentype::GDEF_mark_chars::is_nonspacing_mark;
use fontations::skrifa::{raw::TableProvider, GlyphId16, MetadataProvider};
use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};

fn swaption<T, U>(a: T, b: Option<U>) -> Option<(T, U)> {
    b.map(|b| (a, b))
}

#[check(
    id = "opentype/GDEF_non_mark_chars",
    rationale = "
        Glyphs in the GDEF mark glyph class become non-spacing and may be repositioned
        if they have mark anchors.

        Only combining mark glyphs should be in that class. Any non-mark glyph
        must not be in that class, in particular spacing glyphs.
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/2877",
    title = "Check GDEF mark glyph class doesn't have characters that are not marks."
)]
fn GDEF_non_mark_chars(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let gdef = f
        .font()
        .gdef()
        .map_err(|_| FontspectorError::skip("no-gdef", "GDEF table unreadable or not present"))?;
    let glyph_classdef = gdef.glyph_class_def().ok_or_else(|| {
        FontspectorError::skip("no-glyph-class-def", "GDEF table has no GlyphClassDef")
    })??;
    let codepoints = f.codepoints(Some(context));
    let non_mark_gids = codepoints
        .iter()
        .flat_map(|cp| char::from_u32(*cp))
        .filter(|&cp| !is_nonspacing_mark(cp))
        .flat_map(|cp| swaption(cp, f.font().charmap().map(cp)))
        .flat_map(|(cp, gid)| swaption(cp, GlyphId16::try_from(gid).ok()));
    let non_mark_gids_in_mark = non_mark_gids.filter(|(_cp, gid)| glyph_classdef.get(*gid) == 3);
    if non_mark_gids_in_mark.clone().count() > 0 {
        return Ok(Status::just_one_warn(
            "non-mark-chars",
            &format!(
                "The following non-mark characters should not be in the GDEF mark glyph class:\n\n{}",
                bullet_list(
                    context,
                    non_mark_gids_in_mark.map(|(cp, gid)| format!(
                        "U+{:04X} ({})",
                        cp as u32,
                        f.glyph_name_for_id_synthesise(gid)
                    ))
                ),
            ),
        ));
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

    use super::GDEF_non_mark_chars;

    #[test]
    fn test_skip_no_gdef() {
        let testable = test_able("gdef_test.ttf");
        let result = run_check(GDEF_non_mark_chars, testable);
        assert_skip(&result);
    }

    #[test]
    fn test_skip_no_gdef_removed() {
        let mut testable = test_able("nunito/Nunito-Regular.ttf");
        remove_table(&mut testable, b"GDEF");
        let result = run_check(GDEF_non_mark_chars, testable);
        assert_skip(&result);
    }

    #[test]
    fn test_pass_existing_font() {
        let testable = test_able("nunito/Nunito-Regular.ttf");
        let result = run_check(GDEF_non_mark_chars, testable);
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
        let result = run_check(GDEF_non_mark_chars, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_pass_only_mark_chars_in_mark_class() {
        let mut testable = test_able("gdef_test.ttf");
        use fontations::skrifa::{font::FontRef, MetadataProvider, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let acutecomb_gid = f.charmap().map(0x0301u32).unwrap().to_u32() as u16;
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
            (acutecomb_gid >> 8) as u8,
            (acutecomb_gid & 0xFF) as u8,
            (acutecomb_gid >> 8) as u8,
            (acutecomb_gid & 0xFF) as u8,
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
        let result = run_check(GDEF_non_mark_chars, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_warn_non_mark_in_mark_class() {
        let mut testable = test_able("gdef_test.ttf");
        use fontations::skrifa::{font::FontRef, MetadataProvider, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let acute_gid = f.charmap().map(0x00B4u32).unwrap().to_u32() as u16;
        let acutecomb_gid = f.charmap().map(0x0301u32).unwrap().to_u32() as u16;

        let mut gdef_bytes: Vec<u8> = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x02,
        ];
        gdef_bytes.extend_from_slice(&[(acute_gid >> 8) as u8, (acute_gid & 0xFF) as u8]);
        gdef_bytes.extend_from_slice(&[(acute_gid >> 8) as u8, (acute_gid & 0xFF) as u8]);
        gdef_bytes.extend_from_slice(&[0x00, 0x03]);
        gdef_bytes.extend_from_slice(&[(acutecomb_gid >> 8) as u8, (acutecomb_gid & 0xFF) as u8]);
        gdef_bytes.extend_from_slice(&[(acutecomb_gid >> 8) as u8, (acutecomb_gid & 0xFF) as u8]);
        gdef_bytes.extend_from_slice(&[0x00, 0x03]);

        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        builder.add_raw(Tag::new(b"GDEF"), &gdef_bytes);
        testable.contents = builder.build();

        let result = run_check(GDEF_non_mark_chars, testable);
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("non-mark-chars".to_string()),
        );
    }
}
