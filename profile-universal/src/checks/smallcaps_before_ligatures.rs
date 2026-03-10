use fontspector_checkapi::{prelude::*, skip, testfont, FileTypeConvert};

#[check(
    id = "smallcaps_before_ligatures",
    rationale = "
        OpenType small caps should be defined before ligature lookups to ensure
        proper functionality.

        Rainer Erich Scheichelbauer (a.k.a. MekkaBlue) pointed out in a tweet
        (https://twitter.com/mekkablue/status/1297486769668132865) that the ordering
        of small caps and ligature lookups can lead to bad results such as the example
        he provided of the word \"WAFFLES\" in small caps, but with an unfortunate
        lowercase ffl ligature substitution.
	
        This check attempts to detect this kind of mistake.",
    proposal = "https://github.com/fonttools/fontbakery/issues/3020",
    title = "Ensure 'smcp' (small caps) lookups are defined before ligature lookups in the 'GSUB' table."
)]
fn smallcaps_before_ligatures(t: &Testable, _context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    // Skip if no smcp
    let smcp_lookups = f
        .feature_records(true)
        .filter(|(r, _l)| r.feature_tag() == "smcp")
        .flat_map(|(_r, l)| l)
        .flat_map(|l| l.lookup_list_indices())
        .collect::<Vec<_>>();
    let liga_lookups = f
        .feature_records(true)
        .filter(|(r, _l)| r.feature_tag() == "liga")
        .flat_map(|(_r, l)| l)
        .flat_map(|l| l.lookup_list_indices())
        .collect::<Vec<_>>();
    skip!(smcp_lookups.is_empty(), "no-smcp", "No smcp feature");
    skip!(liga_lookups.is_empty(), "no-liga", "No liga feature");
    #[allow(clippy::unwrap_used)] // We know that the vecs are not empty
    let first_smcp_lookup = smcp_lookups.iter().min().unwrap();
    #[allow(clippy::unwrap_used)] // We know that the vecs are not empty
    let first_liga_lookup = liga_lookups.iter().min().unwrap();
    if first_smcp_lookup < first_liga_lookup {
        return Ok(Status::just_one_pass());
    }
    return Ok(Status::just_one_fail(
        "feature-ordering",
        "'smcp' lookups are not defined before 'liga' lookups.",
    ));
}

#[cfg(test)]
mod tests {
    use fontspector_checkapi::{
        codetesting::{assert_pass, assert_results_contain, run_check, test_able},
        StatusCode,
    };

    use fontations::skrifa::{font::FontRef, Tag};
    use fontations::write::FontBuilder;

    /// Replace the GSUB table in a font with a custom one
    fn replace_gsub(testable: &mut fontspector_checkapi::prelude::Testable, gsub_data: &[u8]) {
        let gsub_tag = Tag::new(b"GSUB");
        let f = FontRef::new(&testable.contents).unwrap();
        let mut builder = FontBuilder::new();
        builder.add_raw(gsub_tag, gsub_data);
        for table_record in f.table_directory.table_records() {
            let tag = table_record.tag.get();
            if tag != gsub_tag {
                if let Some(table_data) = f.table_data(tag) {
                    builder.add_raw(tag, table_data);
                }
            }
        }
        testable.contents = builder.build();
    }

    #[test]
    fn test_smallcaps_before_ligatures_skip_no_smcp() {
        // Mada Regular has no 'smcp' feature, so check should SKIP
        let testable = test_able("mada/Mada-Regular.ttf");
        let results = run_check(super::smallcaps_before_ligatures, testable);
        assert_results_contain(&results, StatusCode::Skip, Some("no-smcp".to_string()));
    }

