use fontations::skrifa::MetadataProvider;
use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert};

#[check(
    id = "opentype/fvar/axis_ranges_correct",
    title = "Axes and named instances fall within correct ranges?",
    rationale = "According to the Open-Type spec's registered design-variation tags, instances in a variable font should have certain prescribed values.
        If a variable font has a 'wght' (Weight) axis, the valid coordinate range is 1-1000.
        If a variable font has a 'wdth' (Width) axis, the valid numeric range is strictly greater than zero.
        If a variable font has a 'slnt' (Slant) axis, then the coordinate of its 'Regular' instance is required to be 0.
        If a variable font has a 'ital' (Slant) axis, then the coordinate of its 'Regular' instance is required to be 0.",
    proposal = "https://github.com/fonttools/fontbakery/issues/2572"
)]
fn axis_ranges_correct(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    skip!(!f.is_variable_font(), "not-variable", "Not a variable font");

    let mut problems = vec![];
    for (name, location) in f.named_instances() {
        if let Some(wght) = location.get("wght") {
            if !(1.0..=1000.0).contains(wght) {
                problems.push(Status::fail(
                    "wght-out-of-range",
                    &format!(
                        "Instance {name} has wght coordinate of {wght}, expected between 1 and 1000"
                    ),
                ));
            }
        }
        if let Some(wdth) = location.get("wdth") {
            if *wdth < 1.0 {
                problems.push(Status::fail(
                    "wdth-out-of-range",
                    &format!("Instance {name} has wdth coordinate of {wdth}, expected at least 1"),
                ));
            }
            if *wdth > 1000.0 {
                problems.push(Status::warn(
                    "wdth-greater-than-1000",
                    &format!(
                        "Instance {name} has wdth coordinate of {wdth}, which is valid but unusual"
                    ),
                ));
            }
        }
    }

    let axes = f.font().axes();
    if let Some(ital) = axes.iter().find(|axis| axis.tag() == "ital") {
        if !(ital.min_value() == 0.0 && ital.max_value() == 1.0) {
            problems.push(Status::fail(
                "invalid-ital-range",
                &format!(
                    "The range of values for the \"ital\" axis in this font is {} to {}.
                    The italic axis range must be 0 to 1, where Roman is 0 and Italic 1.
                    If you prefer a bigger variation range consider using the \"Slant\" axis instead of \"Italic\".",
                    ital.min_value(), ital.max_value()
                ),
            ));
        }
    }

    if let Some(slnt) = axes.iter().find(|axis| axis.tag() == "slnt") {
        if !(slnt.min_value() < 0.0 && slnt.max_value() >= 0.0) {
            problems.push(Status::warn(
                "unusual-slnt-range",
                &format!(
                    "The range of values for the \"slnt\" axis in this font only allows positive coordinates (from {} to {}),
                    indicating that this may be a back slanted design, which is rare. If that's not the case, then
                    the \"slnt\" axis should be a range of negative values instead.",
                    slnt.min_value(), slnt.max_value()
                ),
            ));
        }
    }
    return_result(problems)
}

