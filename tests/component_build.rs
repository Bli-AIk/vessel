use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal_guest")
}

fn build_fixture_guest() -> PathBuf {
    let fixture_dir = fixture_dir();

    let status = Command::new("cargo")
        .current_dir(&fixture_dir)
        .args(["build", "--target", "wasm32-wasip2"])
        .status()
        .expect("failed to invoke cargo build for fixture guest");

    assert!(status.success(), "fixture guest should build successfully");

    let component_path = fixture_dir.join("target/wasm32-wasip2/debug/cauld-ron_test_guest.wasm");
    assert!(
        component_path.exists(),
        "fixture wasm component should exist"
    );

    component_path
}

fn temp_output_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let output_dir = std::env::temp_dir().join(format!("{prefix}_{unique}"));
    let _ = fs::remove_dir_all(&output_dir);
    output_dir
}

fn generated_file(path: impl Into<PathBuf>) -> cauld_ron::GeneratedRonFile {
    cauld_ron::GeneratedRonFile {
        path: path.into(),
        ron_text: "(value: 1)\n".to_owned(),
    }
}

#[test]
fn builds_generated_ron_from_wasm_component() {
    let component_path = build_fixture_guest();
    let output_dir = temp_output_dir("cauld-ron_component_build");

    let summary = cauld_ron::build_component(&component_path, &output_dir)
        .expect("cauld-ron host should build files from the wasm component");

    assert_eq!(
        summary.written_files, 1,
        "expected exactly one generated file"
    );

    let output_path = output_dir.join("example/test.ron");
    assert!(output_path.exists(), "generated ron file should exist");

    let content = fs::read_to_string(&output_path).expect("generated file should be readable");
    assert!(
        content.contains("BOOTSTRAPPED BY CAULD-RON"),
        "generated file should include the default cauld-ron bootstrap header"
    );
    assert!(
        content.contains("// Generated at: "),
        "generated file should include a human-readable generation timestamp"
    );
    assert!(
        content.contains("fixture"),
        "generated file should contain fixture payload"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn prunes_stale_files_from_previous_output_manifest() {
    let component_path = build_fixture_guest();
    let output_dir = temp_output_dir("cauld-ron_component_manifest");

    fs::create_dir_all(output_dir.join("example")).expect("should create example directory");
    fs::write(output_dir.join("example/stale.ron"), "(stale: true)\n")
        .expect("should write stale managed file");
    fs::create_dir_all(output_dir.join(".build")).expect("should create build directory");
    fs::write(
        output_dir.join(".build/cauld-ron-output-manifest.toml"),
        r#"version = 1
owned_paths = ["example/stale.ron", "example/test.ron"]
"#,
    )
    .expect("should write previous output manifest");

    cauld_ron::build_component(&component_path, &output_dir)
        .expect("manifest-managed output should be generated successfully");

    assert!(
        output_dir.join("example/test.ron").exists(),
        "fresh generated file should exist"
    );
    assert!(
        !output_dir.join("example/stale.ron").exists(),
        "stale generated file should be pruned"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn rejects_generated_files_inside_tooling_directories() {
    let output_dir = temp_output_dir("cauld-ron_component_rejects_tooling_roots");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    let err = cauld_ron::write_generated_files(
        &[generated_file("content/ron/forbidden.ron")],
        &output_dir,
    )
    .expect_err("host should reject generated files inside tooling roots");
    let err_text = err.to_string();
    assert!(
        err_text.contains("not allowed under tooling/source root"),
        "unexpected error: {err_text}"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn rejects_generated_files_without_ron_extension() {
    let output_dir = temp_output_dir("cauld-ron_component_rejects_non_ron");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    let err =
        cauld_ron::write_generated_files(&[generated_file("battle/not_ron.txt")], &output_dir)
            .expect_err("host should reject non-RON output paths");
    let err_text = err.to_string();
    assert!(
        err_text.contains("must end with .ron"),
        "unexpected error: {err_text}"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn rejects_duplicate_generated_paths() {
    let output_dir = temp_output_dir("cauld-ron_component_duplicate_paths");

    fs::create_dir_all(&output_dir).expect("should create output directory");

    let err = cauld_ron::write_generated_files(
        &[
            cauld_ron::GeneratedRonFile {
                path: PathBuf::from("example/duplicate.ron"),
                ron_text: "(value: 1)\n".to_owned(),
            },
            cauld_ron::GeneratedRonFile {
                path: PathBuf::from("./example/duplicate.ron"),
                ron_text: "(value: 2)\n".to_owned(),
            },
        ],
        &output_dir,
    )
    .expect_err("host should reject duplicate output paths");
    let err_text = err.to_string();
    assert!(
        err_text.contains("was emitted more than once"),
        "unexpected error: {err_text}"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn rejects_unsafe_generated_paths() {
    let cases = [
        (PathBuf::from("/tmp/escape.ron"), "must be relative"),
        (
            PathBuf::from("../escape.ron"),
            "must not contain parent traversal",
        ),
        (PathBuf::from(""), "must not be empty"),
        (
            PathBuf::from("C:/escape.ron"),
            "must not contain a Windows drive prefix",
        ),
        (
            PathBuf::from("windows\\escape.ron"),
            "must use forward slashes",
        ),
    ];

    for (path, expected_message) in cases {
        let output_dir = temp_output_dir("cauld-ron_component_rejects_unsafe_path");
        fs::create_dir_all(&output_dir).expect("should create output directory");
        let err = cauld_ron::write_generated_files(&[generated_file(path)], &output_dir)
            .expect_err("host should reject unsafe output path");
        let err_text = err.to_string();
        assert!(
            err_text.contains(expected_message),
            "unexpected error for {expected_message}: {err_text}"
        );
        let _ = fs::remove_dir_all(&output_dir);
    }
}

#[test]
fn rejects_overwriting_existing_files_not_owned_by_manifest() {
    let output_dir = temp_output_dir("cauld-ron_component_rejects_unmanaged_existing");

    fs::create_dir_all(output_dir.join("example")).expect("should create example directory");
    fs::write(output_dir.join("example/existing.ron"), "(manual: true)\n")
        .expect("should write unmanaged file");

    let err =
        cauld_ron::write_generated_files(&[generated_file("example/existing.ron")], &output_dir)
            .expect_err("host should reject overwriting unmanaged files");
    let err_text = err.to_string();
    assert!(
        err_text.contains("is not managed by Cauld-ron manifest"),
        "unexpected error: {err_text}"
    );

    let preserved = fs::read_to_string(output_dir.join("example/existing.ron"))
        .expect("unmanaged file should still be readable");
    assert_eq!(preserved, "(manual: true)\n");

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn respects_custom_generated_file_header_from_mod_toml() {
    let output_dir = temp_output_dir("cauld-ron_component_custom_header");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    fs::write(
        output_dir.join("mod.toml"),
        r#"[content_library]
generated_file_header = "// custom header\n// generated for tests"
"#,
    )
    .expect("should write mod.toml");

    cauld_ron::write_generated_files(
        &[cauld_ron::GeneratedRonFile {
            path: PathBuf::from("example/custom.ron"),
            ron_text: "(value: 1)\n".to_owned(),
        }],
        &output_dir,
    )
    .expect("host should write file with custom header");

    let content = fs::read_to_string(output_dir.join("example/custom.ron"))
        .expect("generated file should be readable");
    assert!(
        content.starts_with("// custom header\n// generated for tests\n// Generated at: "),
        "custom header should receive a generated-at line: {content}"
    );
    assert!(
        !content.contains("BOOTSTRAPPED BY CAULD-RON"),
        "custom header should replace the default header"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn allows_disabling_generated_file_header_via_empty_override() {
    let output_dir = temp_output_dir("cauld-ron_component_no_header");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    fs::write(
        output_dir.join("mod.toml"),
        r#"[content_library]
generated_file_header = ""
"#,
    )
    .expect("should write mod.toml");

    cauld_ron::write_generated_files(
        &[cauld_ron::GeneratedRonFile {
            path: PathBuf::from("example/plain.ron"),
            ron_text: "(value: 1)\n".to_owned(),
        }],
        &output_dir,
    )
    .expect("host should write file without header");

    let content = fs::read_to_string(output_dir.join("example/plain.ron"))
        .expect("generated file should be readable");
    assert_eq!(
        content, "(value: 1)\n",
        "empty override should disable the generated header"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn preserves_existing_timestamp_when_content_does_not_change() {
    let output_dir = temp_output_dir("cauld-ron_component_stable_timestamp");

    fs::create_dir_all(&output_dir).expect("should create output directory");

    let files = [cauld_ron::GeneratedRonFile {
        path: PathBuf::from("example/stable.ron"),
        ron_text: "(value: 1)\n".to_owned(),
    }];

    cauld_ron::write_generated_files(&files, &output_dir)
        .expect("host should write generated file on first pass");
    let output_path = output_dir.join("example/stable.ron");
    let first = fs::read_to_string(&output_path).expect("first generated file should be readable");

    thread::sleep(Duration::from_millis(1100));

    cauld_ron::write_generated_files(&files, &output_dir)
        .expect("host should allow regenerating unchanged output");
    let second =
        fs::read_to_string(&output_path).expect("second generated file should be readable");

    assert_eq!(
        second, first,
        "unchanged output should preserve the existing timestamp and avoid churn"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn replaces_changed_existing_body_without_semantic_hook() {
    let output_dir = temp_output_dir("cauld-ron_component_no_semantic_hook");

    fs::create_dir_all(output_dir.join("example")).expect("should create output directory");
    fs::create_dir_all(output_dir.join(".build")).expect("should create build directory");
    fs::write(
        output_dir.join(".build/cauld-ron-output-manifest.toml"),
        r#"version = 1
owned_paths = ["example/changed.ron"]
"#,
    )
    .expect("should write output manifest");
    fs::write(
        output_dir.join("example/changed.ron"),
        r#"// =============================================================================
// BOOTSTRAPPED BY CAULD-RON
// =============================================================================
// This file was initially generated by Cauld-ron to provide a baseline structure.
//
// [ MANUAL EDITS ALLOWED - PROCEED WITH CAUTION ]
// You can modify this file to suit your specific needs. However,
// be aware that regenerating this module via Cauld-ron will OVERWRITE this file.
//
// GUIDELINES FOR MODIFICATION:
// 1. SAFETY FIRST: Ensure your current state is committed to version control
//    (e.g., Git) before making changes, so you can easily revert or handle
//    future merge conflicts.
// 2. THE "PROPER" WAY: If your goal is to change the underlying logic or
//    structure globally, please edit the Cauld-ron source schema/configuration
//    instead of modifying this file directly.
// =============================================================================
// Generated at: 2000-01-01 00:00:00 +00:00

(value: "manual")
"#,
    )
    .expect("should write existing output");

    cauld_ron::write_generated_files(
        &[cauld_ron::GeneratedRonFile {
            path: PathBuf::from("example/changed.ron"),
            ron_text: "(value: \"generated\")\n".to_owned(),
        }],
        &output_dir,
    )
    .expect("host should write generated body without semantic hook");

    let content = fs::read_to_string(output_dir.join("example/changed.ron"))
        .expect("generated file should be readable");
    assert!(
        content.contains("(value: \"generated\")"),
        "generated body should replace changed existing body: {content}"
    );
    assert!(
        !content.contains("(value: \"manual\")"),
        "manual body should not be preserved without a semantic hook: {content}"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn semantic_hook_preserves_existing_body_and_refreshes_timestamp() {
    let output_dir = temp_output_dir("cauld-ron_component_semantic_hook");

    fs::create_dir_all(output_dir.join("example")).expect("should create output directory");
    fs::create_dir_all(output_dir.join(".build")).expect("should create build directory");
    fs::write(
        output_dir.join(".build/cauld-ron-output-manifest.toml"),
        r#"version = 1
owned_paths = ["example/equivalent.ron"]
"#,
    )
    .expect("should write output manifest");
    fs::write(
        output_dir.join("example/equivalent.ron"),
        r#"// =============================================================================
// BOOTSTRAPPED BY CAULD-RON
// =============================================================================
// This file was initially generated by Cauld-ron to provide a baseline structure.
//
// [ MANUAL EDITS ALLOWED - PROCEED WITH CAUTION ]
// You can modify this file to suit your specific needs. However,
// be aware that regenerating this module via Cauld-ron will OVERWRITE this file.
//
// GUIDELINES FOR MODIFICATION:
// 1. SAFETY FIRST: Ensure your current state is committed to version control
//    (e.g., Git) before making changes, so you can easily revert or handle
//    future merge conflicts.
// 2. THE "PROPER" WAY: If your goal is to change the underlying logic or
//    structure globally, please edit the Cauld-ron source schema/configuration
//    instead of modifying this file directly.
// =============================================================================
// Generated at: 2000-01-01 00:00:00 +00:00

(value: "manual")
"#,
    )
    .expect("should write existing output");

    let options = cauld_ron::WriteGeneratedFilesOptions {
        semantic_equal: Some(&|relative_path, existing_body, generated_body| {
            relative_path == "example/equivalent.ron"
                && existing_body.contains("(value: \"manual\")")
                && generated_body.contains("(value: \"generated\")")
        }),
    };

    cauld_ron::write_generated_files_with_options(
        &[cauld_ron::GeneratedRonFile {
            path: PathBuf::from("example/equivalent.ron"),
            ron_text: "(value: \"generated\")\n".to_owned(),
        }],
        &output_dir,
        options,
    )
    .expect("host should preserve existing body when semantic hook matches");

    let content = fs::read_to_string(output_dir.join("example/equivalent.ron"))
        .expect("generated file should be readable");
    assert!(
        !content.contains("2000-01-01 00:00:00 +00:00"),
        "timestamp should refresh"
    );
    assert!(
        content.contains("(value: \"manual\")"),
        "existing body should be preserved: {content}"
    );
    assert!(
        !content.contains("(value: \"generated\")"),
        "generated body should not replace semantically equal existing body: {content}"
    );

    let _ = fs::remove_dir_all(&output_dir);
}
