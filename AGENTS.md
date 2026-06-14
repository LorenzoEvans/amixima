# Codex Execution Prompt: Ship Amixima v0.1

You are working in the Amixima repository.

Your task is to take Amixima from its current prototype state to a shippable v0.1 beta-quality local tool for users. Amixima is a Rust Terminal User Interface for audio sculpting and batch processing. It lets users define a reusable sequence of audio effects called a Soundcourse, preview that chain against selected audio, save/load it as JSON-LD or INI, and apply it safely to a directory of WAV files.

Do not turn this into a broad DAW, plugin host, cloud service, or GUI rewrite. The v0.1 target is narrow, reliable, local-first, and user-safe.

## Product Target

Ship this first:

> Amixima v0.1: a local-first TUI and CLI for building reusable audio effect chains and batch-processing WAV samples safely.

The v0.1 promise:

- WAV input and WAV output for batch export.
- Preview of the selected file through the current Soundcourse.
- JSON-LD and INI Soundcourse loading/saving.
- Non-destructive batch export to an output directory.
- A small, reliable effect set: gain, EQ, delay, reverb, compressor.
- Clear errors, examples, docs, and reproducible tests.

Explicitly defer:

- VST/AU plugin support.
- Full RDF/OWL reasoning.
- GUI rewrite.
- Real-time multitrack processing.
- Cloud sync.
- MP3/FLAC/OGG export.
- Complex waveform editing.
- Preset marketplace or registry.

## Existing Project Context

The repository already contains:

- A README describing Amixima as a TUI tool for audio sculpting and batch processing.
- A blog post describing Amixima as a terminal-based audio development environment for deterministic multi-file sample processing.
- A context document describing the core user workflow: import files/directories, choose effects, configure parameters, save the effect sequence, parse it, apply it to files, and save outputs separately from originals.
- Source files including `main.rs`, `ui.rs`, `parser.rs`, and `ontology.rs`.

Important current concepts:

- `Soundcourse` is the reusable effect chain.
- `EffectNode` currently includes Reverb, EQ, Delay, Compressor, and Gain.
- The parser supports JSON-LD and INI.
- The TUI currently owns a lot of application state and workflow logic.
- Batch output should never overwrite source files.
- Preview is already part of the intended workflow.

## Core Engineering Principles

Follow these Rust and product-quality rules throughout:

- Keep the TUI thin. It should call core APIs rather than directly owning processing, parsing, persistence, and filesystem behavior.
- Keep DSP and batch processing independent from Ratatui.
- Keep parser behavior explicit and testable.
- Prefer type-driven APIs and validation over stringly typed ad hoc logic.
- Avoid `unwrap` and `expect` in user-facing execution paths.
- Preserve current behavior where it is already correct.
- Refactor incrementally so the app continues compiling after each meaningful step.
- Keep public errors actionable for users.
- Do not overwrite input files.
- Keep generated files out of source directories unless they are intentional examples, docs, or test fixtures.
- Run formatting, linting, and tests before declaring completion.

## Required Final Repository Shape

Refactor toward this structure. It does not need to be mechanically identical if the existing crate layout makes small deviations necessary, but the separation of responsibilities must be achieved.

```text
src/
  main.rs              # binary entrypoint only
  cli.rs               # Clap args/subcommands
  error.rs             # shared AmiximaError and Result
  app.rs               # TUI state machine and event dispatch
  tui/
    mod.rs
    layout.rs
    widgets.rs
    input.rs
    style.rs
  core/
    mod.rs
    soundcourse.rs     # Soundcourse, EffectNode, metadata
    validation.rs      # parameter ranges, empty chains, sample-rate checks
    presets.rs         # default effect values
  parser/
    mod.rs
    ini.rs
    jsonld.rs
  audio/
    mod.rs
    decode.rs
    encode.rs
    process.rs
    preview.rs
    peaks.rs
  batch/
    mod.rs
    job.rs
    output.rs
    report.rs
```

Also add or update:

