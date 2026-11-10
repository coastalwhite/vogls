import { colors } from "./colors.ts";
import type { Trace } from "./types.ts";
import type { CycleRange } from "./layout/types.ts";

const textPad = 8;

export class PipelineCanvas {
    width: number = 0;
    height: number = 0;

    offsetX: number = 0;
    offsetY: number = 0;

    wrap: HTMLDivElement;
    canvas: HTMLCanvasElement;
    ro: ResizeObserver;

    trace: Trace;
    maxTextWidth: number;

    selection: CycleRange;
    onSelect: (range: CycleRange) => void;
    /** Set while the user is dragging a selection out on this canvas. */
    selecting: boolean = false;

    /** Cycle the current drag started from. */
    anchor: number | null = null;

    activeTouchId: number | null = null;
    lastTouchX: number | null = null;
    lastTouchY: number | null = null;

    keyFrame: {
        startTime: number;
        startOffset: number;
        endTime: number;
        endOffset: number;
    } | null = null;

    constructor(
        elem: HTMLElement,
        trace: Trace,
        selection: CycleRange,
        onSelect: (range: CycleRange) => void,
    ) {
        this.trace = trace;

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

        this.maxTextWidth = this.calculateMaxTextWidth();

        this.selection = selection;
        this.onSelect = onSelect;

        this.ro = new ResizeObserver((entries) => {
            const r = entries[0].contentRect;
            this.width = r.width;
            this.height = r.height;
            this.offsetX = this.clampOffsetX(this.offsetX);
            this.offsetY = this.clampOffsetY(this.offsetY);
            this.draw();
        });
        this.ro.observe(this.wrap, { box: "content-box" });

        this.canvas.addEventListener("wheel", (e) => this.onWheel(e), {
            passive: false,
        });
        this.canvas.addEventListener(
            "touchstart",
            (e) => this.onTouchStart(e),
            {
                passive: false,
            },
        );
        this.canvas.addEventListener("touchmove", (e) => this.onTouchMove(e), {
            passive: false,
        });
        this.canvas.addEventListener("touchend", (e) => this.onTouchEnd(e));
        this.canvas.addEventListener("touchcancel", (e) => this.onTouchEnd(e));

        this.canvas.addEventListener("pointerdown", (e) => this.onPointerDown(e));
        this.canvas.addEventListener("pointermove", (e) => this.onPointerMove(e));
        this.canvas.addEventListener("pointerup", (e) => this.onPointerUp(e));
        this.canvas.addEventListener("pointercancel", (e) => this.onPointerUp(e));

        this.draw();
    }

    setTrace(trace: Trace) {
        this.trace = trace;
        this.maxTextWidth = this.calculateMaxTextWidth();
        this.offsetX = this.clampOffsetX(this.offsetX);
        this.offsetY = this.clampOffsetY(this.offsetY);
        this.draw();
    }
    setSelection(selection: CycleRange) {
        this.selection = selection;
        this.draw();
    }

    calculateMaxTextWidth(): number {
        const ctx = this.canvas.getContext("2d")!;
        ctx.font = "1rem monospace";
        return this.trace.instructions.reduce(
            (max, item) => Math.max(max, ctx.measureText(item).width),
            0,
        );
    }

    clampOffsetX(x: number): number {
        return clamp(
            x,
            -5 * convertRemToPixels(2),
            Math.max(
                (this.trace.pipeline.cycles + 5) * convertRemToPixels(2) -
                    (this.width - this.maxTextWidth - 2 * textPad),
                0,
            ),
        );
    }
    clampOffsetY(y: number): number {
        return clamp(
            y,
            0,
            Math.max(
                (this.trace.instructions.length + 1) * convertRemToPixels(2) -
                    this.height,
                0,
            ),
        );
    }

    applyDxDy(dx: number, dy: number) {
        this.offsetX = this.clampOffsetX(this.offsetX + dx);
        this.offsetY = this.clampOffsetY(this.offsetY + dy);
        this.draw();
    }

    draw() {
        const dpr = window.devicePixelRatio || 1;

        const canvasWidth = Math.round(this.width * dpr);
        const canvasHeight = Math.round(this.height * dpr);
        if (this.canvas.width !== canvasWidth) this.canvas.width = canvasWidth;
        if (this.canvas.height !== canvasHeight) {
            this.canvas.height = canvasHeight;
        }

        const ctx = this.canvas.getContext("2d")!;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, this.width, this.height);

        const cellW = convertRemToPixels(2);
        const startX = Math.floor(this.offsetX / cellW) - 1;
        const endX = Math.ceil(startX + this.width / cellW) + 2;

