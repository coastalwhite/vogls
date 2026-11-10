import { EMPTY_PIPELINE, dataOf } from "../types.ts";
import type { PipelineData, SymbolsData, Trace } from "../types.ts";
import { Table } from "../table.ts";
import type { Column, Row as TableRow } from "../table.ts";
import type {
    CycleRange,
    ViewContext,
    ViewInstance,
    ViewSpec,
} from "../layout/types.ts";

function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className !== undefined) node.className = className;
    return node;
}

/** A symbol and the slice of `.text` it owns, up to the next symbol. */
type Region = {
    name: string;
    global: boolean;
    from: number;
    to: number;
    /** Index into `regions` of the global this one sits under, or -1. */
    parent: number;
};

/** Regions for `.text` symbols, in address order, each ending at the next. */
function regionsOf(syms: SymbolsData, textEnd: number): Region[] {
    const regions: Region[] = [];
    for (const [i, name] of syms.names.entries()) {
        if (syms.sections[i] !== ".text") continue;
        regions.push({
            name,
            global: syms.globals[i] === 1,
            from: syms.addresses[i] >>> 0,
            to: 0,
            parent: -1,
        });
    }
    regions.sort((a, b) => a.from - b.from);
    for (const [i, r] of regions.entries()) {
        r.to = i + 1 < regions.length ? regions[i + 1].from : Math.max(textEnd, r.from);
    }
    // A label belongs to the last global at or before it; labels ahead of the
    // first global stand on their own.
    let current = -1;
    for (const [i, r] of regions.entries()) {
        if (r.global) current = i;
        else r.parent = current;
    }
    return regions;
}

/** Per-region, per-stage cycle counts over `range`. */
function tally(
    pipeline: PipelineData,
    regions: Region[],
    range: CycleRange,
): number[][] {
    const counts = regions.map(() => pipeline.keys.map(() => 0));
    if (regions.length === 0) return counts;

    const last = pipeline.cycles - 1;
    const from = Math.max(0, Math.min(range.start, last));
    const to = Math.max(from, Math.min(range.end, last));

    for (const [stage, cells] of pipeline.traces.entries()) {
        for (let c = from; c <= to; c++) {
            const slot = cells[c];
            // Slot 0 means the stage was idle this cycle.
            if (slot === 0) continue;
            const addr = pipeline.textAddr + 4 * (slot - 1);
            // Regions are contiguous and sorted, so a scan from the back finds
            // the owner without a per-cycle search over all of them.
            for (let r = regions.length - 1; r >= 0; r--) {
                if (addr >= regions[r].from) {
                    if (addr < regions[r].to) counts[r][stage] += 1;
                    break;
                }
            }
        }
    }
    return counts;
}