```text
docs/
  quickstart.md
  soundcourse-v1.md
  examples.md
  troubleshooting.md
  release-checklist.md
examples/
  warm-bus.ini
  warm-bus.jsonld
  clean-gain.jsonld
  slap-delay.ini
  soft-compress.jsonld
tests/
  fixtures/
    README.md
```

## Phase 1: Establish Shared Error Handling

Create `src/error.rs`.

Requirements:

- Define a shared project result type, such as `pub type Result<T> = std::result::Result<T, AmiximaError>`.
- Prefer `thiserror` if already present or acceptable to add. Otherwise use a simple custom error type.
- Error categories should cover:
  - IO errors.
  - Parser errors.
  - Validation errors.
  - Unsupported format errors.
  - Audio decode errors.
  - Audio encode errors.
  - Processing errors.
  - Batch errors.
  - User-facing path errors.
- Preserve or bridge existing `color_eyre` usage if needed, but do not let every library module depend on terminal-reporting behavior.

Acceptance criteria:

- Core modules return the shared project result type.
- User-facing errors include enough context to fix the issue.
- There are no new `unwrap`/`expect` calls in user-facing paths.

## Phase 2: Extract the Core Domain Model

Move `Soundcourse` and `EffectNode` out of the current ontology module into `src/core/soundcourse.rs`.

Requirements:

- Preserve JSON-LD serialization compatibility.
- Preserve current effect variants:
  - Reverb
  - EQ
  - Delay
  - Compressor
  - Gain
- Keep the default JSON-LD context available.
- Keep `Soundcourse::new` or equivalent constructor.
- Add helper methods only when they clarify usage, for example:
  - `is_empty`
  - `effect_count`
  - `title_or_default`

Acceptance criteria:

- Existing parser tests still pass after import path updates.
- JSON-LD generated before and after the refactor remains semantically compatible.
- TUI imports the model from `core::soundcourse`, not from an ontology catch-all module.

## Phase 3: Add Soundcourse Validation

Create `src/core/validation.rs`.

Validation rules for v0.1:

| Effect | Parameter | Valid range |
|---|---|---|
| Gain | `gain_db` | `-24.0..=24.0` |
| EQ | `frequency` | `20.0..=20000.0` |
| EQ | `gain` | `-24.0..=24.0` |
| Delay | `delay_ms` | `0.0..=2000.0` |
| Delay | `feedback` | `0.0..=0.95` |
| Reverb | `room_size` | `0.0..=1.0` |
| Reverb | `dry_wet` | `0.0..=1.0` |
| Compressor | `threshold` | `-60.0..=0.0` |
| Compressor | `ratio` | `1.0..=20.0` |

Also validate:

- Soundcourse sequence must not be empty before processing.
- Sample rate, if provided, must be positive and reasonable. Use `8000.0..=192000.0`.
- Unknown or malformed effect types should not be silently accepted in strict parsing paths.

Add two validation modes:

- Strict mode: invalid Soundcourses fail.
- UI-friendly mode: missing optional values may be defaulted, but warnings should be collectable.

Acceptance criteria:

- Unit tests cover valid and invalid values for every effect.
- Batch processing validates before writing output.
- CLI `validate` reports failures clearly.
- TUI displays validation errors in the status area or modal.

## Phase 4: Split the Parser Layer

Move parser responsibilities into:

- `src/parser/mod.rs`
- `src/parser/ini.rs`
- `src/parser/jsonld.rs`

Requirements:

- Keep JSON-LD parsing.
- Keep INI parsing.
- Keep INI serialization.
- Add JSON-LD serialization as a parser-facing API if needed.
- Preserve existing round-trip tests.
- Add strict parse functions and lenient parse functions if useful.
- Unknown effect types in strict mode should produce an error, not be ignored.
- Missing parameters should either use documented defaults with warnings or fail, depending on mode.

Acceptance criteria:

- Existing tests continue to pass.
- Add tests for unknown effect types.
- Add tests for missing parameters.
- Add tests for invalid numeric values.
- Add tests for section ordering in INI.

## Phase 5: Extract Audio Processing Core

Move audio behavior out of the TUI into:

- `src/audio/decode.rs`
- `src/audio/encode.rs`
- `src/audio/process.rs`
- `src/audio/preview.rs`
- `src/audio/peaks.rs`

