//! File output — writes RON strings to disk.
//!
//! 文件输出 — 将 RON 字符串写入磁盘。

use crate::component_host::GeneratedRonFile;
use anyhow::{Context, Result, anyhow};
use globset::{Glob, GlobMatcher};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ProjectFile {
    content_library: ContentLibraryConfig,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct ContentLibraryConfig {
    managed_paths: Vec<String>,
}

#[derive(Debug)]
struct ManagedOutputPolicy {
    project_root: PathBuf,
    patterns: Vec<String>,
    matchers: Vec<GlobMatcher>,
}

impl ManagedOutputPolicy {
    fn load(project_root: &Path) -> Result<Option<Self>> {
        let mod_toml = project_root.join("mod.toml");
        if !mod_toml.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&mod_toml)
            .with_context(|| format!("failed to read project file: {}", mod_toml.display()))?;
        let parsed: ProjectFile = toml::from_str(&contents)
            .with_context(|| format!("failed to parse project file: {}", mod_toml.display()))?;

        let mut matchers = Vec::with_capacity(parsed.content_library.managed_paths.len());
        for pattern in &parsed.content_library.managed_paths {
            let matcher = Glob::new(pattern)
                .with_context(|| format!("invalid managed_paths glob: {pattern}"))?
                .compile_matcher();
            matchers.push(matcher);
        }

        Ok(Some(Self {
            project_root: project_root.to_path_buf(),
            patterns: parsed.content_library.managed_paths,
            matchers,
        }))
    }

    fn validate_generated_paths(&self, files: &[GeneratedRonFile]) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }
        if self.matchers.is_empty() {
            return Err(anyhow!(
                "project {} has generated files but content_library.managed_paths is empty",
                self.project_root.display()
            ));
        }

        for file in files {
            let relative = normalized_relative_path(&file.path)?;
            let owned = self
                .matchers
                .iter()
                .any(|matcher| matcher.is_match(&relative));
            if !owned {
                return Err(anyhow!(
                    "generated file '{}' is outside content_library.managed_paths",
                    relative
                ));
            }
        }

        Ok(())
    }

    fn prune_stale_files(&self, files: &[GeneratedRonFile]) -> Result<()> {
        if self.patterns.is_empty() {
            return Ok(());
        }

        let expected_paths: BTreeSet<String> = files
            .iter()
            .map(|file| normalized_relative_path(&file.path))
            .collect::<Result<_>>()?;

        for pattern in &self.patterns {
            self.prune_pattern(pattern, &expected_paths)?;
        }

        Ok(())
    }

    fn prune_pattern(&self, pattern: &str, expected_paths: &BTreeSet<String>) -> Result<()> {
        let absolute_pattern = self.project_root.join(pattern);
        let glob_pattern = absolute_pattern.to_string_lossy().into_owned();
        for entry in glob::glob(&glob_pattern)
            .with_context(|| format!("invalid glob while pruning stale files: {pattern}"))?
        {
            let path = entry.with_context(|| {
                format!("failed to enumerate managed path for pattern {pattern}")
            })?;
            if !path.is_file() {
                continue;
            }

            let relative = self.relative_managed_path(&path)?;
            if expected_paths.contains(&relative) {
                continue;
            }

            fs::remove_file(&path).with_context(|| {
                format!("failed to remove stale managed file: {}", path.display())
            })?;
        }
        Ok(())
    }

    fn relative_managed_path(&self, path: &Path) -> Result<String> {
        Ok(path
            .strip_prefix(&self.project_root)
            .with_context(|| {
                format!(
                    "managed path '{}' is not under project root '{}'",
                    path.display(),
                    self.project_root.display()
                )
            })?
            .to_string_lossy()
            .replace('\\', "/"))
    }
}

fn normalized_relative_path(path: &Path) -> Result<String> {
    if path.is_absolute() {
        return Err(anyhow!(
            "generated path must be relative: {}",
            path.display()
        ));
    }

    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(anyhow!(
                    "generated path must not contain parent traversal: {}",
                    path.display()
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!(
                    "generated path must be relative: {}",
                    path.display()
                ));
            }
        }
    }

    if parts.is_empty() {
        return Err(anyhow!("generated path must not be empty"));
    }

    Ok(parts.join("/"))
}

/// Write generated files emitted by a Vessel content component.
///
/// 写入 Vessel 内容组件发射的生成文件。
pub fn write_generated_files(files: &[GeneratedRonFile], output_dir: &Path) -> Result<()> {
    if let Some(policy) = ManagedOutputPolicy::load(output_dir)? {
        policy.validate_generated_paths(files)?;
        policy.prune_stale_files(files)?;
    }

    for file in files {
        let relative = normalized_relative_path(&file.path)?;
        let full = output_dir.join(&relative);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
        }
        fs::write(&full, &file.ron_text)
            .with_context(|| format!("failed to write: {}", full.display()))?;
    }
    Ok(())
}
