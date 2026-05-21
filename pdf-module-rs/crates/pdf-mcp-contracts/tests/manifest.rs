use pdf_mcp_contracts::{
    CONTRACT_VERSION, all_api_methods, all_tool_specs, code_mode_tool_count, code_mode_tool_specs,
    manifest_sha256, search_api, tool_count,
};

#[test]
fn contract_version_is_semver() {
    assert!(!CONTRACT_VERSION.is_empty());
}

#[test]
fn all_tools_have_output_schema() {
    for spec in all_tool_specs() {
        assert!(!spec.name.is_empty(), "tool name required");
        assert!(spec.input_schema.is_object(), "{} inputSchema", spec.name);
        assert!(spec.output_schema.is_object(), "{} outputSchema", spec.name);
    }
}

#[test]
fn tool_names_unique() {
    let specs = all_tool_specs();
    let mut names: Vec<_> = specs.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    let unique: std::collections::HashSet<_> = names.iter().copied().collect();
    assert_eq!(unique.len(), specs.len(), "duplicate tool names");
}

#[test]
fn expected_tool_count() {
    // 6 extract + 15 knowledge + 12 index + 9 management + 11 platform = 53
    assert_eq!(tool_count(), 53);
}

#[test]
fn manifest_hash_stable_for_fixed_specs() {
    let h = manifest_sha256();
    assert_eq!(h.len(), 64);
}

#[test]
fn code_mode_tool_count_is_two() {
    assert_eq!(code_mode_tool_count(), 2);
    let names: Vec<String> = code_mode_tool_specs().into_iter().map(|s| s.name).collect();
    assert!(names.iter().any(|n| n == "search_compendium_api"));
    assert!(names.iter().any(|n| n == "execute_compendium"));
}

#[test]
fn api_catalog_matches_full_tool_count() {
    assert_eq!(all_api_methods().len(), tool_count());
}

#[test]
fn search_api_returns_hits() {
    let hits = search_api("lint", 5);
    assert!(hits.iter().any(|h| h.name == "lint_wiki"));
}
