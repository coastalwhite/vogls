import type {
    CycleRange,
    View,
    ViewContext,
    ViewInstance,
    ViewSpec,
} from "../layout/types.ts";
import { PipelineCanvas } from "../pipeline.ts";
import { ScrubberCanvas } from "../scrubber.ts";
import type { Trace } from "../types.ts";

const liveViews = new Set<PipelineView>();

/**
 * Adapter around `PipelineCanvas`, which builds its own wrapper and canvas into
 * the element it is handed and self-sizes from there. A foldable scrubber
 * overview of the whole trace is docked along the bottom.
 */
export class PipelineView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title: string = "Pipeline";

    private ctx: ViewContext;
    private canvas: PipelineCanvas | null = null;
    private scrubber: ScrubberCanvas | null = null;

    /** Hosts `canvas`; takes whatever height the scrubber dock leaves. */
    private body: HTMLDivElement;
    private scrubberHost: HTMLDivElement;
    private foldBtn: HTMLButtonElement;

    constructor(ctx: ViewContext) {
        this.ctx = ctx;

        this.root = document.createElement("div");
        this.root.className = "pipeline-view";

        this.body = document.createElement("div");
        this.body.className = "pipeline-view__body";

        const dock = document.createElement("div");
        dock.className = "pipeline-view__dock";
        this.foldBtn = document.createElement("button");
        this.foldBtn.type = "button";
        this.foldBtn.className = "pipeline-view__fold";
        // A down chevron; CSS flips it while folded.
        this.foldBtn.innerHTML =
            `<svg viewBox="0 0 12 8" aria-hidden="true"><path d="M1 1.5 6 6.5 11 1.5"/></svg>`;
        this.foldBtn.addEventListener("click", () =>
            this.setFolded(!this.scrubberHost.hidden),
        );
        this.scrubberHost = document.createElement("div");
        this.scrubberHost.className = "pipeline-view__scrubber";
        dock.append(this.foldBtn, this.scrubberHost);

        this.root.append(this.body, dock);
        this.setFolded(false);

        // PipelineCanvas measures and draws from a trace, so there is nothing
        // to build until one exists. The body stays empty until then.
        if (ctx.trace !== null) this.onTraceChange(ctx.trace);

        liveViews.add(this);
    }

    onCycleChange(cycles: CycleRange) {
        this.scrubber?.setSelection(cycles);
        if (this.canvas === null) return;
        this.canvas.setSelection(cycles);
        // Scrolling to the selection while the user is dragging it out on this
        // very canvas would fight the gesture.
        if (!this.canvas.selecting) this.canvas.moveToCycle(cycles.start, 0.25);
    }

    onTraceChange(trace: Trace) {
        if (this.canvas === null) {
            this.canvas = new PipelineCanvas(
                this.body,
                trace,
                this.ctx.cycles,
                (range) => this.ctx.selectCycles(range),
            );
            this.scrubber = new ScrubberCanvas(
                this.scrubberHost,
                trace,
                this.ctx.cycles,
                (cycle) => this.ctx.goToCycle(cycle),
            );
        } else {
            this.canvas.setTrace(trace);
            this.scrubber?.setTrace(trace);
        }
    }

    private setFolded(folded: boolean) {
        this.scrubberHost.hidden = folded;
        const label = folded ? "Show scrubber" : "Hide scrubber";
        this.foldBtn.title = label;
        this.foldBtn.setAttribute("aria-label", label);
        this.foldBtn.setAttribute("aria-expanded", String(!folded));
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
        if (this.scrubber !== null) this.scrubber.ro.disconnect();
        this.canvas = null;
        this.scrubber = null;
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
