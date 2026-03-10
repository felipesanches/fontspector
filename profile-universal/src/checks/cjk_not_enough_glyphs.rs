use fontations::skrifa::raw::TableProvider;
use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert, Metadata, TestFont};
use serde_json::json;

const CJK_CODEPAGE_BITS: [u8; 5] = [17, 18, 19, 20, 21];

fn is_claiming_to_be_cjk_font(f: &TestFont) -> bool {
    if let Ok(os2) = f.font().os2() {
        if let Some(codepages) = os2.ul_code_page_range_1() {
            for bit in CJK_CODEPAGE_BITS.iter() {
                if codepages & (1 << bit) != 0 {
                    return true;
                }
            }
        }
        // Urgh this is messy
        if (os2.ul_unicode_range_1() & (1 << 28)) != 0 || // Jamo
           (os2.ul_unicode_range_2() & (1 << (49-32))) != 0 || // Katakana
           (os2.ul_unicode_range_2() & (1 << (50-32))) != 0 || // Hiragana
            (os2.ul_unicode_range_2() & (1 << (51-32))) != 0 || // Bopomofo
            (os2.ul_unicode_range_2() & (1 << (52-32))) != 0 || // Hangul Compatibility Jamo
            (os2.ul_unicode_range_2() & (1 << (54-32))) != 0 || // Enclosed CJK Letters And Months
            (os2.ul_unicode_range_2() & (1 << (55-32))) != 0 || // CJK Compatibility
            (os2.ul_unicode_range_2() & (1 << (56-32))) != 0 || // Hangul Syllables
            (os2.ul_unicode_range_2() & (1 << (59-32))) != 0 || // CJK Unified Ideographs
            (os2.ul_unicode_range_2() & (1 << (61-32))) != 0
        // CJK Strokes
        {
            return true;
        }
        false
    } else {
        false
    }
}

#[check(
    id = "cjk_not_enough_glyphs",
    rationale = "
        Kana has 150 characters and it's the smallest CJK writing system.

        If a font contains less CJK glyphs than this writing system, we inform the
        user that some glyphs may be encoded incorrectly.
    ",
    title = "Any CJK font should contain at least a minimal set of 150 CJK characters.",
    proposal = "https://github.com/fonttools/fontbakery/pull/3214"
)]
fn cjk_not_enough_glyphs(f: &Testable, context: &Context) -> CheckFnResult {
    let font = testfont!(f);
    skip!(
        !is_claiming_to_be_cjk_font(&font),
        "not-cjk",
        "Not a CJK font."
    );
    let cjk_glyphs: Vec<_> = font.cjk_codepoints(Some(context)).collect();
    let cjk_glyph_count = cjk_glyphs.len();

    if cjk_glyph_count > 0 && cjk_glyph_count < 150 {
        let num_cjk_glyphs = if cjk_glyph_count == 1 {
            "There is only one CJK glyph"
        } else {
            &format!("There are only {cjk_glyph_count} CJK glyphs")
        };
        let cjk_glyphs_str: Vec<String> = cjk_glyphs.iter().map(|s| s.to_string()).collect();
        let message = format!(
            "{} when there needs to be at least 150 in order to support the smallest CJK writing system, Kana.\nThe following CJK glyphs were found:\n\n{}\nPlease check that these glyphs have the correct unicodes.",
            num_cjk_glyphs,
            bullet_list(context, cjk_glyphs_str.clone())
        );
        let mut status = Status::warn("cjk-not-enough-glyphs", &message);
        status.add_metadata(Metadata::FontProblem {
            message: message.clone(),
            context: Some(json!({
                "cjk_glyph_count": cjk_glyph_count,
                "required_minimum": 150,
                "cjk_glyphs_found": cjk_glyphs_str,
            })),
        });
        return return_result(vec![status]);
    }
    Ok(Status::just_one_pass())
}

#[cfg(test)]
mod tests {
    use fontspector_checkapi::{
        codetesting::{
            assert_messages_contain, assert_pass, assert_results_contain, remap_glyph, run_check,
            test_able,
        },
        StatusCode,
    };

    use fontations::skrifa::{font::FontRef, Tag};
    use fontations::write::FontBuilder;

    #[test]
    fn test_cjk_not_enough_glyphs_pass() {
        // NotoSansJP is a CJK font with plenty of CJK glyphs (>150), should PASS
        let testable = test_able("cjk/NotoSansJP[wght].ttf");
        let results = run_check(super::cjk_not_enough_glyphs, testable);
        assert_pass(&results);
    }

    #[test]
    fn test_cjk_not_enough_glyphs_skip_not_cjk() {
        // Montserrat is not a CJK font, should be SKIPPED
        let testable = test_able("montserrat/Montserrat-Regular.ttf");
        let results = run_check(super::cjk_not_enough_glyphs, testable);
        assert_results_contain(&results, StatusCode::Skip, Some("not-cjk".to_string()));
    }

    /// Set the CJK codepage bit (bit 17) in OS/2 ulCodePageRange1.
    /// ulCodePageRange1 is at byte offset 78 in the OS/2 table (for version >= 1).
    fn set_os2_cjk_codepage_bit(testable: &mut fontspector_checkapi::prelude::Testable) {
        let os2_tag = Tag::new(b"OS/2");
        let f = FontRef::new(&testable.contents).unwrap();
        let os2_data = f.table_data(os2_tag).unwrap();
        let mut os2_bytes = os2_data.as_ref().to_vec();
        // ulCodePageRange1 is a big-endian u32 at offset 78
        let cp_range = u32::from_be_bytes(os2_bytes[78..82].try_into().unwrap());
        let new_cp_range = cp_range | (1 << 17);
        os2_bytes[78..82].copy_from_slice(&new_cp_range.to_be_bytes());
        // Rebuild font with modified OS/2 table
        let mut builder = FontBuilder::new();
        builder.add_raw(os2_tag, &os2_bytes);
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag != os2_tag {
                if let Some(table_data) = f.table_data(tag) {
                    builder.add_raw(tag, table_data);
                }
            }
        }
        testable.contents = builder.build();
    }

    #[test]
    fn test_cjk_not_enough_glyphs_warn_one_glyph() {
        // Modify Montserrat to claim CJK and have only one CJK glyph
        let mut testable = test_able("montserrat/Montserrat-Regular.ttf");
        // Add first CJK Unified Ideograph codepoint mapped to glyph "A"
        remap_glyph(&mut testable, 0x4E00, "A").unwrap();
        // Set the CJK codepage bit in OS/2
        set_os2_cjk_codepage_bit(&mut testable);
        let results = run_check(super::cjk_not_enough_glyphs, testable);
        assert_results_contain(
            &results,
            StatusCode::Warn,
            Some("cjk-not-enough-glyphs".to_string()),
        );
        assert_messages_contain(&results, "There is only one CJK glyph");
    }

    #[test]
    fn test_cjk_not_enough_glyphs_warn_two_glyphs() {
        // Modify Montserrat to claim CJK and have two CJK glyphs
        let mut testable = test_able("montserrat/Montserrat-Regular.ttf");
        remap_glyph(&mut testable, 0x4E00, "A").unwrap();
        remap_glyph(&mut testable, 0x4E01, "B").unwrap();
        set_os2_cjk_codepage_bit(&mut testable);
        let results = run_check(super::cjk_not_enough_glyphs, testable);
        assert_results_contain(
            &results,
            StatusCode::Warn,
            Some("cjk-not-enough-glyphs".to_string()),
        );
        assert_messages_contain(&results, "There are only 2 CJK glyphs");
    }
}
