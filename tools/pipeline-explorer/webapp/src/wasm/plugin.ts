// Loader for uploaded design plugins.
//
// A plugin is a standalone `wasm32-unknown-unknown` module carrying one design
// and exporting the `pe_*` ABI documented in `tools/pipeline-explorer/plugin`.
// It is instantiated with an empty import object, so anything it needs has to
// be compiled into it -- no `wasm-bindgen` glue is involved.

import type { Trace } from "../types.ts";

/** Must match `ABI_VERSION` in the plugin crate. */
export const PLUGIN_ABI_VERSION = 1;

export type PluginField = {
    id: string;
    type: "checkbox" | "number";
    title: string;
    default: boolean | number;
};

export type PluginManifest = {
    abi: number;
    id: string;
    name: string;
    fields: PluginField[];
};

type PluginExports = {
    memory: WebAssembly.Memory;
    pe_abi_version(): number;
    pe_alloc(len: number): number;
    pe_dealloc(ptr: number, len: number): void;
    pe_free_buffer(ptr: number): void;
    pe_manifest(): number;
    pe_run(
        asmPtr: number,
        asmLen: number,
        cfgPtr: number,
        cfgLen: number,
        numCycles: number,
    ): number;
};

const REQUIRED_EXPORTS = [
    "pe_abi_version",
    "pe_alloc",
    "pe_dealloc",
    "pe_free_buffer",
    "pe_manifest",
    "pe_run",
] as const;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/** Sequential reader over a decoded payload, mirroring the Rust encoder. */
class Reader {
    private bytes: Uint8Array;
    private view: DataView;
    private offset = 0;

    constructor(bytes: Uint8Array) {
        this.bytes = bytes;
        this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    }

    u32(): number {
        if (this.offset + 4 > this.bytes.byteLength) {
            throw new Error("plugin returned a truncated payload");
        }
        const value = this.view.getUint32(this.offset, true);
        this.offset += 4;
        return value;
    }

    str(): string {
        const len = this.u32();
        if (this.offset + len > this.bytes.byteLength) {
            throw new Error("plugin returned a truncated payload");
        }
        const s = decoder.decode(this.bytes.subarray(this.offset, this.offset + len));
        this.offset += len;
        return s;
    }

    u32Array(len: number): Uint32Array {
        const values = new Uint32Array(len);
        for (let i = 0; i < len; i++) values[i] = this.u32();
        return values;
    }
}

/**
 * Copies a length-prefixed host buffer out of the plugin's memory and releases
 * it. The copy matters: any later call may grow the memory and detach every
 * view onto it.
 */
function takeHostBuffer(exports: PluginExports, ptr: number): Uint8Array {
    const len = new DataView(exports.memory.buffer).getUint32(ptr, true);
    const payload = new Uint8Array(exports.memory.buffer, ptr + 4, len).slice();
    exports.pe_free_buffer(ptr);
    return payload;
}

function parseManifest(json: string): PluginManifest {
    let raw: unknown;
    try {
        raw = JSON.parse(json);
    } catch {
        throw new Error("plugin manifest is not valid JSON");
    }
    if (typeof raw !== "object" || raw === null) {
        throw new Error("plugin manifest is not an object");
    }

    const manifest = raw as Record<string, unknown>;
    if (manifest.abi !== PLUGIN_ABI_VERSION) {
        throw new Error(
            `plugin manifest declares ABI ${manifest.abi}, expected ${PLUGIN_ABI_VERSION}`,
        );
    }
    if (typeof manifest.id !== "string" || manifest.id === "") {
        throw new Error("plugin manifest is missing an 'id'");
    }
    if (typeof manifest.name !== "string" || manifest.name === "") {
        throw new Error("plugin manifest is missing a 'name'");
    }
    if (!Array.isArray(manifest.fields)) {
        throw new Error("plugin manifest is missing 'fields'");
    }

    const seen = new Set<string>();
    const fields = manifest.fields.map((raw: unknown, i: number): PluginField => {
        const field = raw as Record<string, unknown>;
        if (typeof field?.id !== "string" || field.id === "") {
            throw new Error(`plugin manifest field ${i} is missing an 'id'`);
        }
        if (seen.has(field.id)) {
            throw new Error(`plugin manifest repeats the field id '${field.id}'`);
        }
        seen.add(field.id);
        if (field.type !== "checkbox" && field.type !== "number") {
            throw new Error(
                `plugin manifest field '${field.id}' has unsupported type '${field.type}'`,
            );
        }
        if (typeof field.title !== "string") {
            throw new Error(`plugin manifest field '${field.id}' is missing a 'title'`);
        }
        return {
            id: field.id,
            type: field.type,
            title: field.title,
            default: field.type === "checkbox"
                ? field.default === true
                : Number(field.default ?? 0),
        };
    });

    return { abi: PLUGIN_ABI_VERSION, id: manifest.id, name: manifest.name, fields };
}

