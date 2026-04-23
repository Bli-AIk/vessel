use std::fs;
use std::path::PathBuf;
use std::process::Command;
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

    let component_path = fixture_dir.join("target/wasm32-wasip2/debug/vessel_test_guest.wasm");
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

fn generated_file(path: impl Into<PathBuf>) -> vessel::GeneratedRonFile {
    vessel::GeneratedRonFile {
        path: path.into(),
        ron_text: "(value: 1)\n".to_owned(),
    }
}

#[test]
fn builds_generated_ron_from_wasm_component() {
    let component_path = build_fixture_guest();
    let output_dir = temp_output_dir("vessel_component_build");

    let summary = vessel::build_component(&component_path, &output_dir)
        .expect("vessel host should build files from the wasm component");

    assert_eq!(
        summary.written_files, 1,
        "expected exactly one generated file"
    );

    let output_path = output_dir.join("example/test.ron");
    assert!(output_path.exists(), "generated ron file should exist");

    let content = fs::read_to_string(&output_path).expect("generated file should be readable");
    assert!(
        content.contains("BOOTSTRAPPED BY VESSEL"),
        "generated file should include the default vessel bootstrap header"
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
    let output_dir = temp_output_dir("vessel_component_manifest");

    fs::create_dir_all(output_dir.join("example")).expect("should create example directory");
    fs::write(output_dir.join("example/stale.ron"), "(stale: true)\n")
        .expect("should write stale managed file");
    fs::create_dir_all(output_dir.join(".build")).expect("should create build directory");
    fs::write(
        output_dir.join(".build/vessel-output-manifest.toml"),
        r#"version = 1
owned_paths = ["example/stale.ron", "example/test.ron"]
"#,
    )
    .expect("should write previous output manifest");

    vessel::build_component(&component_path, &output_dir)
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
    let output_dir = temp_output_dir("vessel_component_rejects_tooling_roots");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    let err =
        vessel::write_generated_files(&[generated_file("content/ron/forbidden.ron")], &output_dir)
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
    let output_dir = temp_output_dir("vessel_component_rejects_non_ron");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    let err = vessel::write_generated_files(&[generated_file("battle/not_ron.txt")], &output_dir)
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
    let output_dir = temp_output_dir("vessel_component_duplicate_paths");

    fs::create_dir_all(&output_dir).expect("should create output directory");

    let err = vessel::write_generated_files(
        &[
            vessel::GeneratedRonFile {
                path: PathBuf::from("example/duplicate.ron"),
                ron_text: "(value: 1)\n".to_owned(),
            },
            vessel::GeneratedRonFile {
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
        let output_dir = temp_output_dir("vessel_component_rejects_unsafe_path");
        fs::create_dir_all(&output_dir).expect("should create output directory");
        let err = vessel::write_generated_files(&[generated_file(path)], &output_dir)
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
    let output_dir = temp_output_dir("vessel_component_rejects_unmanaged_existing");

    fs::create_dir_all(output_dir.join("example")).expect("should create example directory");
    fs::write(output_dir.join("example/existing.ron"), "(manual: true)\n")
        .expect("should write unmanaged file");

    let err = vessel::write_generated_files(&[generated_file("example/existing.ron")], &output_dir)
        .expect_err("host should reject overwriting unmanaged files");
    let err_text = err.to_string();
    assert!(
        err_text.contains("is not managed by Vessel manifest"),
        "unexpected error: {err_text}"
    );

    let preserved = fs::read_to_string(output_dir.join("example/existing.ron"))
        .expect("unmanaged file should still be readable");
    assert_eq!(preserved, "(manual: true)\n");

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn respects_custom_generated_file_header_from_mod_toml() {
    let output_dir = temp_output_dir("vessel_component_custom_header");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    fs::write(
        output_dir.join("mod.toml"),
        r#"[content_library]
generated_file_header = "// custom header\n// generated for tests"
"#,
    )
    .expect("should write mod.toml");

    vessel::write_generated_files(
        &[vessel::GeneratedRonFile {
            path: PathBuf::from("example/custom.ron"),
            ron_text: "(value: 1)\n".to_owned(),
        }],
        &output_dir,
    )
    .expect("host should write file with custom header");

    let content = fs::read_to_string(output_dir.join("example/custom.ron"))
        .expect("generated file should be readable");
    assert!(
        content.starts_with("// custom header\n// generated for tests\n\n"),
        "custom header should be prepended verbatim: {content}"
    );
    assert!(
        !content.contains("BOOTSTRAPPED BY VESSEL"),
        "custom header should replace the default header"
    );

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn allows_disabling_generated_file_header_via_empty_override() {
    let output_dir = temp_output_dir("vessel_component_no_header");

    fs::create_dir_all(&output_dir).expect("should create output directory");
    fs::write(
        output_dir.join("mod.toml"),
        r#"[content_library]
generated_file_header = ""
"#,
    )
    .expect("should write mod.toml");

    vessel::write_generated_files(
        &[vessel::GeneratedRonFile {
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
