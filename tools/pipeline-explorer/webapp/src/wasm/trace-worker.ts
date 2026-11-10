import init, {
    get_js_ibex_trace,
    get_js_neorv32_trace,
    get_js_hazard3_trace,
    get_js_trace,
} from "./pipeline_explorer.js";

const ready = init();

const tracers: Record<string, (program: string | Uint8Array, config: object, numCycles: number) => object> = {
    picorv32: get_js_trace,
    ibex: get_js_ibex_trace,
    neorv32: get_js_neorv32_trace,
    hazard3: get_js_hazard3_trace,
};

self.onmessage = async (e: MessageEvent) => {
    const tracer = tracers[e.data["proc"]];
    if (!tracer) {
        return;
    }

    await ready;
    try {
        self.postMessage(tracer(e.data["program"], e.data["config"], e.data["numCycles"]));
    } catch (err) {
        self.postMessage({ error: err instanceof Error ? err.message : String(err) });
    }
};
