use super::load_graph_source_snippet;

#[test]
fn graph_source_snippet_reads_context_only_inside_the_selected_project(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "enforcer-graph-source-snippet-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(
        root.join("src/lib.rs"),
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\n",
    )?;

    let snippet =
        load_graph_source_snippet(root.display().to_string(), "src/lib.rs".to_owned(), 5)?;

    assert_eq!(snippet.start_line, 2);
    assert_eq!(snippet.end_line, 8);
    assert_eq!(
        snippet.content,
        "    2 | two\n    3 | three\n    4 | four\n    5 | five\n    6 | six\n    7 | seven\n    8 | eight"
    );
    let error =
        load_graph_source_snippet(root.display().to_string(), "../outside.rs".to_owned(), 1)
            .err()
            .ok_or("parent path must be rejected")?;
    assert_eq!(error, "graph source path must be project-relative");
    std::fs::remove_dir_all(root)?;
    Ok(())
}
