import { dataOf } from "../types.ts";
import type { RegistersData, Trace } from "../types.ts";
import { Table } from "../table.ts";
import type { Row as TableRow } from "../table.ts";
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

const hex = (v: number) => `0x${(v >>> 0).toString(16).padStart(8, "0")}`;

/** Signed reading, which is what most RISC-V values are meant as. */
const dec = (v: number) => (v | 0).toString();

/**
 * The register file: its values at the selected cycle, or what changed across
 * the selection when a span is picked.
 */
export class RegistersView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title = "Registers";

    private readonly caption: HTMLElement;
    private readonly table: Table;

    private trace: Trace | null = null;
    private cycles: CycleRange;

    constructor(ctx: ViewContext) {
        this.cycles = ctx.cycles;
        this.trace = ctx.trace;

        this.root = el("div", "ws-regs");
        this.caption = el("p", "ws-regs__caption");
        // The third column stays even at a single cycle, so switching between
        // a value and a difference does not reflow the first two.
        this.table = new Table([
            { header: "Register", width: "5rem" },
            { header: "Value", align: "right", numeric: true, width: "8rem" },
            { header: "Previous", align: "right", numeric: true, width: "9rem" },
        ]);
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

    /** The value of register `i` at `cycle`, clamped to the run. */
    private at(regs: RegistersData, i: number, cycle: number): number {
        const row = regs.values[i];
        if (row === undefined || row.length === 0) return 0;
        const c = Math.min(Math.max(cycle, 0), row.length - 1);
        return row[c] >>> 0;
    }

    private render(): void {
        const regs = dataOf(this.trace, "registers");
        if (regs === null) {
            this.caption.textContent = this.trace === null
                ? "No trace yet."
                : "This core does not report its register file.";
            this.table.setMessage("No registers.");
            return;
        }

        const { start, end } = this.cycles;
        const diff = end > start;

        const rows: TableRow[] = [];
        let changed = 0;
        for (const [i, name] of regs.names.entries()) {
            const to = this.at(regs, i, end);
            const from = this.at(regs, i, start);
            const moved = diff && from !== to;
            if (moved) changed += 1;

            // Every register stays on screen; only the movers are called out.
            rows.push({
                className: moved ? "is-changed" : to !== 0 ? "is-set" : "",
                cells: [
                    name,
                    { text: hex(to), title: `${dec(to)} (${to >>> 0})`, className: "ws-regs__value" },
                    moved
                        ? { text: `\u2190 ${hex(from)}`, title: `was ${dec(from)}`, className: "ws-regs__was" }
                        : "",
                ],
            });
        }

        this.caption.textContent = diff
            ? `Cycles ${start}\u2013${end} \u2014 ${
                changed === 0 ? "nothing changed" : `${changed} changed`
            }`
            : `Values at cycle ${start}`;
        this.table.setRows(rows);
    }

    destroy(): void {
        this.root.remove();
    }
}

export const registersSpec: ViewSpec = {
    type: "registers",
    label: "Registers",
    create: (_view: unknown, ctx: ViewContext) => new RegistersView(ctx),
};
