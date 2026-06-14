# Release Checklist

- Clean checkout builds.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- `cargo test --all-targets --all-features` passes.
- Examples validate.
- TUI smoke test completed.
- CLI apply smoke test completed.
- Batch output does not overwrite input.
- README checked against actual behavior.
- Known limitations updated.
- Version bumped.
- Changelog updated.