Requirements:

- Batch export is stable for WAV input and WAV output.
- Preview can remain best-effort through the existing audio stack.
- Keep effect order deterministic.
- Preserve sample rate and channel count for WAV export unless there is an explicit, tested reason not to.
- Do not introduce clipping silently if it can be detected. For v0.1, report clipping in the batch report rather than trying to solve mastering automatically.
- Ensure processing never emits NaN or infinity samples.
- Keep peak calculation independent from the TUI.

Acceptance criteria:

- TUI calls audio APIs rather than implementing decode/peak/process logic inline.
- Audio modules do not depend on Ratatui.
- Unit or integration tests cover gain, delay, silence, and no-NaN behavior.
- Processing a WAV with a no-op or zero-gain chain preserves duration and channel count.

## Phase 6: Add Batch Job and Output Report Modules

Create:

- `src/batch/job.rs`
- `src/batch/output.rs`
- `src/batch/report.rs`

Requirements:

- Process only top-level `.wav` files in the selected directory for v0.1.
- Write output to `output/` under the current directory by default.
- Never overwrite existing files.
- If a target filename exists, append a numeric suffix.
- Continue processing other files if one file fails.
- Produce a machine-readable report, preferably JSON, in the output directory.

Report fields:

- Input path.
- Output path, if successful.
- Soundcourse ID.
- Effects applied.
- Input sample rate, if available.
- Channel count, if available.
- Success/failure status.
- Error message if failed.
- Clipping or safety warnings, if available.

Acceptance criteria:

- Batch processing returns a structured report, not just a string.
- TUI can display progress from the batch job.
- CLI can print a concise summary and write the report.
- Tests cover filename collision behavior.
- Tests cover mixed directories where non-WAV files are skipped.
- Tests cover a failing file without aborting the whole batch.

## Phase 7: Keep and Thin the TUI

Move TUI-only rendering and input behavior into `app.rs` and `tui/` modules.

Requirements:

- Preserve the existing keyboard workflow:
  - `Tab` cycles panes.
  - `Enter` selects/adds.
  - `Shift + ↑/↓` reorders effects.
  - `d` deletes effect.
  - `←/→` adjusts parameters.
  - `Ctrl + Enter` starts batch processing.
  - `s` saves Soundcourse.
  - `o` opens directory prompt.
  - `p` toggles preview.
  - `?` opens help.
  - `q` quits.
- Add a pre-processing confirmation step that shows:
  - Number of WAV files found.
  - Output directory.
  - Active Soundcourse title or ID.
  - Effect count.
- Add cancel behavior during batch processing if feasible. If not feasible, document this as a known limitation for v0.1-beta.
- Stop preview when switching directories or quitting.
- Show preview state clearly.
- Improve status messages so failures are actionable.

Acceptance criteria:

- `main.rs` is reduced to app startup and error reporting.
- TUI modules do not contain DSP implementation details.
- TUI behavior remains usable after refactor.
- Help text matches actual controls.

## Phase 8: Add CLI Commands

Add a CLI path using Clap.

Required commands:

```text
amixima tui [path]
amixima apply --course <path> --input <dir> [--output <dir>]
amixima inspect --course <path>
amixima validate --course <path>
amixima list-effects
```

Behavior:

- Running `amixima` with no subcommand may launch the TUI for compatibility.
- Running `amixima <path>` may launch the TUI at that path if the current behavior already supports this.
- `apply` should run batch processing without launching the TUI.
- `validate` should parse and validate a Soundcourse.
- `inspect` should print metadata and the ordered effect chain.
- `list-effects` should print supported effects, parameters, defaults, and ranges.

Acceptance criteria:

- CLI uses the same parser, validation, audio, and batch modules as the TUI.
- CLI exits non-zero for invalid Soundcourses or failed batch setup.
- CLI prints concise user-facing output.
- CLI has tests where feasible.

## Phase 9: Add Tests and Fixtures

Add test coverage in layers.

Unit tests:

