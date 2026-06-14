# Examples

Included examples:

- `examples/warm-bus.ini`: gain, EQ, and compression.
- `examples/warm-bus.jsonld`: JSON-LD form of the warm bus chain.
- `examples/clean-gain.jsonld`: single gain stage.
- `examples/slap-delay.ini`: short delay plus output trim.
- `examples/soft-compress.jsonld`: light compression.
- `examples/spacious-delay.jsonld`: delay and reverb.

Validate examples:

```bash
cargo run -- validate examples/warm-bus.ini
cargo run -- validate examples/warm-bus.jsonld
cargo run -- validate examples/clean-gain.jsonld
cargo run -- validate examples/slap-delay.ini
cargo run -- validate examples/soft-compress.jsonld
```
