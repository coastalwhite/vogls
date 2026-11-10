/**
 * Shared popover and menu primitive.
 *
 * Every popover and menu in the app goes through here, so there is exactly one
 * document-level `click` listener, one document-level `keydown` listener and one
 * "currently open" reference. Opening anything closes whatever was open before,
 * which is what keeps sibling popovers from having to know about each other.
 */

export type MenuItem =
    | { kind: "item"; label: string; disabled?: boolean; onSelect(): void }
    | { kind: "submenu"; label: string; items: MenuItem[] }
    | { kind: "separator" };

export interface PopoverHandle {
    readonly isOpen: boolean;
    open(): void;
    close(): void;
    toggle(): void;
}

/** Everything the two document-level listeners need from what is open. */
interface OpenEntry {
    /** Is `node` part of this popover — its panel, a menu level, its trigger? */
    owns(node: Node): boolean;
    close(): void;
    onKeyDown(event: KeyboardEvent): void;
}

let current: OpenEntry | null = null;
let listening = false;

function listen() {
    if (listening) return;
    listening = true;

    document.addEventListener("click", (e) => {
        const entry = current;
        if (entry === null) return;
        const target = e.target;
        if (!(target instanceof Node)) return;
        // A node that has already been detached (a control that re-rendered
        // itself on this very click) cannot be tested for containment any
        // more; assume it was ours rather than closing out from under it.
        if (!target.isConnected) return;
        if (!entry.owns(target)) entry.close();
    });

    document.addEventListener("keydown", (e) => current?.onKeyDown(e));
}

/** Closes whatever is open, if anything. */
export function closeOpenPopover() {
    current?.close();
}

/* ---------- popovers ---------- */

/** Wraps an existing trigger + panel pair (the chip menu, the share panel). */
export function registerPopover(opts: {
    trigger: HTMLElement;
    panel: HTMLElement;
    onOpenChange?(open: boolean): void;
}): PopoverHandle {
    const { trigger, panel } = opts;
    listen();

    const entry: OpenEntry = {
        owns: (node) => panel.contains(node) || trigger.contains(node),
        close: () => closePopover(),
        onKeyDown: (e) => {
            if (e.key !== "Escape") return;
            closePopover();
            trigger.focus();
        },
    };

    function openPopover() {
        if (current === entry) return;
        closeOpenPopover();
        current = entry;
        // Visible before the callback runs: panel contents that redraw
        // themselves on open bail out while the panel is still hidden.
        panel.hidden = false;
        trigger.setAttribute("aria-expanded", "true");
        opts.onOpenChange?.(true);
    }

    function closePopover() {
        if (current !== entry) return;
        current = null;
        panel.hidden = true;
        trigger.setAttribute("aria-expanded", "false");
        opts.onOpenChange?.(false);
    }

    // Start from the closed state whatever the markup said, so `isOpen`,
    // `hidden` and `aria-expanded` cannot disagree.
    panel.hidden = true;
    trigger.setAttribute("aria-expanded", "false");
    trigger.addEventListener("click", () => {
        if (current === entry) closePopover();
        else openPopover();
    });

    return {
        get isOpen() {
            return current === entry;
        },
        open: openPopover,
        close: closePopover,
        toggle() {
            if (current === entry) closePopover();
            else openPopover();
        },
    };
}

/* ---------- menus ---------- */

/** Distance from an anchor to the menu it opens. */
const ANCHOR_GAP = 2;
/** Keep-out band at the viewport edges. */
const EDGE_MARGIN = 4;

interface MenuRow {
    item: MenuItem;
    el: HTMLElement;
}

interface MenuLevel {
    el: HTMLElement;
    rows: MenuRow[];
    /** Keyboard-focused row, or -1. */
    active: number;
    /** The submenu row this level hangs off, for `aria-expanded`. */
    parentRow: HTMLElement | null;
}

interface MenuState {
    anchor: HTMLElement;
    /** Root menu first, deepest open submenu last. */
    levels: MenuLevel[];
    entry: OpenEntry;
}

let openMenuState: MenuState | null = null;

