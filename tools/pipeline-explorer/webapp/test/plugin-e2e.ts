// End-to-end check for the design-plugin upload path.
//
// Loads the Hazard3 plugin `.wasm` through the very loader the webapp's worker
// uses, then runs the same program natively and diffs the two traces. That
// covers the whole ABI -- allocation, the manifest, the trace encoding -- with
// no browser in the loop.
//
// Run it with `just test-plugin` from `tools/pipeline-explorer`.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { DesignPlugin, PLUGIN_ABI_VERSION } from "../src/wasm/plugin.ts";
import type { Trace } from "../src/types.ts";

const here = fileURLToPath(new URL(".", import.meta.url));
const wasmPath = process.argv[2] ?? `${here}../../plugins/dist/hazard3.wasm`;
const dumpPath = process.argv[3] ?? `${here}../../../../target/release/hazard3-plugin-dump`;
const asmPath = process.argv[4] ?? `${here}plugin-e2e.S`;
const cycles = Number(process.argv[5] ?? 500);
const echoPath = `${here}../../plugins/dist/echo.wasm`;

const failures: string[] = [];

function check(what: string, ok: boolean, detail = "") {
    if (ok) {
        console.log(`  ok   ${what}`);
    } else {
        console.log(`  FAIL ${what}${detail ? `: ${detail}` : ""}`);
        failures.push(what);
    }
}

/** The shape `hazard3-plugin-dump` prints, so the two can be compared as text. */
function canonical(trace: Trace): string {
    return JSON.stringify({
        cycles: trace.pipeline.cycles,
        instructions: trace.instructions,
        stages: trace.pipeline.keys,
        traces: trace.pipeline.traces.map((t) => Array.from(t)),
    });
}

const plugin = await DesignPlugin.compile(readFileSync(wasmPath));
const manifest = plugin.manifest;

console.log(`manifest of ${wasmPath}`);
check("declares the current ABI", manifest.abi === PLUGIN_ABI_VERSION, String(manifest.abi));
check("names the design", manifest.name === "Hazard3 (plugin)", manifest.name);
check("exposes the design's knobs", manifest.fields.length === 7, `${manifest.fields.length} fields`);
check(
    "keeps the field order the plugin reads",
    manifest.fields[0]?.id === "extension_m" && manifest.fields[6]?.id === "fast_branchcmp",
    manifest.fields.map((f) => f.id).join(","),
);

// Defaults first, then a couple of configurations that should change the
// pipeline, so the config encoding is exercised rather than ignored.
const configurations: Record<string, boolean>[] = [
    Object.fromEntries(manifest.fields.map((f) => [f.id, f.default === true])),
    Object.fromEntries(manifest.fields.map((f) => [f.id, true])),
    Object.fromEntries(manifest.fields.map((f) => [f.id, false])),
];

const assembly = readFileSync(asmPath, "utf8");
const traces: string[] = [];

for (const config of configurations) {
    const bits = manifest.fields.map((f) => (config[f.id] ? 1 : 0));
    const label = bits.join(",");

    const actual = canonical(plugin.run(assembly, config, cycles));
    const expected = execFileSync(dumpPath, [asmPath, String(cycles), label], {
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
    }).trim();

    console.log(`config [${label}]`);
    check("plugin trace matches a native run", actual === expected);
    if (actual !== expected) {
        console.log(`    plugin:  ${actual.slice(0, 300)}`);
        console.log(`    native:  ${expected.slice(0, 300)}`);
    }

    const trace = JSON.parse(actual);
    check("reports cycles", trace.cycles > 0, String(trace.cycles));
    check("disassembles the program", trace.instructions.length > 0);
    check("reports the F/X/M stages", JSON.stringify(trace.stages) === '["F","X","M"]');
    check(
        "fills every stage trace",
        trace.traces.length === 3 && trace.traces.every((t: number[]) => t.length > 0),
    );
    traces.push(JSON.stringify(trace.traces));
}

// This repo's Hazard3 instance traps a few cycles out of reset whatever it is
// given -- a pure `addi` program traces the same as the demo one -- so its
// output cannot show that a knob was honoured. The echo fixture below pins
// down the input side of the ABI instead, on a plugin whose output is fully
// determined by its inputs.
if (new Set(traces).size === 1) {
    console.log(
        "  note  every Hazard3 config traced identically; the design traps early " +
            "regardless of input (pre-existing, same for the built-in Hazard3)",
    );
}

console.log(`fixture ${echoPath}`);
const echo = await DesignPlugin.compile(readFileSync(echoPath));
check("reads a manifest with mixed field types", echo.manifest.fields.length === 3);
check(
    "carries the declared defaults",
    echo.manifest.fields[0]?.default === true && echo.manifest.fields[2]?.default === 7,
    JSON.stringify(echo.manifest.fields.map((f) => f.default)),
);

const echoed = echo.run("some assembly", { first: false, second: true, count: 99 }, 321);
check(
    "config crosses the ABI in manifest order",
    JSON.stringify(Array.from(echoed.pipeline.traces[0])) === "[0,1,99]",
    JSON.stringify(Array.from(echoed.pipeline.traces[0])),
);
check(
    "assembly and cycle count cross the ABI",
    echoed.instructions[0] === "asm 13 bytes" && echoed.instructions[1] === "cycles 321",
    echoed.instructions.join(" | "),
);
check(
    "falls back to a manifest default when the host omits a field",
    JSON.stringify(Array.from(echo.run("", {}, 1).pipeline.traces[0])) === "[1,0,7]",
);

let echoError = "";
try {
    echo.run("fail", {}, 1);
} catch (err) {
    echoError = String(err);
}
check("surfaces an error returned by the plugin", echoError.includes("asked to fail"), echoError);

// A plugin is untrusted input, so the loader has to reject junk rather than
// throw something unhelpful from deep inside the ABI.
console.log("rejects bad input");
await DesignPlugin.compile(new Uint8Array([0, 1, 2, 3])).then(
    () => check("refuses a non-wasm file", false),
    (err) => check("refuses a non-wasm file", String(err).includes("not a valid WebAssembly")),
);

// (module (import "env" "x" (func)))
const importsSomething = new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
    0x02, 0x09, 0x01, 0x03, 0x65, 0x6e, 0x76, 0x01, 0x78, 0x00, 0x00,
]);
await DesignPlugin.compile(importsSomething).then(
    () => check("refuses a module with imports", false),
    (err) => check("refuses a module with imports", String(err).includes("must not import")),
);

// (module (memory (export "memory") 1))
const noPluginExports = new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x05, 0x03, 0x01, 0x00, 0x01,
    0x07, 0x0a, 0x01, 0x06, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00,
]);
await DesignPlugin.compile(noPluginExports).then(
    () => check("refuses a module without the pe_* exports", false),
    (err) => check("refuses a module without the pe_* exports", String(err).includes("does not export")),
);

if (failures.length > 0) {
    console.error(`\n${failures.length} check(s) failed`);
    process.exit(1);
}
console.log("\nall checks passed");
