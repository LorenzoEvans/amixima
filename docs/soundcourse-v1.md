# Soundcourse v1

Soundcourse v1 is the Amixima effect-chain format for local WAV batch processing.

Supported file forms:

- `.ini`
- `.json`
- `.jsonld`

Required for processing:

- At least one effect.
- Valid effect parameters.
- Optional `mo:sampleRate` must be within `8000..=192000`.

Supported effects:

| Effect | Parameter | Range | Unit | Default |
|---|---:|---:|---|---:|
| Gain | `gain_db` | `-24..=24` | dB | `0` |
| EQ | `frequency` | `20..=20000` | Hz | `1000` |
| EQ | `gain` | `-24..=24` | dB | `0` |
| Delay | `delay_ms` | `0..=2000` | ms | `100` |
| Delay | `feedback` | `0..=0.95` | scalar | `0.5` |
| Reverb | `room_size` | `0..=1` | scalar | `0.5` |
| Reverb | `dry_wet` | `0..=1` | scalar | `0.3` |
| Compressor | `threshold` | `-60..=0` | dB | `-20` |
| Compressor | `ratio` | `1..=20` | scalar | `4` |

Strict parsing fails on unknown effect types and missing required INI effect parameters. The TUI may use lenient INI loading to preserve older files, but `amixima validate` is strict.

Example INI:

```ini
[effect1]
type=gain
gain_db=3
```

Example JSON-LD:

```json
{
  "@context": {
    "mo": "http://purl.org/ontology/mo/",
    "aufx": "http://purl.org/ontology/aufx-o/"
  },
  "@id": "amixima:sc:example",
  "@type": "mo:Workflow",
  "creator": "Amixima",
  "sequence": [
    {
      "aufx:type": "aufx:Gain",
      "aufx:parameters": {
        "aufx:gainDb": 3.0
      }
    }
  ]
}
```
