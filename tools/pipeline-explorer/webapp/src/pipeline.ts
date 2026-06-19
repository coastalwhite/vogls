import { colors } from "./colors.ts";
import type { Trace } from "./lib/types.ts";

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

    currentCycle: number;
    goToCycle: (cycle: number) => void;

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
        currentCycle: number,
        goToCycle: (cycle: number) => void,
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

        this.currentCycle = currentCycle;
        this.goToCycle = goToCycle;

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

        this.canvas.addEventListener("click", (e) => this.onClick(e));

        this.draw();
    }

    setTrace(trace: Trace) {
        this.trace = trace;
        this.maxTextWidth = this.calculateMaxTextWidth();
        this.offsetX = this.clampOffsetX(this.offsetX);
        this.offsetY = this.clampOffsetY(this.offsetY);
        this.draw();
    }
    setCurrentCycle(cycle: number) {
        this.currentCycle = cycle;
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

        const numStages = this.trace.pipeline.keys.length;
        const canvasWidth = Math.round(this.width * dpr);
        const canvasHeight = Math.round(this.height * dpr);
        if (this.canvas.width !== canvasWidth) this.canvas.width = canvasWidth;
        if (this.canvas.height !== canvasHeight) {
            this.canvas.height = canvasHeight;
        }

        const ctx = this.canvas.getContext("2d")!;
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, this.width, this.height);

        const pxHeight = (this.trace.instructions.length + 1) *
            convertRemToPixels(2);

        const paddingX = this.maxTextWidth + textPad * 2;

        const cellW = convertRemToPixels(2);
        const cellH = convertRemToPixels(2);

        const numWidthCells = this.width / cellW;

        const startX = Math.floor(this.offsetX / cellW) - 1;
        const endX = Math.ceil(startX + numWidthCells) + 2;

        if (this.currentCycle >= startX && this.currentCycle <= endX) {
            const baseX = this.currentCycle * cellW + paddingX;
            const baseY = 0;

            const x = baseX - this.offsetX;
            const y = baseY - this.offsetY;

            ctx.fillStyle = "#ccc";
            ctx.fillRect(x, y, convertRemToPixels(2), this.height);
        }

        // Vertical guide lines.
        ctx.fillStyle = "#000";
        ctx.font = "0.75rem monospace";
        ctx.textBaseline = "top";
        ctx.setLineDash([5, 15]);
        for (let c = startX - (startX % 5); c < endX; c += 5) {
            const baseX = c * cellW + paddingX;
            const baseY = 0;

            const x = baseX - this.offsetX;
            const y = baseY - this.offsetY;

            ctx.beginPath();
            ctx.moveTo(x, y);
            ctx.lineTo(x, y + pxHeight);
            ctx.stroke();

            ctx.fillText(c.toString(), x + 2, baseY + 2);
        }

        // Horizontal guide lines.
        ctx.setLineDash([1, 15]);
        for (let i = 0; i < this.trace.instructions.length; i++) {
            ctx.beginPath();
            const y = cellH * (i + 1);
            ctx.moveTo(this.width + 16 - this.offsetX % 16, y - this.offsetY);
            ctx.lineTo(paddingX, y - this.offsetY);
            ctx.stroke();
        }

        // Pipeline stages.
        ctx.font = "1rem monospace";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        for (let i = 0; i < numStages; i++) {
            const stageTrace = this.trace.pipeline.traces[i];
            for (let c = startX; c < endX; c++) {
                const value = stageTrace[c];
                if (value === 0) continue;
                const y = value * cellH - this.offsetY;
                const x = c * cellW - this.offsetX + paddingX;
                ctx.fillStyle = colors[i % colors.length];
                ctx.fillRect(x, y, cellW, cellH);
                ctx.fillStyle = "#000";
                ctx.fillText(
                    this.trace.pipeline.keys[i],
                    x + cellW / 2,
                    y + cellH / 2,
                );
            }
        }

        ctx.textAlign = "left";
        ctx.textBaseline = "middle";
        ctx.fillStyle = "#fff";
        ctx.fillRect(0, 0, paddingX - 1, this.height);
        ctx.fillStyle = "#000";
        for (let i = 0; i < this.trace.instructions.length; i++) {
            ctx.fillText(
                this.trace.instructions[i],
                textPad,
                cellW * (i + 1) + cellW / 2 - this.offsetY,
            );
        }
    }

    onClick(e: MouseEvent) {
        console.log(e.offsetX, e.offsetY)
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
