import { colors } from "./colors.ts";
import type { PipelineData, Trace } from "./types.ts";
import { EMPTY_PIPELINE, dataOf } from "./types.ts";
import type { CycleRange } from "./layout/types.ts";

export class ScrubberCanvas {
    width: number = 0;
    height: number = 0;

    dragging: boolean = false;
    selection: CycleRange;

    wrap: HTMLDivElement;
    canvas: HTMLCanvasElement;
    ro: ResizeObserver;

    trace: Trace;
    /** The pipeline data of `trace`, or an empty stand-in. */
    pipeline: PipelineData;

    goToCycle: (cycle: number, secs: number) => void;

    constructor(
        elem: HTMLElement,
        trace: Trace,
        selection: CycleRange,
        goToCycle: (cycle: number, secs: number) => void,
    ) {
        this.trace = trace;
        this.pipeline = dataOf(trace, "pipeline") ?? EMPTY_PIPELINE;

        this.wrap = document.createElement("div");
        this.wrap.style.position = "relative";
        this.wrap.style.width = "100%";
        this.wrap.style.height = "100%";
        this.canvas = document.createElement("canvas");
        this.canvas.style.width = "100%";
        this.canvas.style.height = "100%";
        this.canvas.style.inset = "0";
        this.canvas.style.display = "block";
        this.canvas.style.position = "absolute";
        this.wrap.appendChild(this.canvas);
        elem.appendChild(this.wrap);

        this.selection = selection;
        this.goToCycle = goToCycle;

        this.ro = new ResizeObserver((entries) => {
            const r = entries[0].contentRect;
            this.width = r.width;
            this.height = r.height;
            this.draw();
        });
        this.ro.observe(this.wrap, { box: "content-box" });

        this.canvas.addEventListener(
            "pointerdown",
            (e) => this.onPointerDown(e),
            {
                passive: false,
            },
        );
        this.canvas.addEventListener("pointerup", (e) => this.onPointerUp(e), {
            passive: false,
        });
        this.canvas.addEventListener(
            "pointermove",
            (e) => this.onPointerMove(e),
        );

        this.draw();
    }

    setTrace(trace: Trace) {
        this.trace = trace;
        this.pipeline = dataOf(trace, "pipeline") ?? EMPTY_PIPELINE;
        this.draw();
    }
    setSelection(selection: CycleRange) {
        this.selection = selection;
        this.draw();
    }

    draw() {
        const dpr = window.devicePixelRatio || 1;
        this.canvas.width = this.width * dpr;
        this.canvas.height = this.height * dpr;

        const numStages = this.pipeline.keys.length;
        const cycles = this.pipeline.cycles;
        const numInstructions = this.trace.instructions.length;

        const ctx = this.canvas.getContext("2d")!;
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, this.width, this.height);

        const cellW = this.width / cycles;
        const cellH = this.height / numInstructions;

        // Draw all stage cells. Iterate by stage so we batch fillStyle changes.
        for (let i = 0; i < numStages; i++) {
            ctx.fillStyle = colors[i % colors.length];
            const stageTrace = this.pipeline.traces[i];
            for (let c = 0; c < cycles; c++) {
                const value = stageTrace[c];
                if (value === 0) continue;
                const y = value * cellH;
                const x = c * cellW;
                // Use ceil to avoid sub-pixel gaps between cells
                ctx.fillRect(x, y, Math.ceil(cellW), Math.ceil(cellH));
            }
        }

        // The selected span, then the playhead at its start.
        const span = Math.max(1, cycles - 1);
        const startX = (this.selection.start / span) * this.width;
        if (this.selection.end > this.selection.start) {
            const endX = (this.selection.end / span) * this.width;
            ctx.fillStyle = "rgba(0, 0, 0, 0.14)";
            ctx.fillRect(startX, 0, Math.max(1, endX - startX), this.height);
        }
        ctx.fillStyle = "rgba(0, 0, 0, 0.6)";
        ctx.fillRect(startX - 1, 0, 2, this.height);
    }
    seek(e: PointerEvent) {
        const rect = this.canvas.getBoundingClientRect();
        const fraction = Math.max(
            0,
            Math.min(1, (e.clientX - rect.left) / rect.width),
        );
        const cycle = Math.round(fraction * (this.pipeline.cycles - 1));
        this.goToCycle(cycle, 0.25);
    }

    onPointerDown(e: PointerEvent) {
        this.dragging = true;
        this.canvas.setPointerCapture(e.pointerId);
        this.seek(e);
    }
    onPointerMove(e: PointerEvent) {
        if (this.dragging) this.seek(e);
    }
    onPointerUp(e: PointerEvent) {
        this.dragging = false;
        this.canvas.releasePointerCapture(e.pointerId);
    }
}
