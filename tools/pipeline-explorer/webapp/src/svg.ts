import type { Trace } from "./types.ts";
import { EMPTY_PIPELINE, dataOf } from "./types.ts";
import {
    type Ctx2D,
    measureRows,
    pipelineCellSize,
    pipelineTextPad,
    renderPipeline,
    type Row,
} from "./pipeline.ts";

/** Records the canvas calls `renderPipeline` makes and replays them as SVG
 *  elements, so the exported image is drawn by the same code as the canvas. */
class SvgContext implements Ctx2D {
    fillStyle: string | CanvasGradient | CanvasPattern = "#000";
    font: string = "10px sans-serif";
    textAlign: CanvasTextAlign = "start";
    textBaseline: CanvasTextBaseline = "alphabetic";

    out: string[] = [];
    path: string = "";
    dash: number[] = [];

    fillRect(x: number, y: number, w: number, h: number) {
        this.out.push(
            `<rect x="${n(x)}" y="${n(y)}" width="${n(w)}" height="${
                n(h)
            }" fill="${this.fillStyle}"/>`,
        );
    }

    fillText(text: string, x: number, y: number) {
        const anchor = { center: "middle", right: "end", end: "end" }[
            this.textAlign as string
        ] ?? "start";
        const baseline = {
            top: "text-before-edge",
            hanging: "hanging",
            middle: "central",
            bottom: "text-after-edge",
        }[this.textBaseline as string] ?? "alphabetic";
        this.out.push(
            `<text x="${n(x)}" y="${n(y)}" font-size="${
                fontPx(this.font)
            }" text-anchor="${anchor}" dominant-baseline="${baseline}" fill="${this.fillStyle}">${
                escapeXml(text)
            }</text>`,
        );
    }

    beginPath() {
        this.path = "";
    }
    moveTo(x: number, y: number) {
        this.path += `M${n(x)} ${n(y)}`;
    }
    lineTo(x: number, y: number) {
        this.path += `L${n(x)} ${n(y)}`;
    }
    stroke() {
        const dash = this.dash.length
            ? ` stroke-dasharray="${this.dash.join(" ")}"`
            : "";
        this.out.push(`<path d="${this.path}" stroke="#000"${dash}/>`);
    }
    setLineDash(segments: number[]) {
        this.dash = [...segments];
    }
}

const n = (v: number) => Number(v.toFixed(2)).toString();

/** Escapes `s` for XML text. Spaces become non-breaking so the column
 *  alignment of the instruction labels survives every SVG renderer's
 *  whitespace collapsing. */
function escapeXml(s: string): string {
    return s.replace(
        /[&<>" ]/g,
        (c) =>
            ({
                "&": "&amp;",
                "<": "&lt;",
                ">": "&gt;",
                '"': "&quot;",
                " ": "&#160;",
            })[c]!,
    );
}

/** Canvas font shorthand ("0.75rem monospace") to a pixel size. */
function fontPx(font: string): number {
    const m = /([\d.]+)(px|rem)/.exec(font);
    if (!m) return 16;
    const v = parseFloat(m[1]);
    return m[2] === "rem"
        ? v * parseFloat(getComputedStyle(document.documentElement).fontSize)
        : v;
}

/** Instruction indices that occupy any stage in cycles [start, end]. */
function usedInstructions(trace: Trace, start: number, end: number): number[] {
    const used = new Set<number>();
    for (const stageTrace of (dataOf(trace, "pipeline") ?? EMPTY_PIPELINE).traces) {
        for (let c = start; c <= end && c < stageTrace.length; c++) {
            if (stageTrace[c]) used.add(stageTrace[c] - 1);
        }
    }
    return [...used].sort((a, b) => a - b);
}

/** The display rows when limiting the PC domain: every used instruction plus
 *  `spacing` neighbours on either side. Each skipped run becomes a `...` row,
 *  unless it is a single instruction, which takes up the same space anyway. */
export function limitedRows(
    trace: Trace,
    start: number,
    end: number,
    spacing: number,
): Row[] {
    const total = trace.instructions.length;
    const keep = new Uint8Array(total);
    for (const i of usedInstructions(trace, start, end)) {
        const lo = Math.max(0, i - spacing);
        const hi = Math.min(total - 1, i + spacing);
        for (let j = lo; j <= hi; j++) keep[j] = 1;
    }

    const rows: Row[] = [];
    let i = 0;
    while (i < total) {
        if (keep[i]) {
            rows.push(i++);
            continue;
        }
        let j = i;
        while (j < total && !keep[j]) j++;
        if (j - i === 1) rows.push(i);
        else rows.push(null);
        i = j;
    }
    return rows;
}

export type SvgOptions = {
    start: number;
    end: number;
    /** `null` keeps every instruction row. */
    spacing: number | null;
};

export function pipelineSvg(trace: Trace, opts: SvgOptions): string {
    const { start, end } = opts;
    const rows: Row[] = opts.spacing === null
        ? trace.instructions.map((_, i) => i)
        : limitedRows(trace, start, end, opts.spacing);

    const cell = pipelineCellSize();
    const maxTextWidth = measureRows(trace, rows);
    const paddingX = maxTextWidth + 2 * pipelineTextPad;
    const width = Math.ceil(paddingX + (end - start + 1) * cell);
    const height = Math.ceil((rows.length + 1) * cell);

    const ctx = new SvgContext();
    renderPipeline(ctx, trace, {
        rows,
        startCycle: start,
        endCycle: end + 1,
        offsetX: start * cell,
        offsetY: 0,
        width,
        height,
        maxTextWidth,
        selection: null,
    });

    return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" font-family="monospace">
<rect width="100%" height="100%" fill="#fff"/>
${ctx.out.join("\n")}
</svg>
`;
}