/** One uploaded design, ready to be run. */
export class DesignPlugin {
    private module: WebAssembly.Module;
    readonly manifest: PluginManifest;

    private constructor(module: WebAssembly.Module, manifest: PluginManifest) {
        this.module = module;
        this.manifest = manifest;
    }

    /**
     * Compiles an uploaded `.wasm` and reads its manifest, rejecting anything
     * that does not implement the expected ABI.
     */
    static async compile(bytes: BufferSource): Promise<DesignPlugin> {
        let module: WebAssembly.Module;
        try {
            module = await WebAssembly.compile(bytes);
        } catch (err) {
            throw new Error(`not a valid WebAssembly module: ${errorMessage(err)}`);
        }

        const imports = WebAssembly.Module.imports(module);
        if (imports.length !== 0) {
            const names = imports.map((i) => `${i.module}.${i.name}`).join(", ");
            throw new Error(
                `plugin must not import anything, but asks for: ${names}. ` +
                    "Build it for wasm32-unknown-unknown without wasm-bindgen.",
            );
        }

        const exports = DesignPlugin.instantiate(module);
        const abi = exports.pe_abi_version();
        if (abi !== PLUGIN_ABI_VERSION) {
            throw new Error(`plugin implements ABI ${abi}, expected ${PLUGIN_ABI_VERSION}`);
        }

        const manifest = parseManifest(
            decoder.decode(takeHostBuffer(exports, exports.pe_manifest())),
        );
        return new DesignPlugin(module, manifest);
    }

    private static instantiate(module: WebAssembly.Module): PluginExports {
        const exports = new WebAssembly.Instance(module, {}).exports;
        for (const name of REQUIRED_EXPORTS) {
            if (typeof exports[name] !== "function") {
                throw new Error(`plugin does not export '${name}'`);
            }
        }
        if (!(exports.memory instanceof WebAssembly.Memory)) {
            throw new Error("plugin does not export its memory");
        }
        return exports as unknown as PluginExports;
    }

    /**
     * Traces `assembly` on this design.
     *
     * Every run gets a fresh instance: a design that panics traps and leaves
     * its instance unusable, and starting over also keeps one long-running
     * simulation from holding on to the memory it grew into.
     */
    run(assembly: string, config: Record<string, boolean | number>, numCycles: number): Trace {
        const exports = DesignPlugin.instantiate(this.module);

        const asm = encoder.encode(assembly);
        // The configuration is positional: one `u32` per manifest field, in
        // manifest order. The plugin knows what each slot means.
        const cfg = this.manifest.fields.map((field) => {
            const value = config[field.id] ?? field.default;
            return field.type === "checkbox" ? (value ? 1 : 0) : Number(value) >>> 0;
        });

        // Allocate both blocks before writing either: a second allocation can
        // grow the memory, which detaches any view taken before it.
        const asmPtr = exports.pe_alloc(asm.byteLength);
        const cfgPtr = exports.pe_alloc(cfg.length * 4);

        new Uint8Array(exports.memory.buffer).set(asm, asmPtr);
        const view = new DataView(exports.memory.buffer);
        cfg.forEach((value, i) => view.setUint32(cfgPtr + i * 4, value, true));

        let out: number;
        try {
            out = exports.pe_run(asmPtr, asm.byteLength, cfgPtr, cfg.length, numCycles);
        } catch (err) {
            // A trap means the design panicked; the instance is gone with it,
            // so there is nothing left to free.
            throw new Error(`plugin crashed while tracing: ${errorMessage(err)}`);
        }
        exports.pe_dealloc(asmPtr, asm.byteLength);
        exports.pe_dealloc(cfgPtr, cfg.length * 4);

        return decodeTrace(takeHostBuffer(exports, out));
    }
}

function decodeTrace(payload: Uint8Array): Trace {
    const reader = new Reader(payload);

    const status = reader.u32();
    if (status !== 0) {
        throw new Error(reader.str());
    }

    const cycles = reader.u32();

    const instructions: string[] = [];
    const numInstructions = reader.u32();
    for (let i = 0; i < numInstructions; i++) instructions.push(reader.str());

    const keys: string[] = [];
    const traces: Uint32Array[] = [];
    const numStages = reader.u32();
    for (let i = 0; i < numStages; i++) {
        keys.push(reader.str());
        traces.push(reader.u32Array(reader.u32()));
    }

    return { instructions, pipeline: { traces, keys, cycles } };
}

export function errorMessage(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
}
