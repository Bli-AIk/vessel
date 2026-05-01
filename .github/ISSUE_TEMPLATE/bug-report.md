---
name: Bug report
about: Report a bug or unexpected behavior in cauld-ron
title: 'bug: [Short description]'
labels: bug
assignees: ''

---

### 🐛 Problem Description
Provide a clear description of the issue. Include:
- **cauld-ron version**: (e.g., crate version or commit hash)
- **Type of issue**: compile-time / runtime / output mismatch / unexpected behavior
- Short code snippet showing the context (optional, but helpful)

---

### 📝 Steps to Reproduce
Provide a minimal, self-contained example:

```rust
fn main() {
    // Your reproduction here
}
```

1. What command did you run?
2. Operating system (for example: Windows 11, Ubuntu 24.04, macOS 15)
3. Which input WASM component or output directory layout was involved?

---

### ✅ Expected Result
Describe what the correct behavior should be.

---

### 📄 Actual Result / Logs
Include any errors, stack traces, or console output:

```text
// Paste your error message here
```

---

### ⚙ Additional Information
- Does the problem reproduce with a minimal component?
- Does it only happen with specific paths, headers, or manifest state?
- Any workaround already tried?