        renderPipeline(ctx, this.trace, {
            rows: this.trace.instructions.map((_, i) => i),
            startCycle: startX,
            endCycle: endX,
            offsetX: this.offsetX,
            offsetY: this.offsetY,
            width: this.width,
            height: this.height,
            maxTextWidth: this.maxTextWidth,
            selection: this.selection,
        });
    }

    /** The cycles currently (at least partially) on screen. */
    visibleCycles(): [number, number] {
        const cellW = convertRemToPixels(2);
        const lo = Math.floor(this.offsetX / cellW);
        const hi = Math.ceil(
            (this.offsetX + this.width - this.maxTextWidth - 2 * textPad) /
                cellW,
        ) - 1;
        const last = this.trace.pipeline.cycles - 1;
        return [clamp(lo, 0, last), clamp(hi, 0, last)];
    }

    /** The cycle under a canvas-relative x, or null when past the trace. */
    cycleAt(offsetX: number): number | null {
        const cellW = convertRemToPixels(2);
        const paddingX = this.maxTextWidth + textPad * 2;
        const cycle = Math.floor((offsetX + this.offsetX - paddingX) / cellW);
        const last = this.trace.pipeline.cycles - 1;
        return cycle < 0 || cycle > last ? null : cycle;
    }

    /**
     * Press picks one cycle, drag extends to a span. Mouse only: touch is
     * already the pan gesture, and stealing it would make the view unusable.
     */
    onPointerDown(e: PointerEvent) {
        if (e.pointerType !== "mouse" || e.button !== 0) return;
        const cycle = this.cycleAt(e.offsetX);
        if (cycle === null) return;
        e.preventDefault();
        this.anchor = cycle;
        this.selecting = true;
        this.canvas.setPointerCapture(e.pointerId);
        this.emit(cycle, cycle);
    }

    onPointerMove(e: PointerEvent) {
        if (!this.selecting || this.anchor === null) return;
        const cycle = this.cycleAt(e.offsetX);
        if (cycle === null) return;
        this.emit(
            Math.min(this.anchor, cycle),
            Math.max(this.anchor, cycle),
        );
    }

    onPointerUp(e: PointerEvent) {
        if (!this.selecting) return;
        this.selecting = false;
        this.anchor = null;
        if (this.canvas.hasPointerCapture(e.pointerId)) {
            this.canvas.releasePointerCapture(e.pointerId);
        }
    }

    private emit(start: number, end: number) {
        if (this.selection.start === start && this.selection.end === end) return;
        this.setSelection({ start, end });
        this.onSelect({ start, end });
    }

    onWheel(e: WheelEvent) {
        e.preventDefault();
        const lineHeight = convertRemToPixels(1);
        const pageWidth = this.canvas
            ? this.canvas.clientWidth
            : window.innerWidth;

        let scale;
        switch (e.deltaMode) {
            case WheelEvent.DOM_DELTA_PIXEL:
                scale = 1;
                break;
            case WheelEvent.DOM_DELTA_LINE:
                scale = lineHeight;
                break;
            case WheelEvent.DOM_DELTA_PAGE:
                scale = pageWidth;
                break;
            default:
                scale = 1;
        }
        if (e.shiftKey) {
            this.applyDxDy(e.deltaY * scale, 0);
        } else {
            this.applyDxDy(e.deltaX * scale, e.deltaY * scale);
        }
    }

    onTouchStart(e: TouchEvent) {
        const t = e.changedTouches[0];
        this.activeTouchId = t.identifier;
        this.lastTouchX = t.clientX;
        this.lastTouchY = t.clientY;
    }
    onTouchMove(e: TouchEvent) {
        e.preventDefault(); // works because of touch-action:none + passive:false
        // find the touch we're tracking
        let t = null;
        for (const touch of e.changedTouches) {
            if (touch.identifier === this.activeTouchId) {
                t = touch;
                break;
            }
        }
        if (!t || this.lastTouchX === null || this.lastTouchY === null) return;
        // dragging right should feel like scrolling content left -> negate
        const dx = -(t.clientX - this.lastTouchX);
        const dy = -(t.clientY - this.lastTouchY);
        this.lastTouchX = t.clientX;
        this.lastTouchY = t.clientY;
        if (dx !== 0 || dy !== 0) this.applyDxDy(dx, dy);
    }
    onTouchEnd(e: TouchEvent) {
        for (const touch of e.changedTouches) {
            if (touch.identifier === this.activeTouchId) {
                this.activeTouchId = null;
                this.lastTouchX = null;
                this.lastTouchY = null;
                break;
            }
        }
    }

    moveToCycle(cycle: number, seconds: number) {
        const targetOffsetX = cycle * convertRemToPixels(2) -
            ((this.width - this.maxTextWidth - textPad * 2) / 2);
        if (seconds === 0.0) {
            this.offsetX = this.clampOffsetX(targetOffsetX);
            return;
        }

        const t = performance.now();
        this.keyFrame = {
            startTime: t,
            startOffset: this.offsetX,
            endTime: t + seconds * 1000,
            endOffset: targetOffsetX,
        };

        const { startTime, startOffset, endTime, endOffset } = this.keyFrame;
        const dur = endTime - startTime;
        const tick = () => {
            const now = performance.now();
            const t = dur <= 0
                ? 1
                : Math.min(1, Math.max(0, (now - startTime) / dur));
            this.offsetX = this.clampOffsetX(
                lerp(startOffset, endOffset, easeInOut(t)),
            );
            this.draw();
            if (t < 1) requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
    }
}

