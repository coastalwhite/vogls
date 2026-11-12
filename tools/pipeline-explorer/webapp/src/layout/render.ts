import { axisOf } from "./tree.ts";
import { MIN_PX } from "./types.ts";
import type {
    LayoutNode,
    NodeId,
    StackElement,
    StackNode,
    TabsNode,
    ViewNode,
    WorkspaceLayout,
} from "./types.ts";

export interface RenderContext {
    /** Stable, caller-cached element for a leaf view node or a tabs node. */
    frameFor(node: ViewNode | TabsNode): HTMLElement;
    /** Called while the user drags a divider. `deltaRatio` is in this stack's ratio units. */
    onDividerResize(
        stackId: NodeId,
        dividerIndex: number,
        deltaRatio: number,
    ): void;
    /** Element shown when the layout is `null`. Also caller-cached. */
    emptyPlaceholder(): HTMLElement;
}

/** Arrow-key nudge as a fraction of the stack; shift takes the coarse one. */
const KEY_STEP = 0.02;
const KEY_STEP_SHIFT = 0.1;

/**
 * One divider's live state. The element and its listeners are created once and
 * cached; `horizontal` and `ctx` are refreshed on every render, so the handlers
 * never close over a stale context.
 */
interface Divider {
    el: HTMLElement;
    stackId: NodeId;
    index: number;
    horizontal: boolean;
    ctx: RenderContext;
    /** Non-null only while dragging. */
    pointerId: number | null;
    /** Pointer position the applied ratios correspond to, along the axis. */
    lastPos: number;
    onEscape: (e: KeyboardEvent) => void;
}

/** Measured pixel extents of a stack and the two slots a divider sits between. */
interface Extents {
    stackPx: number;
    prevPx: number;
    nextPx: number;
}

interface Cache {
    /** Scaffolding and frames, keyed `"<role>:<NodeId>"`. */
    elements: Map<string, HTMLElement>;
    dividers: Map<string, Divider>;
}

/** Per-container so two workspaces in one page cannot collide on a NodeId. */
const caches = new WeakMap<HTMLElement, Cache>();

/**
 * Reconciles `container` against `layout`. Scaffolding is reused by NodeId and
 * children are reordered in place, so frames handed over by `ctx.frameFor` are
 * only ever re-parented -- never cloned, emptied or rebuilt. Idempotent:
 * rendering the same layout twice touches no DOM.
 */
export function renderLayout(
    container: HTMLElement,
    layout: WorkspaceLayout,
    ctx: RenderContext,
): void {
    const cache = cacheFor(container);
    const seen = new Set<string>();

    container.classList.add("ws-root");
    container.classList.remove("ws-root--column");
    reconcile(container, [
        layout === null
            ? ctx.emptyPlaceholder()
            : renderNode(cache, seen, layout, ctx),
    ]);
    prune(cache, seen);
}

/**
 * The small-screen rendering: `items` (frames in tree order, then whatever
 * trails them) one under the other in a scrolling column. The tree's stacks
 * and ratios are ignored but untouched, so `renderLayout` can take over again
 * as soon as the screen is wide enough. Frames are re-parented, as there.
 */
export function renderColumn(container: HTMLElement, items: HTMLElement[]): void {
    container.classList.add("ws-root", "ws-root--column");
    reconcile(container, items);
}

function cacheFor(container: HTMLElement): Cache {
    let cache = caches.get(container);
    if (cache === undefined) {
        cache = { elements: new Map(), dividers: new Map() };
        caches.set(container, cache);
    }
    return cache;
}

function renderNode(
    cache: Cache,
    seen: Set<string>,
    node: LayoutNode,
    ctx: RenderContext,
): HTMLElement {
    switch (node.kind) {
        case "hstack":
        case "vstack":
            return renderStack(cache, seen, node, ctx);
        case "tabs":
        case "view": {
            // Leaves as far as the reconciler is concerned: frame.ts owns what
            // is inside. Cached only so a vanished node's frame gets detached.
            const key = `leaf:${node.id}`;
            const el = ctx.frameFor(node);
            seen.add(key);
            cache.elements.set(key, el);
            return el;
        }
    }
}