    #[test]
    fn test_smallcaps_before_ligatures_pass() {
        // Build a GSUB table with smcp (lookup 0) before liga (lookup 1) => PASS
        #[rustfmt::skip]
        const GSUB_SMCP_BEFORE_LIGA: &[u8] = &[
            0x00, 0x01, 0x00, 0x00, // version 1.0
            0x00, 0x0a,             // ScriptList offset
            0x00, 0x20,             // FeatureList offset
            0x00, 0x3a,             // LookupList offset
            // ScriptList
            0x00, 0x01,             // scriptCount = 1
            0x44, 0x46, 0x4c, 0x54, // 'DFLT'
            0x00, 0x08,             // offset to Script table
            // Script table
            0x00, 0x04,             // defaultLangSysOffset
            0x00, 0x00,             // langSysCount = 0
            // Default LangSys
            0x00, 0x00,             // lookupOrder
            0xff, 0xff,             // reqFeatureIndex
            0x00, 0x02,             // featureIndexCount = 2
            0x00, 0x00, 0x00, 0x01, // featureIndices [0, 1]
            // FeatureList
            0x00, 0x02,             // featureCount = 2
            0x73, 0x6d, 0x63, 0x70, // 'smcp'
            0x00, 0x0e,             // offset to Feature[0]
            0x6c, 0x69, 0x67, 0x61, // 'liga'
            0x00, 0x14,             // offset to Feature[1]
            // Feature[0] (smcp) - lookupListIndex = [0]
            0x00, 0x00,             // featureParams = NULL
            0x00, 0x01,             // lookupCount = 1
            0x00, 0x00,             // lookupListIndex[0] = 0
            // Feature[1] (liga) - lookupListIndex = [1]
            0x00, 0x00,             // featureParams = NULL
            0x00, 0x01,             // lookupCount = 1
            0x00, 0x01,             // lookupListIndex[0] = 1
            // LookupList
            0x00, 0x02,             // lookupCount = 2
            0x00, 0x06,             // offset to Lookup[0]
            0x00, 0x0c,             // offset to Lookup[1]
            // Lookup[0] (for smcp)
            0x00, 0x01,             // lookupType = 1 (Single)
            0x00, 0x00,             // lookupFlag
            0x00, 0x00,             // subTableCount = 0
            // Lookup[1] (for liga)
            0x00, 0x04,             // lookupType = 4 (Ligature)
            0x00, 0x00,             // lookupFlag
            0x00, 0x00,             // subTableCount = 0
        ];

        let mut testable = test_able("mada/Mada-Regular.ttf");
        replace_gsub(&mut testable, GSUB_SMCP_BEFORE_LIGA);
        let results = run_check(super::smallcaps_before_ligatures, testable);
        assert_pass(&results);
    }

    #[test]
    fn test_smallcaps_before_ligatures_fail() {
        // Build a GSUB table with liga (lookup 0) before smcp (lookup 1) => FAIL
        #[rustfmt::skip]
        const GSUB_LIGA_BEFORE_SMCP: &[u8] = &[
            0x00, 0x01, 0x00, 0x00, // version 1.0
            0x00, 0x0a,             // ScriptList offset
            0x00, 0x20,             // FeatureList offset
            0x00, 0x3a,             // LookupList offset
            // ScriptList
            0x00, 0x01,             // scriptCount = 1
            0x44, 0x46, 0x4c, 0x54, // 'DFLT'
            0x00, 0x08,             // offset to Script table
            // Script table
            0x00, 0x04,             // defaultLangSysOffset
            0x00, 0x00,             // langSysCount = 0
            // Default LangSys
            0x00, 0x00,             // lookupOrder
            0xff, 0xff,             // reqFeatureIndex
            0x00, 0x02,             // featureIndexCount = 2
            0x00, 0x00, 0x00, 0x01, // featureIndices [0, 1]
            // FeatureList
            0x00, 0x02,             // featureCount = 2
            0x73, 0x6d, 0x63, 0x70, // 'smcp'
            0x00, 0x0e,             // offset to Feature[0]
            0x6c, 0x69, 0x67, 0x61, // 'liga'
            0x00, 0x14,             // offset to Feature[1]
            // Feature[0] (smcp) - lookupListIndex = [1]
            0x00, 0x00,             // featureParams = NULL
            0x00, 0x01,             // lookupCount = 1
            0x00, 0x01,             // lookupListIndex[0] = 1
            // Feature[1] (liga) - lookupListIndex = [0]
            0x00, 0x00,             // featureParams = NULL
            0x00, 0x01,             // lookupCount = 1
            0x00, 0x00,             // lookupListIndex[0] = 0
            // LookupList
            0x00, 0x02,             // lookupCount = 2
            0x00, 0x06,             // offset to Lookup[0]
            0x00, 0x0c,             // offset to Lookup[1]
            // Lookup[0] (for liga)
            0x00, 0x04,             // lookupType = 4 (Ligature)
            0x00, 0x00,             // lookupFlag
            0x00, 0x00,             // subTableCount = 0
            // Lookup[1] (for smcp)
            0x00, 0x01,             // lookupType = 1 (Single)
            0x00, 0x00,             // lookupFlag
            0x00, 0x00,             // subTableCount = 0
        ];

        let mut testable = test_able("mada/Mada-Regular.ttf");
        replace_gsub(&mut testable, GSUB_LIGA_BEFORE_SMCP);
        let results = run_check(super::smallcaps_before_ligatures, testable);
        assert_results_contain(
            &results,
            StatusCode::Fail,
            Some("feature-ordering".to_string()),
        );
    }
}
