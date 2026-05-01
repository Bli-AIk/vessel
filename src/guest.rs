//! Guest-side WIT bindings for Cauld-ron content modules.
//!
//! Cauld-ron 内容模块的 guest 侧 WIT 绑定。

wit_bindgen::generate!({
    path: "wit",
    world: "content-module",
    pub_export_macro: true,
});

/// Re-exports of generated WIT guest-side items.
///
/// WIT guest 侧生成项的重导出。
pub mod wit {
    pub use super::Guest;
    pub use super::cauld_ron::build::types::GeneratedFile;
    pub use super::export;
}