function renderStack(
    cache: Cache,
    seen: Set<string>,
    node: StackNode,
    ctx: RenderContext,
): HTMLElement {
    const horizontal = axisOf(node.kind) === "horizontal";
    const el = ensure(cache, seen, `stack:${node.id}`);
    el.className = horizontal ? "ws-stack ws-stack--h" : "ws-stack ws-stack--v";

    const children: HTMLElement[] = [];
    for (let i = 0; i < node.elements.length; i++) {
        if (i > 0) {
            children.push(divider(cache, seen, node.id, i - 1, horizontal, ctx));
        }
        children.push(
            renderSlot(cache, seen, node.elements[i], horizontal, ctx),
        );
    }
    reconcile(el, children);
    return el;
}

function renderSlot(
    cache: Cache,
    seen: Set<string>,
    element: StackElement,
    horizontal: boolean,
    ctx: RenderContext,
): HTMLElement {
    const child = element.element;
    const el = ensure(cache, seen, `slot:${child.id}`);
    el.className = "ws-slot";

    const ratio = element.ratio > 0 && Number.isFinite(element.ratio)
        ? element.ratio
        : 1;
    el.style.flex = `${ratio} 1 0px`;
    // MIN_PX along the axis is the CSS backstop for the clamping done at drag
    // time; 0 across the axis is what lets a self-sizing canvas shrink at all.
    el.style.minWidth = horizontal ? `${MIN_PX}px` : "0px";
    el.style.minHeight = horizontal ? "0px" : `${MIN_PX}px`;

    reconcile(el, [renderNode(cache, seen, child, ctx)]);
    return el;
}

function ensure(cache: Cache, seen: Set<string>, key: string): HTMLElement {
    seen.add(key);
    let el = cache.elements.get(key);
    if (el === undefined) {
        el = document.createElement("div");
        cache.elements.set(key, el);
    }
    return el;
}

function divider(
    cache: Cache,
    seen: Set<string>,
    stackId: NodeId,
    index: number,
    horizontal: boolean,
    ctx: RenderContext,
): HTMLElement {
    const key = `div:${stackId}:${index}`;
    seen.add(key);

    let d = cache.dividers.get(key);
    if (d === undefined) {
        d = createDivider(stackId, index, horizontal, ctx);
        cache.dividers.set(key, d);
    }
    d.horizontal = horizontal;
    d.ctx = ctx;

    // toggle(), not className: an in-flight drag must keep `is-active`.
    d.el.classList.toggle("ws-divider--h", horizontal);
    d.el.classList.toggle("ws-divider--v", !horizontal);
    // aria-orientation describes the separator itself, not the direction it
    // travels: the one splitting a row of panes is a vertical line.
    d.el.setAttribute("aria-orientation", horizontal ? "vertical" : "horizontal");
    return d.el;
}

function createDivider(
    stackId: NodeId,
    index: number,
    horizontal: boolean,
    ctx: RenderContext,
): Divider {
    const el = document.createElement("div");
    el.className = "ws-divider";
    el.setAttribute("role", "separator");
    el.setAttribute("aria-label", "Resize");
    el.setAttribute("tabindex", "0");

    // Hoisted, so it can name `d` while still being a stable listener identity.
    function onEscape(e: KeyboardEvent): void {
        // The ratios already applied stay; only the drag stops.
        if (e.key === "Escape") endDrag(d);
    }

    const d: Divider = {
        el,
        stackId,
        index,
        horizontal,
        ctx,
        pointerId: null,
        lastPos: 0,
        onEscape,
    };

    el.addEventListener("pointerdown", (e) => {
        if (d.pointerId !== null || !e.isPrimary || e.button !== 0) return;
        d.pointerId = e.pointerId;
        d.lastPos = position(d, e);
        el.setPointerCapture(e.pointerId);
        el.classList.add("is-active");
        window.addEventListener("keydown", d.onEscape);
        e.preventDefault();
    });
    el.addEventListener("pointermove", (e) => {
        if (d.pointerId !== e.pointerId) return;
        const m = measure(d);
        if (m === null) return;
        // Advance by what was actually applied, so a clamped drag resumes only
        // once the pointer comes back instead of jumping.
        d.lastPos += apply(d, position(d, e) - d.lastPos, m);
    });
    el.addEventListener("pointerup", () => endDrag(d));
    el.addEventListener("pointercancel", () => endDrag(d));
    el.addEventListener("keydown", (e) => onKeyDown(d, e));

    return d;
}

