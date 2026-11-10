import { colors } from "../colors.ts";
import type { Trace } from "../types.ts";
import type {
    CycleRange,
    ViewContext,
    ViewInstance,
    ViewSpec,
} from "../layout/types.ts";
import { cycleCount } from "../layout/types.ts";

/** Percentage of the selected cycles a stage held an instruction. */
function occupancy(trace: Trace, stage: number, range: CycleRange): number {
    const last = trace.pipeline.cycles - 1;
    const from = Math.max(0, Math.min(range.start, last));
    const to = Math.max(from, Math.min(range.end, last));
    const cells = trace.pipeline.traces[stage];
    let busy = 0;
    for (let c = from; c <= to; c++) {
        if (cells[c] !== 0) busy += 1;
    }
    return Math.round((busy / (to - from + 1)) * 10000) / 100;
}

function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className !== undefined) node.className = className;
    return node;
}

/** Selected cycles and per-stage occupancy across them. */
export class StatsView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title = "Pipeline Statistics";

    private readonly ctx: ViewContext;
    private readonly label: HTMLElement;
    private readonly startInput: HTMLInputElement;
    private readonly endInput: HTMLInputElement;
    private readonly cycleTotal: HTMLElement;
    private readonly span: HTMLElement;
    private readonly stages: HTMLElement;

    private trace: Trace | null = null;
    private cycles: CycleRange;

    constructor(ctx: ViewContext) {
        this.ctx = ctx;
        this.cycles = ctx.cycles;
        this.root = el("div", "ws-stats");

        const cycle = el("div", "ws-stats__cycle");
        this.label = el("span", "ws-stats__label");
        this.startInput = this.cycleInput("Start cycle");
        const dash = el("span", "ws-stats__dash");
        dash.textContent = "–";
        this.endInput = this.cycleInput("End cycle");
        const sep = el("span", "ws-stats__sep");
        sep.textContent = "/";
        this.cycleTotal = el("span", "ws-stats__total");
        cycle.append(
            this.label,
            this.startInput,
            dash,
            this.endInput,
            sep,
            this.cycleTotal,
        );

        this.span = el("div", "ws-stats__span");
        this.stages = el("div", "ws-stats__stages");
        this.root.append(cycle, this.span, this.stages);

        this.trace = ctx.trace;
        this.render();
    }

    onCycleChange(cycles: CycleRange): void {
        this.cycles = cycles;
        this.render();
    }

    onTraceChange(trace: Trace): void {
        this.trace = trace;
        this.render();
    }

    private cycleInput(label: string): HTMLInputElement {
        const input = el("input", "ws-stats__input");
        input.type = "number";
        input.min = "0";
        input.step = "1";
        input.setAttribute("aria-label", label);
        // `change`, not `input`: committing per keystroke would select cycle 1
        // on the way to typing 12.
        input.addEventListener("change", () => this.commit(input));
        return input;
    }

    /** Reads both boxes back, clamped to the trace, and selects the result. */
    private commit(edited: HTMLInputElement): void {
        if (this.trace === null) return;
        const last = this.trace.pipeline.cycles - 1;
        const read = (input: HTMLInputElement, fallback: number): number => {
            const v = input.valueAsNumber;
            return Number.isFinite(v)
                ? Math.min(Math.max(Math.round(v), 0), last)
                : fallback;
        };
        let start = read(this.startInput, this.cycles.start);
        let end = read(this.endInput, this.cycles.end);
        // Pushing the other end along beats refusing to move, which is what
        // the share panel's range controls already do.
        if (edited === this.startInput && start > end) end = start;
        if (edited === this.endInput && end < start) start = end;
        this.ctx.selectCycles({ start, end });
    }

    private render(): void {
        const { start, end } = this.cycles;
        const ranged = end > start;
        const total = this.trace?.pipeline.cycles ?? 0;
        this.label.textContent = ranged ? "Cycles" : "Cycle";
        this.cycleTotal.textContent = total.toString();

        const max = Math.max(0, total - 1).toString();
        for (const [input, value] of [
            [this.startInput, start],
            [this.endInput, end],
        ] as const) {
            input.max = max;
            input.disabled = this.trace === null;
            // Never overwrite the box being typed into.
            if (document.activeElement !== input) {
                input.value = value.toString();
            }
        }
        // Only meaningful for a span; a single cycle already reads as one.
        this.span.textContent = ranged
            ? `${cycleCount(this.cycles)} cycles selected`
            : "";
        this.span.hidden = !ranged;

        if (this.trace === null) {
            const hint = el("p", "ws-stats__hint");
            hint.textContent = "No trace yet.";
            this.stages.replaceChildren(hint);
            return;
        }

        const rows: HTMLElement[] = [];
        for (const [i, key] of this.trace.pipeline.keys.entries()) {
            const row = el("div", "ws-stats__stage");
            const swatch = el("span", "ws-stats__swatch");
            swatch.style.backgroundColor = colors[i];
            const name = el("span", "ws-stats__name");
            name.textContent = key;
            const pct = el("span", "ws-stats__pct");
            pct.textContent = `${occupancy(this.trace, i, this.cycles)}%`;
            row.append(swatch, name, pct);
            rows.push(row);
        }
        this.stages.replaceChildren(...rows);
    }

    destroy(): void {
        this.root.remove();
    }
}

export const statsSpec: ViewSpec = {
    type: "stats",
    label: "Pipeline Statistics",
    create: (_view: unknown, ctx: ViewContext) => new StatsView(ctx),
};
