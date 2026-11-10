import { assemblySpec, primaryAssembly } from "../views/assembly.ts";
import { chooserSpec } from "../views/chooser.ts";
import { pipelineSpec } from "../views/pipeline.ts";
import type { Trace } from "../types.ts";
import type {
    NodeId,
    View,
    ViewContext,
    ViewInstance,
    ViewSpec,
    ViewType,
} from "./types.ts";

/** Every view type the app offers. A new one is one import and one entry. */
const specs: ViewSpec[] = [pipelineSpec, assemblySpec, chooserSpec];

export function viewSpecs(): ViewSpec[] {
    return specs.slice();
}

/** The types a user can pick, i.e. everything but the chooser itself. */
export function pickableSpecs(): ViewSpec[] {
    return specs.filter((spec) => spec.pickable !== false);
}

export function specFor(type: ViewType): ViewSpec | undefined {
    return specs.find((spec) => spec.type === type);
}

/** A fresh payload for a newly created view of this type. */
export function newView(type: ViewType): View {
    // Only assembly carries configuration; everything else is its bare tag, so
    // a payload-less view type needs no case here.
    if (type === "assembly") return { type, primary: !primaryAssembly() };
    return { type };
}

/** One view throwing must not stop the rest of the app from updating. */
function guard(what: string, run: () => void) {
    try {
        run();
    } catch (e) {
        console.error(`view ${what} failed`, e);
    }
}

type Entry = { type: ViewType; instance: ViewInstance };

/** Live view instances, keyed by the layout node that shows them. */
export class ViewRegistry {
    private ctx: ViewContext;
    private entries = new Map<NodeId, Entry>();

    constructor(ctx: ViewContext) {
        // Read live: `main.ts` mutates this object rather than replacing it.
        this.ctx = ctx;
    }

    /** Cached instance for this node, created on first call. */
    instance(nodeId: NodeId, view: View): ViewInstance {
        const entry = this.entries.get(nodeId);
        if (entry !== undefined) {
            if (entry.type === view.type) return entry.instance;
            // The id was reused for another type, so the cached instance can
            // never be shown again; keeping it would render the wrong view.
            this.release(nodeId);
        }

        const spec = specFor(view.type);
        if (spec === undefined) {
            throw new Error(`no view spec for type "${view.type}"`);
        }
        const instance = spec.create(view, this.ctx, nodeId);
        this.entries.set(nodeId, { type: view.type, instance });
        return instance;
    }

    /** Destroys and forgets everything whose id is not in `keep`. */
    retain(keep: Set<NodeId>) {
        for (const nodeId of [...this.entries.keys()]) {
            if (!keep.has(nodeId)) this.release(nodeId);
        }
    }

    /** Destroys and forgets one. */
    release(nodeId: NodeId) {
        const entry = this.entries.get(nodeId);
        if (entry === undefined) return;
        this.entries.delete(nodeId);
        guard("destroy", () => entry.instance.destroy());
    }

    titleOf(nodeId: NodeId): string {
        const entry = this.entries.get(nodeId);
        return entry === undefined ? "" : entry.instance.title;
    }

    broadcastCycle(cycle: number) {
        for (const { instance } of this.entries.values()) {
            const hook = instance.onCycleChange;
            if (hook === undefined) continue;
            guard("onCycleChange", () => hook.call(instance, cycle));
        }
    }

    broadcastTrace(trace: Trace) {
        for (const { instance } of this.entries.values()) {
            const hook = instance.onTraceChange;
            if (hook === undefined) continue;
            guard("onTraceChange", () => hook.call(instance, trace));
        }
    }

    destroyAll() {
        this.retain(new Set());
    }
}
