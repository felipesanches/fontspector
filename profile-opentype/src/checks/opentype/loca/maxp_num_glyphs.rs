use fontations::skrifa::raw::TableProvider;
use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};

#[check(
    id = "opentype/loca/maxp_num_glyphs",
    title = "Does the number of glyphs in the loca table match the maxp table?",
    rationale = "
        The 'maxp' table contains various statistics about the font, including the
        number of glyphs in the font. The 'loca' table contains the offsets to the
        locations of the glyphs in the font. The number of offsets in the 'loca' table
        should match the number of glyphs in the 'maxp' table. A failure here indicates
        a problem with the font compiler.
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/4829",  // legacy check
)]
fn maxp_num_glyphs(t: &Testable, _context: &Context) -> CheckFnResult {
    let font = testfont!(t);

    let loca = font
        .font()
        .loca(None)
        .map_err(|_| FontspectorError::skip("no-loca", "loca table not found"))?;
    if loca.len() != font.glyph_count {
        return Ok(Status::just_one_fail(
            "corrupt",
            "Corrupt \"loca\" table or wrong numGlyphs in \"maxp\" table.",
        ));
    } else {
        return Ok(Status::just_one_pass());
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    use super::maxp_num_glyphs;

    #[test]
    fn test_pass_good_font() {
        let testable = test_able("nunito/Nunito-Regular.ttf");
        let result = run_check(maxp_num_glyphs, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_fail_corrupt_loca() {
        let mut testable = test_able("nunito/Nunito-Regular.ttf");
        use fontations::skrifa::{font::FontRef, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let maxp_data = f.table_data(Tag::new(b"maxp")).unwrap();

        // Change numGlyphs (bytes 4-5) to a different value
        let mut new_maxp = maxp_data.as_bytes().to_vec();
        if new_maxp.len() >= 6 {
            let orig = u16::from_be_bytes([new_maxp[4], new_maxp[5]]);
            let new_val = orig.wrapping_sub(1);
            new_maxp[4] = (new_val >> 8) as u8;
            new_maxp[5] = (new_val & 0xFF) as u8;
        }

        let mut builder = fontations::write::FontBuilder::new();
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag == Tag::new(b"maxp") {
                builder.add_raw(tag, &new_maxp);
            } else if let Some(table_data) = f.table_data(tag) {
                builder.add_raw(tag, table_data);
            }
        }
        testable.contents = builder.build();
        let result = run_check(maxp_num_glyphs, testable);
        assert_results_contain(&result, StatusCode::Fail, Some("corrupt".to_string()));
    }
}
