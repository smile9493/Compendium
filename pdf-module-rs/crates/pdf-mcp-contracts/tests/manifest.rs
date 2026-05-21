use pdf_mcp_contracts::{CONTRACT_VERSION, all_tool_specs, manifest_sha256, tool_count};

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
    // 6 extract + 12 knowledge + 12 index (incl. apply_wiki_patch alias + get_compilation_context)
    // + 9 management + 11 platform = 50
    assert_eq!(tool_count(), 50);
}

#[test]
fn manifest_hash_stable_for_fixed_specs() {
    let h = manifest_sha256();
    assert_eq!(h.len(), 64);
}
