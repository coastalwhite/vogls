import init, {
    get_js_ibex_trace,
    get_js_neorv32_trace,
    get_js_hazard3_trace,
    get_js_trace,
} from "./pipeline_explorer.js";
import { DesignPlugin, errorMessage } from "./plugin.ts";

const ready = init();

/** Uploaded designs, keyed by the `proc` id the UI hands back to us. */
const plugins = new Map<string, DesignPlugin>();

self.onmessage = async (e: MessageEvent) => {
    const message = e.data;

    if (message?.kind === "loadPlugin") {
        try {
            const plugin = await DesignPlugin.compile(message.bytes);
            // Key by the manifest id, so uploading a rebuilt plugin replaces
            // the design it supersedes instead of sitting beside it.
            const proc = `plugin:${plugin.manifest.id}`;
            plugins.set(proc, plugin);
            self.postMessage({ kind: "pluginLoaded", proc, manifest: plugin.manifest });
        } catch (err) {
            self.postMessage({
                kind: "error",
                message: `${message.name}: ${errorMessage(err)}`,
            });
        }
        return;
    }

    if (message?.kind !== "run") {
        return;
    }

    const { proc, asm, config, numCycles } = message;
    const plugin = plugins.get(proc);

    try {
        if (plugin !== undefined) {
            self.postMessage({ kind: "trace", trace: plugin.run(asm, config, numCycles) });
            return;
        }

        await ready;
        if (proc === "picorv32") {
            self.postMessage({ kind: "trace", trace: get_js_trace(asm, config, numCycles) });
        } else if (proc === "ibex") {
            self.postMessage({ kind: "trace", trace: get_js_ibex_trace(asm, config, numCycles) });
        } else if (proc === "neorv32") {
            self.postMessage({ kind: "trace", trace: get_js_neorv32_trace(asm, config, numCycles) });
        } else if (proc === "hazard3") {
            self.postMessage({ kind: "trace", trace: get_js_hazard3_trace(asm, config, numCycles) });
        } else {
            self.postMessage({ kind: "error", proc, message: `unknown design '${proc}'` });
        }
    } catch (err) {
        self.postMessage({ kind: "error", proc, message: errorMessage(err) });
    }
};
