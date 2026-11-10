import { dataOf } from "../types.ts";
import type { Trace } from "../types.ts";
import { Table } from "../table.ts";
import type { Row as TableRow } from "../table.ts";
import type { ViewContext, ViewInstance, ViewSpec } from "../layout/types.ts";

function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className !== undefined) node.className = className;
    return node;
}

const hex = (v: number) => `0x${(v >>> 0).toString(16).padStart(8, "0")}`;

/** Every label the assembler placed, with globals called out. */
export class SymbolsView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title = "Symbols";

    private readonly caption: HTMLElement;
    private readonly table: Table;
    private trace: Trace | null = null;

    constructor(ctx: ViewContext) {
        this.trace = ctx.trace;
        this.root = el("div", "ws-syms");
        this.caption = el("p", "ws-syms__caption");
        this.table = new Table([
            { header: "Address", numeric: true, width: "8rem" },
            { header: "Label", width: "14rem" },
            { header: "Section", width: "6rem" },
        ]);
        this.root.append(this.caption, this.table.element);
        this.render();
    }

    onTraceChange(trace: Trace): void {
        this.trace = trace;
        this.render();
    }

    private render(): void {
        const syms = dataOf(this.trace, "symbols");
        if (syms === null) {
            this.caption.textContent = this.trace === null
                ? "No trace yet."
                : "This program defines no labels.";
            this.table.setMessage("No labels.");
            return;
        }

        const rows: TableRow[] = [];
        let globals = 0;
        for (const [i, name] of syms.names.entries()) {
            const isGlobal = syms.globals[i] === 1;
            if (isGlobal) globals += 1;
            rows.push({
                className: isGlobal ? "is-global" : "",
                cells: [
                    { text: hex(syms.addresses[i]), className: "ws-syms__addr" },
                    // A global is the one thing worth spotting in a long list.
                    {
                        text: name,
                        title: isGlobal ? `${name} (global)` : name,
                        className: "ws-syms__name",
                    },
                    { text: syms.sections[i] ?? "", className: "ws-syms__section" },
                ],
            });
        }

        const total = syms.names.length;
        this.caption.textContent =
            `${total} label${total === 1 ? "" : "s"}, ${globals} global${
                globals === 1 ? "" : "s"
            }`;
        this.table.setRows(rows);
    }

    destroy(): void {
        this.root.remove();
    }
}

export const symbolsSpec: ViewSpec = {
    type: "symbols",
    label: "Symbols",
    create: (_view: unknown, ctx: ViewContext) => new SymbolsView(ctx),
};
