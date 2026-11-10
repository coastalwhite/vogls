import type {
    CycleRange,
    View,
    ViewContext,
    ViewInstance,
    ViewSpec,
} from "../layout/types.ts";
import { PipelineCanvas } from "../pipeline.ts";
import type { Trace } from "../types.ts";

const liveViews = new Set<PipelineView>();

/**
 * Adapter around `PipelineCanvas`, which builds its own wrapper and canvas into
 * the element it is handed and self-sizes from there.
 */
export class PipelineView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title: string = "Pipeline";

    private ctx: ViewContext;
    private canvas: PipelineCanvas | null = null;

    constructor(ctx: ViewContext) {
        this.ctx = ctx;

        this.root = document.createElement("div");
        this.root.className = "pipeline-view";
        this.root.style.position = "relative";
        this.root.style.width = "100%";
        this.root.style.height = "100%";
        this.root.style.overflow = "hidden";

        // PipelineCanvas measures and draws from a trace, so there is nothing
        // to build until one exists. The body stays empty until then.
        if (ctx.trace !== null) this.onTraceChange(ctx.trace);

        liveViews.add(this);
    }

    onCycleChange(cycles: CycleRange) {
        if (this.canvas === null) return;
        this.canvas.setSelection(cycles);
        // Scrolling to the selection while the user is dragging it out on this
        // very canvas would fight the gesture.
        if (!this.canvas.selecting) this.canvas.moveToCycle(cycles.start, 0.25);
    }

    onTraceChange(trace: Trace) {
        if (this.canvas === null) {
            this.canvas = new PipelineCanvas(
                this.root,
                trace,
                this.ctx.cycles,
                (range) => this.ctx.selectCycles(range),
            );
        } else {
            this.canvas.setTrace(trace);
        }
    }

    /** The cycle range on screen, or `null` before the first trace. */
    visibleCycles(): [number, number] | null {
        return this.canvas === null ? null : this.canvas.visibleCycles();
    }

    destroy() {
        liveViews.delete(this);
        // PipelineCanvas has no teardown of its own, and its ResizeObserver
        // would otherwise keep observing a detached element.
        if (this.canvas !== null) this.canvas.ro.disconnect();
        this.canvas = null;
        this.root.remove();
    }
}

export const pipelineSpec: ViewSpec = {
    type: "pipeline",
    label: "Pipeline",
    create: (_view: View, ctx: ViewContext) => new PipelineView(ctx),
};

/** First live pipeline view, for the share panel's cycle range. Null if none. */
export function anyPipelineView(): PipelineView | null {
    for (const view of liveViews) return view;
    return null;
}