/** Builds and opens a transient menu anchored to `anchor`. */
export function openMenu(anchor: HTMLElement, items: MenuItem[]): void {
    if (items.length === 0) return;
    // Typed non-null, but really is null while the document head is parsing.
    const body: HTMLElement | null = document.body;
    if (body === null) return;

    // Re-opening on the same anchor means its trigger was clicked again.
    const sameAnchor = openMenuState?.anchor === anchor;
    closeOpenPopover();
    if (sameAnchor) return;

    listen();

    const levels: MenuLevel[] = [];
    const entry: OpenEntry = {
        owns: (node) =>
            anchor.contains(node) || levels.some((level) => level.el.contains(node)),
        close: closeMenu,
        onKeyDown: onMenuKeyDown,
    };
    openMenuState = { anchor, levels, entry };
    current = entry;
    // Lets a trigger style itself while its menu is down.
    if (anchor.hasAttribute("aria-haspopup")) {
        anchor.setAttribute("aria-expanded", "true");
    }

    const root = buildLevel(items, anchor.getBoundingClientRect(), false, null);
    levels.push(root);
    root.el.focus();
}

function closeMenu() {
    const state = openMenuState;
    if (state === null) return;
    openMenuState = null;
    if (current === state.entry) current = null;
    if (state.anchor.hasAttribute("aria-haspopup")) {
        state.anchor.setAttribute("aria-expanded", "false");
    }

    const active = document.activeElement;
    const hadFocus =
        active instanceof Node && state.levels.some((level) => level.el.contains(active));
    for (const level of state.levels) level.el.remove();
    if (hadFocus) state.anchor.focus();
}

/** Index of `level` in the open stack, or -1 if it is stale. */
function depthOf(level: MenuLevel): number {
    return openMenuState?.levels.indexOf(level) ?? -1;
}

/** Closes every level from `from` down. */
function truncate(from: number) {
    const state = openMenuState;
    if (state === null) return;
    let hadFocus = false;
    while (state.levels.length > from) {
        const level = state.levels.pop()!;
        const active = document.activeElement;
        hadFocus ||= active instanceof Node && level.el.contains(active);
        level.parentRow?.setAttribute("aria-expanded", "false");
        level.el.remove();
    }
    if (hadFocus) state.levels.at(-1)?.el.focus();
}

function isSelectable(item: MenuItem): boolean {
    return item.kind === "submenu" || (item.kind === "item" && item.disabled !== true);
}

/** Marks a row as the keyboard focus; any open submenu of this level closes. */
function focusRow(level: MenuLevel, index: number) {
    const depth = depthOf(level);
    if (depth < 0) return;
    truncate(depth + 1);
    for (const [i, row] of level.rows.entries()) {
        row.el.classList.toggle("is-active", i === index);
    }
    level.active = index;
}

function openSubmenu(level: MenuLevel, index: number): MenuLevel | null {
    const state = openMenuState;
    const depth = depthOf(level);
    if (state === null || depth < 0) return null;
    const row = level.rows[index];
    if (row.item.kind !== "submenu" || row.item.items.length === 0) return null;

    truncate(depth + 1);
    row.el.setAttribute("aria-expanded", "true");
    const sub = buildLevel(row.item.items, row.el.getBoundingClientRect(), true, row.el);
    state.levels.push(sub);
    return sub;
}

function openSubmenuFocused(level: MenuLevel, index: number) {
    const sub = openSubmenu(level, index);
    if (sub === null) return;
    sub.el.focus();
    move(sub, 1);
}

function select(level: MenuLevel, index: number) {
    const item = level.rows[index].item;
    if (item.kind !== "item" || item.disabled === true) return;
    // The whole stack goes away before the callback runs: it is free to drop
    // the anchor, or to open a menu of its own.
    closeMenu();
    item.onSelect();
}

function hover(level: MenuLevel, index: number) {
    if (level.active === index) return;
    const item = level.rows[index].item;
    if (!isSelectable(item)) {
        focusRow(level, -1);
        return;
    }
    focusRow(level, index);
    if (item.kind === "submenu") openSubmenu(level, index);
}

/** Moves the keyboard focus by `delta`, skipping separators and disabled rows. */
function move(level: MenuLevel, delta: number) {
    const count = level.rows.length;
    let index = level.active >= 0 ? level.active : delta > 0 ? -1 : 0;
    for (let step = 0; step < count; step++) {
        index = (((index + delta) % count) + count) % count;
        if (isSelectable(level.rows[index].item)) {
            focusRow(level, index);
            return;
        }
    }
}

