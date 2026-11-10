import { colors } from "../colors.ts";
import type { Trace } from "../types.ts";
import type { ViewContext, ViewInstance, ViewSpec } from "../layout/types.ts";

/** Percentage of the run a stage held an instruction. */
function occupancy(trace: Trace, stage: number): number {
    const busy = trace.pipeline.traces[stage].reduce(
        (a, v) => (v !== 0 ? a + 1 : a),
        0,
    );
    return Math.round((busy / trace.pipeline.cycles) * 10000) / 100;
}

function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className !== undefined) node.className = className;
    return node;
}

/** Cycle position and per-stage occupancy for the current trace. */
export class StatsView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title = "Pipeline Statistics";

    private readonly cycleNow: HTMLElement;
    private readonly cycleTotal: HTMLElement;
    private readonly stages: HTMLElement;

    constructor(ctx: ViewContext) {
        this.root = el("div", "ws-stats");

        const cycle = el("div", "ws-stats__cycle");
        const label = el("span", "ws-stats__label");
        label.textContent = "Cycle";
        this.cycleNow = el("span", "ws-stats__now");
        const sep = el("span", "ws-stats__sep");
        sep.textContent = "/";
        this.cycleTotal = el("span", "ws-stats__total");
        cycle.append(label, this.cycleNow, sep, this.cycleTotal);

        this.stages = el("div", "ws-stats__stages");
        this.root.append(cycle, this.stages);

        this.onCycleChange(ctx.currentCycle);
        if (ctx.trace === null) this.showEmpty();
        else this.onTraceChange(ctx.trace);
    }

    onCycleChange(cycle: number): void {
        this.cycleNow.textContent = cycle.toString();
    }

    onTraceChange(trace: Trace): void {
        this.cycleTotal.textContent = trace.pipeline.cycles.toString();
        this.stages.replaceChildren();
        for (const [i, key] of trace.pipeline.keys.entries()) {
            const row = el("div", "ws-stats__stage");
            const swatch = el("span", "ws-stats__swatch");
            swatch.style.backgroundColor = colors[i];
            const name = el("span", "ws-stats__name");
            name.textContent = key;
            const pct = el("span", "ws-stats__pct");
            pct.textContent = `${occupancy(trace, i)}%`;
            row.append(swatch, name, pct);
            this.stages.append(row);
        }
    }

    private showEmpty(): void {
        this.cycleTotal.textContent = "0";
        const hint = el("p", "ws-stats__hint");
        hint.textContent = "No trace yet.";
        this.stages.replaceChildren(hint);
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