function endDrag(d: Divider): void {
    if (d.pointerId === null) return;
    if (d.el.hasPointerCapture(d.pointerId)) {
        d.el.releasePointerCapture(d.pointerId);
    }
    d.pointerId = null;
    d.el.classList.remove("is-active");
    window.removeEventListener("keydown", d.onEscape);
}

function onKeyDown(d: Divider, e: KeyboardEvent): void {
    // Escape is the window listener's job while a drag is in flight.
    if (d.pointerId !== null) return;

    let sign: number;
    switch (e.key) {
        case "ArrowLeft":
        case "ArrowUp":
            sign = -1;
            break;
        case "ArrowRight":
        case "ArrowDown":
            sign = 1;
            break;
        default:
            return;
    }
    e.preventDefault();

    const m = measure(d);
    if (m === null) return;
    apply(d, sign * (e.shiftKey ? KEY_STEP_SHIFT : KEY_STEP) * m.stackPx, m);
}

function position(d: Divider, e: PointerEvent): number {
    return d.horizontal ? e.clientX : e.clientY;
}

function measure(d: Divider): Extents | null {
    const stack = d.el.parentElement;
    const prev = d.el.previousElementSibling;
    const next = d.el.nextElementSibling;
    if (
        stack === null ||
        !(prev instanceof HTMLElement) ||
        !(next instanceof HTMLElement)
    ) {
        return null;
    }

    const extent = (el: HTMLElement): number => {
        const r = el.getBoundingClientRect();
        return d.horizontal ? r.width : r.height;
    };
    const stackPx = extent(stack);
    if (stackPx <= 0) return null;

    return { stackPx, prevPx: extent(prev), nextPx: extent(next) };
}

/**
 * Clamps `deltaPx` against the measured extents and applies the resize live.
 * Returns the pixels actually applied. `deltaRatio` is incremental: the
 * caller folds it into the current tree, so each move sends only the change
 * since the last one.
 */
function apply(d: Divider, deltaPx: number, m: Extents): number {
    // A neighbour already under MIN_PX (squished stack, not scrollable by
    // design) may still grow -- it just may not shrink any further.
    const lo = Math.min(MIN_PX - m.prevPx, 0);
    const hi = Math.max(m.nextPx - MIN_PX, 0);
    const applied = Math.min(Math.max(deltaPx, lo), hi);
    if (applied === 0) return 0;

    d.ctx.onDividerResize(d.stackId, d.index, applied / m.stackPx);
    return applied;
}

/** Inserts and reorders `children` in place; never clears `parent`. */
function reconcile(parent: HTMLElement, children: HTMLElement[]): void {
    for (let i = 0; i < children.length; i++) {
        const want = children[i];
        const have: ChildNode | undefined = parent.childNodes[i];
        if (have === want) continue;
        parent.insertBefore(want, have ?? null);
    }
    while (parent.childNodes.length > children.length) {
        parent.removeChild(parent.childNodes[children.length]);
    }
}

/** Drops cache entries for vanished nodes, detaching their elements. */
function prune(cache: Cache, seen: Set<string>): void {
    for (const [key, el] of cache.elements) {
        if (seen.has(key)) continue;
        cache.elements.delete(key);
        // Detach only: a frame outlives its slot, and the caller owns it.
        el.remove();
    }
    for (const [key, d] of cache.dividers) {
        if (seen.has(key)) continue;
        cache.dividers.delete(key);
        endDrag(d);
        d.el.remove();
    }
}
