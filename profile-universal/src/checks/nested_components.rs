use std::collections::HashMap;

use fontations::skrifa::{
    raw::{
        tables::{
            glyf::{Glyf, Glyph},
            loca::Loca,
        },
        TableProvider,
    },
    GlyphId,
};
use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};

use super::transformed_components::decompose_components_impl;

#[check(
    id = "nested_components",
    rationale = "
        There have been bugs rendering variable fonts with nested components.
        Additionally, some static fonts with nested components have been reported
        to have rendering and printing issues.

        For more info, see:
        * https://github.com/fonttools/fontbakery/issues/2961
        * https://github.com/arrowtype/recursive/issues/412
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/2961",
    title = "Ensure glyphs do not have components which are themselves components.",
    hotfix = decompose_nested_components
)]
fn nested_components(f: &Testable, context: &Context) -> CheckFnResult {
    let font = testfont!(f);
    let loca = font
        .font()
        .loca(None)
        .map_err(|_| FontspectorError::skip("no-loca", "loca table not found"))?;
    let glyf = font
        .font()
        .glyf()
        .map_err(|_| FontspectorError::skip("no-glyf", "glyf table not found"))?;
    let mut failures = vec![];
    let composite_glyphs: HashMap<GlyphId, _> = font
        .all_glyphs()
        .filter_map(|glyphid| {
            if let Some(Glyph::Composite(composite)) = loca.get_glyf(glyphid, &glyf).ok()? {
                Some((glyphid, composite))
            } else {
                None
            }
        })
        .collect();
    for (glyphid, composite) in composite_glyphs.iter() {
        for component in composite.components() {
            if composite_glyphs.contains_key(&component.glyph.into()) {
                failures.push(font.glyph_name_for_id_synthesise(*glyphid));
                break;
            }
        }
    }
    if failures.is_empty() {
        Ok(Status::just_one_pass())
    } else {
        Ok(Status::just_one_fail(
            "found-nested-components",
            &format!(
                "The following glyphs have components which are themselves component glyphs:\n\n{}",
                bullet_list(context, failures)
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use fontspector_checkapi::codetesting::{
        assert_pass, assert_results_contain, run_check, test_able,
    };
    use fontspector_checkapi::StatusCode;

    use fontations::skrifa::{
        font::FontRef,
        raw::{tables::glyf::Glyph, types::Tag, TableProvider},
        GlyphNames,
    };
    use fontations::write::FontBuilder;

    #[test]
    fn test_nested_components_pass() {
        // Nunito Regular should have no nested components
        let testable = test_able("nunito/Nunito-Regular.ttf");
        let results = run_check(super::nested_components, testable);
        assert_pass(&results);
    }

    #[test]
    fn test_nested_components_fail() {
        // Modify Nunito so that quotedbl's first component points to "second",
        // which itself has components, creating a nested component situation.
        let mut testable = test_able("nunito/Nunito-Regular.ttf");

        let f = FontRef::new(&testable.contents).unwrap();
        let names = GlyphNames::new(&f);

        // Find glyph IDs for "quotedbl" and "second"
        let quotedbl_gid = names
            .iter()
            .find(|(_gid, name)| name.as_str() == "quotedbl")
            .map(|(gid, _)| gid)
            .expect("quotedbl glyph not found");
        let second_gid = names
            .iter()
            .find(|(_gid, name)| name.as_str() == "second")
            .map(|(gid, _)| gid)
            .expect("second glyph not found");

        // Verify that quotedbl is composite and second is composite
        let loca = f.loca(None).unwrap();
        let glyf = f.glyf().unwrap();
        assert!(
            matches!(
                loca.get_glyf(quotedbl_gid, &glyf),
                Ok(Some(Glyph::Composite(_)))
            ),
            "quotedbl should be a composite glyph"
        );
        assert!(
            matches!(
                loca.get_glyf(second_gid, &glyf),
                Ok(Some(Glyph::Composite(_)))
            ),
            "second should be a composite glyph"
        );

        // Get the raw glyf table data and modify it
        let glyf_tag = Tag::new(b"glyf");
        let glyf_data = f.table_data(glyf_tag).unwrap();
        let mut glyf_bytes = glyf_data.as_ref().to_vec();

        // Find the offset of quotedbl's glyph data in the glyf table using loca
        let loca_tag = Tag::new(b"loca");
        let head = f.head().unwrap();
        let loca_data = f.table_data(loca_tag).unwrap();
        let loca_bytes = loca_data.as_ref();
        let gid_val = quotedbl_gid.to_u32() as usize;

        let glyf_offset = if head.index_to_loc_format() == 0 {
            // Short format: offsets are uint16, actual offset = value * 2
            let idx = gid_val * 2;
            (u16::from_be_bytes(loca_bytes[idx..idx + 2].try_into().unwrap()) as usize) * 2
        } else {
            // Long format: offsets are uint32
            let idx = gid_val * 4;
            u32::from_be_bytes(loca_bytes[idx..idx + 4].try_into().unwrap()) as usize
        };

        // In a composite glyph, the data starts with:
        //   int16 numberOfContours (should be -1)
        //   int16 xMin, yMin, xMax, yMax (bounding box = 10 bytes total header)
        // Then component records follow:
        //   uint16 flags
        //   uint16 glyphIndex  <-- this is what we modify
        let component_glyph_id_offset = glyf_offset + 10 + 2; // skip header (10) + flags (2)
        glyf_bytes[component_glyph_id_offset..component_glyph_id_offset + 2]
            .copy_from_slice(&(second_gid.to_u32() as u16).to_be_bytes());

        // Rebuild font with modified glyf table
        let mut builder = FontBuilder::new();
        builder.add_raw(glyf_tag, &glyf_bytes);
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag != glyf_tag {
                if let Some(table_data) = f.table_data(tag) {
                    builder.add_raw(tag, table_data);
                }
            }
        }
        testable.contents = builder.build();

        let results = run_check(super::nested_components, testable);
        assert_results_contain(
            &results,
            StatusCode::Fail,
            Some("found-nested-components".to_string()),
        );
    }
}

fn get_depth(glyph_id: GlyphId, loca: &Loca, glyf: &Glyf) -> u32 {
    let mut depth = 0;
    let glyph_entry = loca.get_glyf(glyph_id, glyf).ok().flatten();
    if let Some(Glyph::Composite(composite)) = glyph_entry {
        depth = 1 + composite
            .components()
            .map(|component| get_depth(component.glyph.into(), loca, glyf))
            .max()
            .unwrap_or(0)
    }
    depth
}

fn decompose_nested_components(t: &mut Testable) -> FixFnResult {
    let font = testfont!(t);
    let loca = font.font().loca(None)?;
    let glyf = font.font().glyf()?;
    let mut depths = HashMap::new();
    for glyph in font.all_glyphs() {
        depths.insert(glyph, get_depth(glyph, &loca, &glyf));
    }
    // Drop all with depth <2
    depths.retain(|_, depth| *depth > 1);
    // Sort by depth, descending
    let mut sorted_glyphs: Vec<GlyphId> = depths.keys().copied().collect();
    #[allow(clippy::indexing_slicing)] // We know the key is present!
    sorted_glyphs.sort_by_key(|&glyph| depths[&glyph]);
    sorted_glyphs.reverse();

    decompose_components_impl(t, &sorted_glyphs)
}