#[allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    #[test]
    fn test_axis_ranges_pass() {
        let testable = test_able("cabinvfbeta/CabinVFBeta.ttf");
        let result = run_check(axis_ranges_correct, testable);
        assert_pass(&result);
    }

    #[test]
    fn test_axis_ranges_unusual_slnt() {
        let testable = test_able("varfont/inter/Inter[slnt,wght].ttf");
        let result = run_check(axis_ranges_correct, testable);
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("unusual-slnt-range".to_string()),
        );
    }

    #[test]
    fn test_fail_wght_out_of_range_zero() {
        use fontations::{
            skrifa::raw::TableProvider,
            write::{from_obj::ToOwnedTable, tables::fvar::Fvar, FontBuilder},
        };
        use fontspector_checkapi::{FileTypeConvert, TTF};

        let mut testable = test_able("cabinvfbeta/CabinVFBeta.ttf");
        let f = TTF.from_testable(&testable).unwrap();
        let mut fvar: Fvar = f.font().fvar().unwrap().to_owned_table();
        // CabinVFBeta axes: 0=wght, 1=wdth. Set wght to 0.0 for instance 0.
        fvar.axis_instance_arrays.instances[0].coordinates[0] =
            fontations::write::types::Fixed::from_f64(0.0);
        let new_bytes = FontBuilder::new()
            .add_table(&fvar)
            .unwrap()
            .copy_missing_tables(f.font())
            .build();
        testable.contents = new_bytes;
        let result = run_check(axis_ranges_correct, testable);
        assert_results_contain(
            &result,
            StatusCode::Fail,
            Some("wght-out-of-range".to_string()),
        );
    }

    #[test]
    fn test_fail_wght_out_of_range_1001() {
        use fontations::{
            skrifa::raw::TableProvider,
            write::{from_obj::ToOwnedTable, tables::fvar::Fvar, FontBuilder},
        };
        use fontspector_checkapi::{FileTypeConvert, TTF};

        let mut testable = test_able("cabinvfbeta/CabinVFBeta.ttf");
        let f = TTF.from_testable(&testable).unwrap();
        let mut fvar: Fvar = f.font().fvar().unwrap().to_owned_table();
        // CabinVFBeta axes: 0=wght, 1=wdth. Set wght to 1001.0 for instance 0.
        fvar.axis_instance_arrays.instances[0].coordinates[0] =
            fontations::write::types::Fixed::from_f64(1001.0);
        let new_bytes = FontBuilder::new()
            .add_table(&fvar)
            .unwrap()
            .copy_missing_tables(f.font())
            .build();
        testable.contents = new_bytes;
        let result = run_check(axis_ranges_correct, testable);
        assert_results_contain(
            &result,
            StatusCode::Fail,
            Some("wght-out-of-range".to_string()),
        );
    }

    #[test]
    fn test_fail_wdth_out_of_range() {
        use fontations::{
            skrifa::raw::TableProvider,
            write::{from_obj::ToOwnedTable, tables::fvar::Fvar, FontBuilder},
        };
        use fontspector_checkapi::{FileTypeConvert, TTF};

        let mut testable = test_able("cabinvfbeta/CabinVFBeta.ttf");
        let f = TTF.from_testable(&testable).unwrap();
        let mut fvar: Fvar = f.font().fvar().unwrap().to_owned_table();
        // CabinVFBeta axes: 0=wght, 1=wdth. Set wdth to 0.0 for instance 0.
        fvar.axis_instance_arrays.instances[0].coordinates[1] =
            fontations::write::types::Fixed::from_f64(0.0);
        let new_bytes = FontBuilder::new()
            .add_table(&fvar)
            .unwrap()
            .copy_missing_tables(f.font())
            .build();
        testable.contents = new_bytes;
        let result = run_check(axis_ranges_correct, testable);
        assert_results_contain(
            &result,
            StatusCode::Fail,
            Some("wdth-out-of-range".to_string()),
        );
    }

    #[test]
    fn test_warn_wdth_greater_than_1000() {
        use fontations::{
            skrifa::raw::TableProvider,
            write::{from_obj::ToOwnedTable, tables::fvar::Fvar, FontBuilder},
        };
        use fontspector_checkapi::{FileTypeConvert, TTF};

        let mut testable = test_able("cabinvfbeta/CabinVFBeta.ttf");
        let f = TTF.from_testable(&testable).unwrap();
        let mut fvar: Fvar = f.font().fvar().unwrap().to_owned_table();
        // CabinVFBeta axes: 0=wght, 1=wdth. Set wdth to 1001.0 for instance 0.
        fvar.axis_instance_arrays.instances[0].coordinates[1] =
            fontations::write::types::Fixed::from_f64(1001.0);
        let new_bytes = FontBuilder::new()
            .add_table(&fvar)
            .unwrap()
            .copy_missing_tables(f.font())
            .build();
        testable.contents = new_bytes;
        let result = run_check(axis_ranges_correct, testable);
        assert_results_contain(
            &result,
            StatusCode::Warn,
            Some("wdth-greater-than-1000".to_string()),
        );
    }

    #[test]
    fn test_pass_slnt_fixed() {
        use fontations::{
            skrifa::raw::TableProvider,
            write::{from_obj::ToOwnedTable, tables::fvar::Fvar, FontBuilder},
        };
        use fontspector_checkapi::{FileTypeConvert, TTF};

        let mut testable = test_able("varfont/inter/Inter[slnt,wght].ttf");
        let f = TTF.from_testable(&testable).unwrap();
        let mut fvar: Fvar = f.font().fvar().unwrap().to_owned_table();
        // Fix the slnt axis range by flipping min/max
        for axis in &mut fvar.axis_instance_arrays.axes {
            if axis.axis_tag == fontations::write::types::Tag::new(b"slnt") {
                let min = axis.min_value;
                let max = axis.max_value;
                axis.min_value = fontations::write::types::Fixed::from_f64(-(max.to_f64()));
                axis.max_value = fontations::write::types::Fixed::from_f64(-(min.to_f64()));
            }
        }
        let new_bytes = FontBuilder::new()
            .add_table(&fvar)
            .unwrap()
            .copy_missing_tables(f.font())
            .build();
        testable.contents = new_bytes;
        let result = run_check(axis_ranges_correct, testable);
        assert_pass(&result);
    }
}
