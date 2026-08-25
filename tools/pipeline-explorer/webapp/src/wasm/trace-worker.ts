import init, {
    get_js_ibex_trace,
    get_js_neorv32_trace,
    get_js_hazard3_trace,
    get_js_trace,
} from "./pipeline_explorer.js";

const ready = init();

self.onmessage = async (e: MessageEvent) => {
    if (!e.data["proc"]) {
        return;
    }

    await ready;
    const proc = e.data["proc"];
    const asm = e.data["asm"];
    const config = e.data["config"];
    const numCycles = e.data["numCycles"];

    if (proc === "picorv32") {
        const trace = get_js_trace(asm, config, numCycles);
        self.postMessage(trace);
    } else if (proc === "ibex") {
        const trace = get_js_ibex_trace(asm, config, numCycles);
        self.postMessage(trace);
    } else if (proc === "neorv32") {
        const trace = get_js_neorv32_trace(asm, config, numCycles);
        self.postMessage(trace);
    } else if (proc === "hazard3") {
        const trace = get_js_hazard3_trace(asm, config, numCycles);
        self.postMessage(trace);
    }
};