/** Cycles each symbol occupied every pipeline stage, globals first. */
export class ProfileView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title = "Symbol Profile";

    private readonly caption: HTMLElement;
    private readonly table: Table;
    private trace: Trace | null = null;
    private cycles: CycleRange;
    /** Globals whose labels are unfolded. Ephemeral, per view. */
    private expanded = new Set<string>();

    constructor(ctx: ViewContext) {
        this.trace = ctx.trace;
        this.cycles = ctx.cycles;
        this.root = el("div", "ws-prof");
        this.caption = el("p", "ws-prof__caption");
        this.table = new Table([]);
        this.root.append(this.caption, this.table.element);
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

    private render(): void {
        const pipeline = dataOf(this.trace, "pipeline") ?? EMPTY_PIPELINE;
        const syms = dataOf(this.trace, "symbols");

        const columns: Column[] = [
            // Cycles summed over every stage: one instruction can occupy
            // several at once, so this is stage-cycles, not wall cycles. The
            // percentage is this symbol's share of all of them, which makes
            // the top-level rows add up to 100%.
            { header: "Total", align: "right", numeric: true, width: "7rem" },
            { header: "Symbol", width: "9rem" },
            ...pipeline.keys.map((key): Column => ({
                header: key,
                align: "right",
                numeric: true,
                width: "7rem",
            })),
        ];
        this.table.setColumns(columns);

        if (syms === null || pipeline.cycles === 0) {
            this.caption.textContent = this.trace === null
                ? "No trace yet."
                : "Needs both pipeline and symbol data.";
            this.table.setMessage("Nothing to profile.");
            return;
        }

        const instructions = this.trace?.instructions.length ?? 0;
        const regions = regionsOf(syms, pipeline.textAddr + 4 * instructions);

        // Profiling one cycle says almost nothing, so a collapsed selection
        // means the whole run rather than a single-cycle snapshot.
        const last = pipeline.cycles - 1;
        const whole = this.cycles.start === this.cycles.end;
        const from = whole ? 0 : Math.max(0, Math.min(this.cycles.start, last));
        const to = whole ? last : Math.max(from, Math.min(this.cycles.end, last));
        const span = to - from + 1;
        const counts = tally(pipeline, regions, { start: from, end: to });

        this.caption.textContent = whole
            ? `Whole program \u2014 ${span} cycle${span === 1 ? "" : "s"}`
            : `${span} cycle${span === 1 ? "" : "s"} (${from}\u2013${to})`;

        // A global covers its own labels, which are listed under it.
        const rolled = regions.map((region, i) => {
            const own = counts[i].slice();
            if (region.parent < 0) {
                for (const [j, child] of regions.entries()) {
                    if (child.parent === i) {
                        for (const [stage, n] of counts[j].entries()) own[stage] += n;
                    }
                }
            }
            return own;
        });
        const totals = rolled.map((own) => own.reduce((sum, n) => sum + n, 0));
        const allCycles = counts.reduce(
            (sum, row) => sum + row.reduce((a, b) => a + b, 0),
            0,
        );

        const cell = (value: number, of: number) => {
            if (value === 0) return { text: "\u2013", className: "is-zero" };
            const pct = of === 0 ? 0 : Math.round((value / of) * 1000) / 10;
            return { text: `${value} (${pct}%)` };
        };

        const rowFor = (i: number, isChild: boolean): TableRow => {
            const region = regions[i];
            const hasChildren = regions.some((r) => r.parent === i);
            const open = this.expanded.has(region.name);
            return {
                className: region.global ? "is-global" : isChild ? "is-child" : "",
                treeColumn: 1,
                depth: isChild ? 1 : 0,
                fold: hasChildren
                    ? {
                        open,
                        label: `${open ? "Hide" : "Show"} labels in ${region.name}`,
                        onToggle: () => {
                            if (open) this.expanded.delete(region.name);
                            else this.expanded.add(region.name);
                            this.render();
                        },
                    }
                    : undefined,
                cells: [
                    {
                        ...cell(totals[i], allCycles),
                        className: totals[i] === 0 ? "is-zero" : "ws-prof__total",
                    },
                    {
                        text: region.name,
                        title: region.name,
                        className: "ws-prof__name",
                    },
                    ...rolled[i].map((value) => cell(value, span)),
                ],
            };
        };

        // Costliest first, at both levels.
        const byTotal = (a: number, b: number) => totals[b] - totals[a];
        const rows: TableRow[] = [];
        const top = regions.map((_, i) => i).filter((i) => regions[i].parent < 0);
        for (const i of top.sort(byTotal)) {
            rows.push(rowFor(i, false));
            if (!this.expanded.has(regions[i].name)) continue;
            const children = regions
                .map((_, j) => j)
                .filter((j) => regions[j].parent === i)
                .sort(byTotal);
            for (const j of children) rows.push(rowFor(j, true));
        }
        this.table.setRows(rows);
    }

    destroy(): void {
        this.root.remove();
    }
}

export const profileSpec: ViewSpec = {
    type: "profile",
    label: "Symbol Profile",
    create: (_view: unknown, ctx: ViewContext) => new ProfileView(ctx),
};
