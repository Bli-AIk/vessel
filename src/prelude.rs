//! Re-exports for convenient use.
//!
//! 重导出模块

#[cfg(feature = "host")]
pub use crate::component_host::{
    BuildSummary, GeneratedRonFile, build_component, build_component_with_options,
    load_component_files,
};
#[cfg(feature = "guest")]
pub use crate::guest;
#[cfg(feature = "host")]
pub use crate::{
    WriteGeneratedFilesOptions, write_generated_files, write_generated_files_with_options,
};