- Soundcourse validation.
- Effect parameter ranges.
- INI parse.
- JSON-LD parse.
- INI serialization.
- JSON-LD round trip.
- Output filename collision behavior.
- Unsupported file detection.
- Empty directory behavior.

Audio tests:

- Gain changes amplitude as expected.
- Zero-gain leaves samples unchanged within a reasonable tolerance.
- Delay produces a delayed signal.
- Compressor does not produce NaN or infinity.
- Reverb keeps output bounded.
- Chain order is preserved.

Integration tests:

- Apply a Soundcourse to a temp directory.
- Output directory is created.
- Existing files are not overwritten.
- Mixed directory processes only WAV files.
- Invalid Soundcourse exits with a useful error.

Fixtures:

- Generate tiny test WAV files programmatically where possible rather than committing large binaries.
- Include a README explaining fixture generation.

Acceptance criteria:

- `cargo test` passes.
- Tests do not depend on a local audio output device.
- Tests are deterministic.

## Phase 10: Add Documentation and Examples

Update README and add docs.

README must explain:

- What Amixima does.
- Who it is for.
- Current stable file support.
- Installation.
- TUI usage.
- CLI usage.
- Soundcourse basics.
- Output safety.
- Known limitations.

Create `docs/soundcourse-v1.md` with:

- Required fields.
- Optional fields.
- Supported effect types.
- Parameter names.
- Parameter ranges.
- Default values.
- Units.
- Example INI.
- Example JSON-LD.
- Compatibility rules.
- Unknown effect handling.
- Strict vs lenient parsing behavior.

Create examples:

- `examples/warm-bus.ini`
- `examples/warm-bus.jsonld`
- `examples/clean-gain.jsonld`
- `examples/slap-delay.ini`
- `examples/soft-compress.jsonld`

Acceptance criteria:

- Examples validate using `amixima validate`.
- README commands are accurate.
- Docs do not promise deferred features as stable.

## Phase 11: Add Quality Gates

Add or update GitLab CI if this repository uses GitLab.

Minimum quality gates:

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Optional if dependencies/tools are already acceptable:

```text
cargo audit
cargo deny check
```

Acceptance criteria:

- CI runs format, clippy, and tests.
- CI failures are easy to understand.
- Any optional audit/deny step is documented and does not block local development unnecessarily unless intentionally configured.

## Phase 12: Release Checklist

Create `docs/release-checklist.md` containing:

- Clean checkout builds.
- `cargo fmt` passes.
- `cargo clippy` passes.
- `cargo test` passes.
- Examples validate.
- TUI smoke test completed.
- CLI apply smoke test completed.
- Batch output does not overwrite input.
- Report file generated.
- README checked against actual behavior.
- Known limitations updated.
- Version bumped.
- Changelog updated.

## Development Workflow

Use a branch-based workflow.

Recommended branch sequence:

1. `chore/shared-error-type`
2. `refactor/core-soundcourse-model`
3. `feat/soundcourse-validation`
4. `refactor/parser-modules`
5. `refactor/audio-core`
6. `feat/batch-reporting`
7. `refactor/thin-tui`
8. `feat/cli-commands`
9. `test/audio-and-batch-fixtures`
10. `docs/v0.1-user-docs`
11. `ci/rust-quality-gates`
12. `release/v0.1-beta-prep`

Commit style:

- Make small commits.
- Each commit should compile unless a temporary WIP commit is clearly marked and later squashed.
- Use messages like:
  - `refactor(core): move soundcourse model into core module`
  - `feat(parser): add strict validation for unknown effect types`
  - `feat(batch): write structured processing report`
  - `test(audio): add gain and delay processing checks`
  - `docs(soundcourse): document v1 parameter ranges`

## Required Checks Before Completion

Run these commands and fix failures:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo run -- validate examples/warm-bus.ini
cargo run -- validate examples/warm-bus.jsonld
cargo run -- list-effects
```

If the CLI command names differ after implementation, update README and docs so they match the implemented interface.

## Final Deliverables

When finished, provide a summary with:

- Files changed.
- Architecture changes made.
- CLI commands added.
- Tests added.
- Docs added.
- Remaining known limitations.
- Commands run and their results.

Do not claim success unless the checks actually passed.

