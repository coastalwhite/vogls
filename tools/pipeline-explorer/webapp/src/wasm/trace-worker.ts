import init, {
    elf_to_js_assembly,
    get_js_ibex_trace,
    get_js_neorv32_trace,
    get_js_hazard3_trace,
    get_js_trace,
} from "./pipeline_explorer.js";

const ready = init();

const tracers: Record<string, (program: string, config: object, numCycles: number, dataPoints: number) => object> = {
    picorv32: get_js_trace,
    ibex: get_js_ibex_trace,
    neorv32: get_js_neorv32_trace,
    hazard3: get_js_hazard3_trace,
};

/**
 * Requests, answered in order with a message of the same `kind`: either the
 * result (`trace` or `assembly`) or an `error`.
 */
export type WorkerRequest =
    | {
          kind: "trace";
          proc: string;
          program: string;
          config: object;
          numCycles: number;
          dataPoints: number;
      }
    | { kind: "disassemble"; proc: string; config: object; elf: Uint8Array; name: string };

export type WorkerReply =
    | { kind: "trace"; trace: object }
    | { kind: "trace"; error: string }
    | { kind: "disassemble"; assembly: string; name: string }
    | { kind: "disassemble"; error: string; name: string };

function errorMessage(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
}

function handle(request: WorkerRequest): WorkerReply {
    switch (request.kind) {
        case "trace": {
            const tracer = tracers[request.proc];
            if (!tracer) return { kind: "trace", error: `unknown core '${request.proc}'` };
            try {
                return { kind: "trace", trace: tracer(
                        request.program,
                        request.config,
                        request.numCycles,
                        request.dataPoints,
                    ) };
            } catch (err) {
                return { kind: "trace", error: errorMessage(err) };
            }
        }
        case "disassemble": {
            const { name } = request;
            try {
                const assembly = elf_to_js_assembly(request.elf, request.proc, request.config);
                return { kind: "disassemble", assembly, name };
            } catch (err) {
                return { kind: "disassemble", error: errorMessage(err), name };
            }
        }
    }
}

self.onmessage = async (e: MessageEvent<WorkerRequest>) => {
    await ready;
    self.postMessage(handle(e.data));
};
