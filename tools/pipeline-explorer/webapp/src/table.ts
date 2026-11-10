/**
 * A small table for the data views.
 *
 * Columns are laid out with `table-layout: fixed` and explicit widths, so a
 * cell never resizes because its contents changed -- numbers that tick every
 * cycle stay put instead of shuffling the columns around them.
 */

export type ColumnAlign = "left" | "right";

export type Column = {
    header: string;
    align?: ColumnAlign;
    /** A CSS length. Columns without one share what is left over. */
    width?: string;
    /** Monospace with tabular figures, so digits line up. */
    numeric?: boolean;
};

export type Cell =
    | string
    | { text: string; className?: string; title?: string }
    | { node: HTMLElement; className?: string };

export type Row = {
    cells: Cell[];
    className?: string;
    /** Indents the tree column, for rows nested under another. */
    depth?: number;
    /** Renders a disclosure control in the tree column. */
    fold?: { open: boolean; label: string; onToggle(): void };
    /** Which column carries the indent and fold. The first by default. */
    treeColumn?: number;
};

function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className !== undefined) node.className = className;
    return node;
}

export class Table {
    readonly element: HTMLElement;

    private readonly colgroup = el("colgroup");
    private readonly head = el("thead");
    private readonly body = el("tbody");
    private columns: Column[] = [];

    constructor(columns: Column[]) {
        this.element = el("div", "ws-table");
        const table = el("table");
        table.append(this.colgroup, this.head, this.body);
        this.element.append(table);
        this.setColumns(columns);
    }

    setColumns(columns: Column[]): void {
        this.columns = columns;

        const cols = columns.map((column) => {
            const col = el("col");
            if (column.width !== undefined) col.style.width = column.width;
            return col;
        });
        this.colgroup.replaceChildren(...cols);

        const row = el("tr");
        for (const column of columns) {
            const th = el("th", this.cellClass(column));
            th.textContent = column.header;
            row.append(th);
        }
        this.head.replaceChildren(row);
    }

    setRows(rows: Row[]): void {
        this.body.replaceChildren(...rows.map((row) => this.renderRow(row)));
    }

    /** Shown in place of the rows, for an empty or unavailable table. */
    setMessage(message: string): void {
        const row = el("tr", "ws-table__message");
        const cell = el("td");
        cell.colSpan = Math.max(1, this.columns.length);
        cell.textContent = message;
        row.append(cell);
        this.body.replaceChildren(row);
    }

    private cellClass(column: Column): string {
        const parts = ["ws-table__cell"];
        if (column.align === "right") parts.push("is-right");
        if (column.numeric) parts.push("is-numeric");
        return parts.join(" ");
    }

    private renderRow(row: Row): HTMLElement {
        const tr = el("tr", row.className);
        for (const [i, column] of this.columns.entries()) {
            const td = el("td", this.cellClass(column));
            const cell = row.cells[i];

            if (i === (row.treeColumn ?? 0)) {
                if (row.depth) td.style.paddingLeft = `${row.depth * 1}rem`;
                if (row.fold !== undefined) td.append(this.renderFold(row.fold));
            }

            if (cell === undefined) {
                // A short row leaves the remaining columns blank rather than
                // shifting later cells left.
                tr.append(td);
                continue;
            }
            if (typeof cell === "string") {
                td.append(cell);
            } else if ("node" in cell) {
                if (cell.className !== undefined) td.classList.add(cell.className);
                td.append(cell.node);
            } else {
                if (cell.className !== undefined) td.classList.add(cell.className);
                if (cell.title !== undefined) td.title = cell.title;
                td.append(cell.text);
            }
            tr.append(td);
        }
        return tr;
    }

    private renderFold(fold: NonNullable<Row["fold"]>): HTMLElement {
        const button = el("button", "ws-table__fold");
        button.type = "button";
        button.textContent = fold.open ? "▾" : "▸";
        button.setAttribute("aria-expanded", fold.open ? "true" : "false");
        button.setAttribute("aria-label", fold.label);
        button.addEventListener("click", fold.onToggle);
        return button;
    }
}
