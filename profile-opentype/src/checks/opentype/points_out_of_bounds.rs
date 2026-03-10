use fontations::skrifa::{
    raw::{
        tables::glyf::{Glyph, PointFlags},
        types::Point,
        TableProvider,
    },
    GlyphId,
};
use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert, Metadata};
use serde_json::json;

#[check(
    id = "opentype/points_out_of_bounds",
    rationale = "
        The glyf table specifies a bounding box for each glyph. This check
        ensures that all points in all glyph paths are within the bounding
        box. Glyphs with out-of-bounds points can cause rendering issues in
        some software, and should be corrected.
    ",
    proposal = "https://github.com/fonttools/fontbakery/issues/735",
    title = "Check for points out of bounds"
)]
fn points_out_of_bounds(t: &Testable, _context: &Context) -> CheckFnResult {
    let ttf = testfont!(t);
    let font = ttf.font();
    skip!(!ttf.has_table(b"glyf"), "no-glyf", "No glyf table");
    let glyf = font.glyf()?;
    let loca = font.loca(None)?;
    let mut problems = vec![];
    for gid in 0..font.maxp()?.num_glyphs() {
        let gid = GlyphId::new(gid.into());
        if let Some(Glyph::Simple(glyph)) = loca.get_glyf(gid, &glyf)? {
            let point_count = glyph.num_points();
            let mut points: Vec<Point<i32>> = vec![Point::default(); point_count];
            let mut flags = vec![PointFlags::default(); point_count];
            glyph.read_points_fast(&mut points, &mut flags)?;
            let x_min: i32 = glyph.x_min().into();
            let x_max: i32 = glyph.x_max().into();
            let y_min: i32 = glyph.y_min().into();
            let y_max: i32 = glyph.y_max().into();
            for point in &points {
                if point.x < x_min || point.x > x_max {
                    let msg = format!(
                        "{} (x={}, bounds are {}<->{})",
                        ttf.glyph_name_for_id_synthesise(gid),
                        point.x,
                        x_min,
                        x_max
                    );
                    let mut status = Status::warn("points-out-of-bounds", &msg);
                    status.add_metadata(Metadata::GlyphProblem {
                        glyph_name: ttf.glyph_name_for_id_synthesise(gid),
                        glyph_id: gid.to_u32(),
                        userspace_location: None,
                        position: Some((point.x as f32, point.y as f32)),
                        actual: Some(json!({"x_value": point.x, "x_min": x_min, "x_max": x_max})),
                        expected: Some(json!({"x_between": [x_min, x_max]})),
                        message: "Point is out of bounds horizontally.".to_string(),
                    });
                    problems.push(status);
                }
                if point.y < y_min || point.y > y_max {
                    let msg = format!(
                        "{} (y={}, bounds are {}<->{})",
                        ttf.glyph_name_for_id_synthesise(gid),
                        point.y,
                        y_min,
                        y_max
                    );
                    let mut status = Status::warn("points-out-of-bounds", &msg);
                    status.add_metadata(Metadata::GlyphProblem {
                        glyph_name: ttf.glyph_name_for_id_synthesise(gid),
                        glyph_id: gid.to_u32(),
                        userspace_location: None,
                        position: Some((point.x as f32, point.y as f32)),
                        actual: Some(json!({"y_value": point.y, "y_min": y_min, "y_max": y_max})),
                        expected: Some(json!({"y_between": [y_min, y_max]})),
                        message: "Point is out of bounds vertically.".to_string(),
                    });
                    problems.push(status);
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

    use super::points_out_of_bounds;

    #[test]
    fn test_pass_good_font() {
        let testable = test_able("familysans/FamilySans-Regular.ttf");
        let result = run_check(points_out_of_bounds, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_warn_points_out_of_bounds() {
        // Modify glyph bbox to be tighter than its actual points
        let mut testable = test_able("nunito/Nunito-Regular.ttf");
        use fontations::skrifa::{font::FontRef, raw::TableProvider, GlyphId, Tag};

        let f = FontRef::new(&testable.contents).unwrap();
        let glyf_data = f.table_data(Tag::new(b"glyf")).unwrap();
        let loca = f.loca(None).unwrap();
        let mut new_glyf = glyf_data.as_bytes().to_vec();

        // Find the first simple glyph with points and shrink its bbox
        // Glyph 1 (.notdef or space) may be empty; let's use a higher glyph
        for gid_val in 1..f.maxp().unwrap().num_glyphs() {
            let gid = GlyphId::new(gid_val.into());
            if let Some(offset) = loca.get_raw(gid_val as usize) {
                let next_offset = loca.get_raw(gid_val as usize + 1).unwrap_or(0);
                let offset = offset as usize;
                let next_offset = next_offset as usize;
                if next_offset > offset && next_offset - offset >= 10 {
                    // Simple glyph header: numberOfContours(2) + xMin(2) + yMin(2) + xMax(2) + yMax(2)
                    let num_contours = i16::from_be_bytes([new_glyf[offset], new_glyf[offset + 1]]);
                    if num_contours > 0 {
                        // Set xMax to 0 to force points to be "out of bounds"
                        new_glyf[offset + 6] = 0;
                        new_glyf[offset + 7] = 0;
                        break;
                    }
                }
            }
        }

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
        let result = run_check(points_out_of_bounds, testable);
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("points-out-of-bounds".to_string()),
        );
    }
}