/** A display row: an instruction index, or `null` for an elided gap. */
export type Row = number | null;

export type PipelineLayout = {
    rows: Row[];
    /** Cells are drawn for cycles in [startCycle, endCycle). */
    startCycle: number;
    endCycle: number;
    offsetX: number;
    offsetY: number;
    width: number;
    height: number;
    maxTextWidth: number;
    selection: CycleRange | null;
};

/** The subset of `CanvasRenderingContext2D` that `renderPipeline` uses, so
 *  the same drawing code can target the canvas and the SVG exporter. */
export type Ctx2D = Pick<
    CanvasRenderingContext2D,
    | "fillStyle"
    | "font"
    | "textAlign"
    | "textBaseline"
    | "fillRect"
    | "fillText"
    | "beginPath"
    | "moveTo"
    | "lineTo"
    | "stroke"
    | "setLineDash"
>;

export function renderPipeline(
    ctx: Ctx2D,
    trace: Trace,
    layout: PipelineLayout,
) {
    const { rows, startCycle, endCycle, offsetX, offsetY, width, height } =
        layout;
    const numStages = trace.pipeline.keys.length;

    // Trace values are 1-based instruction indices; map them to display rows.
    const rowOf = new Int32Array(trace.instructions.length).fill(-1);
    rows.forEach((r, i) => {
        if (r !== null) rowOf[r] = i;
    });

    const cellW = convertRemToPixels(2);
    const cellH = convertRemToPixels(2);

    const pxHeight = (rows.length + 1) * cellH;
    const paddingX = layout.maxTextWidth + textPad * 2;

    // The selection: one cell when collapsed, a band across the span.
    if (
        layout.selection !== null &&
        layout.selection.end >= startCycle &&
        layout.selection.start <= endCycle
    ) {
        const from = Math.max(layout.selection.start, startCycle);
        const to = Math.min(layout.selection.end, endCycle);
        const x = from * cellW + paddingX - offsetX;
        ctx.fillStyle = "#ccc";
        ctx.fillRect(x, 0, (to - from + 1) * cellW, height);
    }

    // Vertical guide lines.
    ctx.fillStyle = "#000";
    ctx.font = "0.75rem monospace";
    ctx.textBaseline = "top";
    ctx.setLineDash([5, 15]);
    for (let c = startCycle - (startCycle % 5); c < endCycle; c += 5) {
        const x = c * cellW + paddingX - offsetX;
        const y = -offsetY;

        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x, y + pxHeight);
        ctx.stroke();

        ctx.fillText(c.toString(), x + 2, 2);
    }

    // Horizontal guide lines.
    ctx.setLineDash([1, 15]);
    for (let i = 0; i < rows.length; i++) {
        if (rows[i] === null) continue;
        ctx.beginPath();
        const y = cellH * (i + 1);
        ctx.moveTo(width + 16 - offsetX % 16, y - offsetY);
        ctx.lineTo(paddingX, y - offsetY);
        ctx.stroke();
    }

    // Pipeline stages.
    ctx.font = "1rem monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (let i = 0; i < numStages; i++) {
        const stageTrace = trace.pipeline.traces[i];
        for (let c = Math.max(startCycle, 0); c < endCycle; c++) {
            const value = stageTrace[c];
            if (!value) continue;
            const row = rowOf[value - 1];
            if (row < 0) continue;
            const y = (row + 1) * cellH - offsetY;
            const x = c * cellW - offsetX + paddingX;
            ctx.fillStyle = colors[i % colors.length];
            ctx.fillRect(x, y, cellW, cellH);
            ctx.fillStyle = "#000";
            ctx.fillText(trace.pipeline.keys[i], x + cellW / 2, y + cellH / 2);
        }
    }

    ctx.textAlign = "left";
    ctx.textBaseline = "middle";
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, 0, paddingX - 1, height);
    ctx.fillStyle = "#000";
    for (let i = 0; i < rows.length; i++) {
        const r = rows[i];
        ctx.fillText(
            r === null ? "..." : trace.instructions[r],
            textPad,
            cellH * (i + 1) + cellH / 2 - offsetY,
        );
    }
}

/** Width of the widest instruction label among `rows`. */
export function measureRows(trace: Trace, rows: Row[]): number {
    const ctx = document.createElement("canvas").getContext("2d")!;
    ctx.font = "1rem monospace";
    return rows.reduce<number>(
        (max, r) =>
            Math.max(
                max,
                ctx.measureText(r === null ? "..." : trace.instructions[r])
                    .width,
            ),
        0,
    );
}

export function pipelineCellSize(): number {
    return convertRemToPixels(2);
}

export const pipelineTextPad = textPad;

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const easeInOut = (t: number) =>
    t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;

function convertRemToPixels(rem: number): number {
    return rem *
        parseFloat(getComputedStyle(document.documentElement).fontSize);
}

function clamp(val: number, min: number, max: number) {
    return Math.min(Math.max(val, min), max);
}
