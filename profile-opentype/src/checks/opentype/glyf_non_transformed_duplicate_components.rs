use fontations::skrifa::{
    raw::{
        tables::glyf::{Anchor, Glyph},
        TableProvider,
    },
    GlyphId,
};
use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert, Metadata};
use serde_json::json;
use std::collections::HashSet;

#[check(
    id = "opentype/glyf_non_transformed_duplicate_components",
    rationale = "
        There have been cases in which fonts had faulty double quote marks, with each
        of them containing two single quote marks as components with the same
        x, y coordinates which makes them visually look like single quote marks.

        This check ensures that glyphs do not contain duplicate components
        which have the same x,y coordinates.
    ",
    proposal = "https://github.com/fonttools/fontbakery/pull/2709",
    title = "Check glyphs do not have duplicate components which have the same x,y coordinates."
)]
fn glyf_non_transformed_duplicate_components(t: &Testable, _context: &Context) -> CheckFnResult {
    let ttf = testfont!(t);
    let font = ttf.font();
    skip!(!ttf.has_table(b"glyf"), "no-glyf", "No glyf table");
    let glyf = font.glyf()?;
    let loca = font.loca(None)?;
    let mut problems = vec![];
    for gid in 0..font.maxp()?.num_glyphs() {
        let gid = GlyphId::new(gid.into());
        if let Some(Glyph::Composite(glyph)) = loca.get_glyf(gid, &glyf)? {
            let mut components = HashSet::new();
            for component in glyph.components() {
                if let Anchor::Offset { x, y } = component.anchor {
                    if !components.insert((component.glyph, x, y)) {
                        let msg = format!(
                            "{}: duplicate component {} at {},{}. Duplicate components may cause rendering issues.",
                            ttf.glyph_name_for_id_synthesise(gid),
                            ttf.glyph_name_for_id_synthesise(component.glyph),
                            x,
                            y
                        );
                        let mut status = Status::fail("found-duplicates", &msg);
                        status.add_metadata(Metadata::GlyphProblem {
                            glyph_name: ttf.glyph_name_for_id_synthesise(gid),
                            glyph_id: gid.to_u32(),
                            userspace_location: None,
                            position: Some((x as f32, y as f32)),
                            actual: Some(json!({
                                "component": ttf.glyph_name_for_id_synthesise(component.glyph),
                                "duplicate_at": [x, y]
                            })),
                            expected: Some(json!("No duplicate components at same position")),
                            message: msg,
                        });
                        problems.push(status);
                    }
                }
            }
        }
    }
    return_result(problems)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    use super::glyf_non_transformed_duplicate_components;

    #[test]
    fn test_pass_good_font() {
        let testable = test_able("nunito/Nunito-Regular.ttf");
        let result = run_check(glyf_non_transformed_duplicate_components, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_fail_duplicate_components() {
        // Modify the font so quotedbl's two components have the same x,y
        // This requires raw glyf table manipulation
        let mut testable = test_able("nunito/Nunito-Regular.ttf");

        use fontations::skrifa::{
            font::FontRef,
            raw::{
                tables::glyf::{Anchor, Glyph},
                TableProvider,
            },
            GlyphId, MetadataProvider, Tag,
        };

        let f = FontRef::new(&testable.contents).unwrap();
        let glyf_data = f.table_data(Tag::new(b"glyf")).unwrap();
        let glyf_bytes = glyf_data.as_bytes();
        let loca = f.loca(None).unwrap();
        let glyf = f.glyf().unwrap();

        // Find the quotedbl glyph
        let quotedbl_gid = f.charmap().map('"' as u32);
        if let Some(gid) = quotedbl_gid {
            if let Some(Glyph::Composite(_)) = loca.get_glyf(gid, &glyf).unwrap() {
                // We know it's composite. Let's get its raw data and modify
                // the component offsets to make them identical (both at 0,0).
                let offset = loca.get_raw(gid.to_u32() as usize).unwrap() as usize;
                let next_offset = loca.get_raw(gid.to_u32() as usize + 1).unwrap() as usize;
                let glyph_data = &glyf_bytes[offset..next_offset];

                // Copy the full glyf table, modify the duplicate component offsets
                let mut new_glyf = glyf_bytes.to_vec();
                // For a composite glyph, after the header (10 bytes),
                // each component has: flags(2) + glyphIndex(2) + args
                // The first component's args depend on flags.
                // For ARG_1_AND_2_ARE_WORDS: args are 2x i16 (4 bytes)
                // For !ARG_1_AND_2_ARE_WORDS: args are 2x i8 (2 bytes)

                // Rather than parsing the binary, let's set all offset bytes
                // in the components to zero. First, read the header.
                if glyph_data.len() >= 10 {
                    let _num_contours = i16::from_be_bytes([glyph_data[0], glyph_data[1]]);
                    // Skip header (10 bytes), parse components
                    let mut pos = 10;
                    while pos + 4 <= glyph_data.len() {
                        let flags = u16::from_be_bytes([glyph_data[pos], glyph_data[pos + 1]]);
                        // Skip flags(2) + glyphIndex(2)
                        let arg_pos = pos + 4;
                        let arg_1_and_2_are_words = (flags & 0x0001) != 0;
                        let arg_size = if arg_1_and_2_are_words { 4 } else { 2 };
                        if arg_pos + arg_size <= glyph_data.len() {
                            // Set x,y offsets to 0
                            for i in 0..arg_size {
                                new_glyf[offset + arg_pos + i] = 0;
                            }
                        }
                        let more_components = (flags & 0x0020) != 0;
                        // Calculate total component size
                        let mut comp_size = 4 + arg_size;
                        if (flags & 0x0008) != 0 {
                            comp_size += 2; // WE_HAVE_A_SCALE
                        } else if (flags & 0x0040) != 0 {
                            comp_size += 4; // WE_HAVE_AN_X_AND_Y_SCALE
                        } else if (flags & 0x0080) != 0 {
                            comp_size += 8; // WE_HAVE_A_TWO_BY_TWO
                        }
                        pos += comp_size;
                        if !more_components {
                            break;
                        }
                    }
                }

                // Rebuild font with modified glyf
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
                let result = run_check(glyf_non_transformed_duplicate_components, testable);
                assert_results_contain(
                    &result,
                    StatusCode::Fail,
                    Some("found-duplicates".to_string()),
                );
                return;
            }
        }
        panic!("Could not find composite quotedbl glyph for test");
    }
}