function onMenuKeyDown(e: KeyboardEvent) {
    const state = openMenuState;
    if (state === null) return;
    const level = state.levels.at(-1);
    if (level === undefined) return;
    const item = level.active >= 0 ? level.rows[level.active].item : null;

    switch (e.key) {
        case "ArrowDown":
            move(level, 1);
            break;
        case "ArrowUp":
            move(level, -1);
            break;
        case "ArrowRight":
            if (item?.kind !== "submenu") return;
            openSubmenuFocused(level, level.active);
            break;
        case "ArrowLeft":
            if (state.levels.length < 2) return;
            truncate(state.levels.length - 1);
            break;
        case "Enter":
        case " ":
            if (item === null) return;
            if (item.kind === "submenu") openSubmenuFocused(level, level.active);
            else select(level, level.active);
            break;
        case "Escape":
            if (state.levels.length > 1) truncate(state.levels.length - 1);
            else closeMenu();
            break;
        default:
            return;
    }
    // Only for keys actually handled above: arrows would scroll the page and
    // Space would activate the trigger underneath.
    e.preventDefault();
}

function buildLevel(
    items: MenuItem[],
    anchorRect: DOMRect,
    side: boolean,
    parentRow: HTMLElement | null,
): MenuLevel {
    const el = document.createElement("div");
    el.className = "menu";
    el.setAttribute("role", "menu");
    el.tabIndex = -1;
    // Keep the pointer from blurring the menu: focus stays inside, so closing
    // can hand it back to the anchor.
    el.addEventListener("mousedown", (e) => e.preventDefault());

    const level: MenuLevel = { el, rows: [], active: -1, parentRow };

    for (const item of items) {
        const index = level.rows.length;
        const row = document.createElement("div");
        if (item.kind === "separator") {
            row.className = "menu__separator";
            row.setAttribute("role", "separator");
        } else {
            row.className = "menu__item";
            row.setAttribute("role", "menuitem");
            row.textContent = item.label;
            if (item.kind === "submenu") {
                row.setAttribute("aria-haspopup", "menu");
                row.setAttribute("aria-expanded", "false");
                const marker = document.createElement("span");
                marker.className = "menu__submenu-marker";
                marker.textContent = "▸";
                row.append(marker);
            } else if (item.disabled === true) {
                row.classList.add("is-disabled");
                row.setAttribute("aria-disabled", "true");
            }
        }
        level.rows.push({ item, el: row });
        el.append(row);

        if (item.kind === "separator") continue;
        row.addEventListener("mouseenter", () => hover(level, index));
        row.addEventListener("click", () => {
            // `hover` is a no-op once this row is the active one, so clicking a
            // submenu parent the pointer is already on does not rebuild it.
            if (level.rows[index].item.kind === "submenu") hover(level, index);
            else select(level, index);
        });
    }

    document.body.append(el);
    place(el, anchorRect, side);
    return level;
}

/** Positions a menu below-right of (or beside) `anchorRect`, flipping to fit. */
function place(el: HTMLElement, anchorRect: DOMRect, side: boolean) {
    const width = el.offsetWidth;
    const height = el.offsetHeight;
    const viewWidth = document.documentElement.clientWidth;
    const viewHeight = document.documentElement.clientHeight;

    let left = side ? anchorRect.right : anchorRect.left;
    if (left + width > viewWidth - EDGE_MARGIN) {
        const flipped = (side ? anchorRect.left : anchorRect.right) - width;
        left = flipped < EDGE_MARGIN ? Math.max(EDGE_MARGIN, viewWidth - EDGE_MARGIN - width) : flipped;
    }

    let top = side ? anchorRect.top : anchorRect.bottom + ANCHOR_GAP;
    if (top + height > viewHeight - EDGE_MARGIN) {
        const flipped = side ? anchorRect.bottom - height : anchorRect.top - ANCHOR_GAP - height;
        top = flipped < EDGE_MARGIN ? Math.max(EDGE_MARGIN, viewHeight - EDGE_MARGIN - height) : flipped;
    }

    // The rects above are in viewport coordinates; an absolutely positioned
    // `.menu` is laid out in page coordinates, a fixed one is not.
    const scrolled = getComputedStyle(el).position !== "fixed";
    el.style.left = `${Math.round(left + (scrolled ? window.scrollX : 0))}px`;
    el.style.top = `${Math.round(top + (scrolled ? window.scrollY : 0))}px`;
}
