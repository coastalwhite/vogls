# Design plugins

The pipeline explorer bundles four designs into its own wasm binary. A *plugin*
is the other way in: a standalone `.wasm` carrying one design, which you hand to
the webapp at runtime through **Custom…** in its design menu. Nothing needs
rebuilding or redeploying on the webapp side, and the design's sources never
have to enter this repository.

```sh
just plugin-hazard3      # builds plugins/dist/hazard3.wasm
just build-site && (cd webapp && npm run preview)
# then open the chip menu, pick "Custom…" and choose plugins/dist/hazard3.wasm
```

The uploaded design joins the built-ins in the chip menu, with the knobs its
manifest declares, and traces exactly like they do.

## Writing one

A plugin is a `cdylib` for `wasm32-unknown-unknown` that supplies two things: a
manifest describing the design's knobs, and a `run` that turns assembly plus
those knobs into a trace. [`echo`](echo) is the smallest possible one and is
worth reading first; [`hazard3`](hazard3) is a real design.

```rust
use pipeline_explorer_plugin::{Trace, cfg_bool};

pub const MANIFEST: &str = r#"{
  "abi": 1,
  "id": "my-cpu",
  "name": "My CPU",
  "fields": [
    { "id": "fast_alu", "type": "checkbox", "title": "Fast ALU", "default": true }
  ]
}"#;

pub fn run(assembly: &str, cfg: &[u32], num_cycles: u32) -> Result<Trace, String> {
    let fast_alu = cfg_bool(cfg, 0);
    // ... elaborate the design, run it, collect per-stage PCs ...
}

pipeline_explorer_plugin::export_plugin!(manifest = MANIFEST, run = run);
```

`export_plugin!` emits the whole ABI. The rules it cannot enforce for you:

- **No imports.** The webapp instantiates a plugin with an empty import object,
  so no `wasm-bindgen`, and nothing that reaches for the host. The loader
  rejects a module that imports anything, and says what it asked for.
- **Configuration is positional.** The host sends one `u32` per manifest field,
  in manifest order, which is why a plugin needs no JSON parser. Reading a field
  back with the wrong index silently reads the wrong knob, so keep `run` and the
  manifest next to each other.
- **A panic is fatal to the run.** `wasm32-unknown-unknown` aborts rather than
  unwinds, so return `Err` for anything you expect. The loader turns a trap into
  a legible error and starts the next run from a fresh instance.

The ABI itself -- exports, buffer layout, trace encoding -- is documented at the
top of [`../plugin/src/lib.rs`](../plugin/src/lib.rs).

Reusing this repo's design code is optional but convenient. The `hazard3` plugin
depends on `pipeline-explorer` with `default-features = false`, which drops the
`wasm-bindgen` entry points and every design but its own:

```toml
pipeline-explorer = { path = "../..", default-features = false, features = ["hazard3"] }
```

## Custom instructions

A plugin may accept mnemonics the base ISA does not have. `trva` dispatches
mnemonics through a table a plugin can add to, and
`pipeline_explorer_plugin::custom` holds the entries this repo ships:

| mnemonic | assembles to |
| --- | --- |
| `square xd, xs` | `mul xd, xs, xs` |
| `l1 xd` | `li xd, 1` |

They are shorthands, not new opcodes: they encode as instructions the design
already runs, so no RTL has to change and they behave exactly like what they
stand for. `square` is a `mul`, so it needs the M extension like any other.

The [`hazard3`](hazard3) plugin registers them, which is all it takes:

```rust
use pipeline_explorer_plugin::custom;

let value = get_hazard3_trace(assembly, num_cycles, &config, custom::ALL)?;
```

Registering is per design and nothing does it on a plugin's behalf — the
explorer's own Hazard3 entry passes none, so `square` there is still an unknown
mnemonic. A design that wants different shorthands does not have to take these:
`CustomInstruction::rd` and `::rd_rs` take a mnemonic and an encoder, for the
`mnemonic xd` and `mnemonic xd, xs` shapes. The encoder returns the word to
emit, so the mnemonic can stand for whatever the design pleases. A mnemonic the
base ISA already defines is silently replaced.

Because they assemble to ordinary instructions, the listing shows what they
became — `square t0, s1` reads back as `mul t0,s1,s1` — the way the base ISA's
own pseudo-instructions do.

Finally, a manifest may name its mnemonics so the editor highlights them:

```json
{ "abi": 1, "id": "my-cpu", "name": "My CPU", "fields": [],
  "instructions": ["square", "l1"] }
```

That list is cosmetic. What makes a mnemonic assemble is the registration above.

## Testing

```sh
just test-plugin     # the ABI, headless: loader vs. a native run of the design
just test-browser    # the "Custom…" entry itself, in chromium
```

`test-plugin` runs the plugin `.wasm` through the very loader the webapp's
worker uses and diffs the result against `hazard3-plugin-dump`, a native build
of the same `run`. Any drift in the encoding shows up as a trace mismatch.
