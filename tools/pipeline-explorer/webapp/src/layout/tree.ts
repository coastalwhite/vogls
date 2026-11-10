import type {
    Axis,
    DropPosition,
    LayoutNode,
    NodeId,
    StackElement,
    StackNode,
    View,
    ViewNode,
    WorkspaceLayout,
} from "./types.ts";
import { RATIO_EPSILON } from "./types.ts";

/**
 * Pure operations on the layout tree. Nothing here mutates its input: every
 * operation rebuilds the path it touches and shares the rest, so a moved
 * subtree keeps its node identities and the renderer can re-parent instead of
 * rebuilding.
 */

let idCounter = 0;

export function newId(): NodeId {
    idCounter += 1;
    return `n${idCounter}`;
}

export function viewNode(view: View): ViewNode {
    return { kind: "view", id: newId(), view };
}

export function defaultLayout(): LayoutNode {
    return {
        kind: "hstack",
        id: newId(),
        elements: [
            { ratio: 1 / 3, element: viewNode({ type: "assembly", primary: true }) },
            { ratio: 2 / 3, element: viewNode({ type: "pipeline" }) },
        ],
    };
}

export function axisOf(kind: LayoutNode["kind"]): Axis | null {
    switch (kind) {
        case "hstack":
            return "horizontal";
        case "vstack":
            return "vertical";
        default:
            return null;
    }
}

function kindOfAxis(axis: Axis): StackNode["kind"] {
    return axis === "horizontal" ? "hstack" : "vstack";
}

/** The axis an edge drop splits along; `null` for "center", which never splits. */
function axisOfPosition(position: DropPosition): Axis | null {
    switch (position) {
        case "left":
        case "right":
            return "horizontal";
        case "top":
        case "bottom":
            return "vertical";
        case "center":
            return null;
    }
}

function isBefore(position: DropPosition): boolean {
    return position === "left" || position === "top";
}

function isStack(node: LayoutNode): node is StackNode {
    return node.kind === "hstack" || node.kind === "vstack";
}

function childrenOf(node: LayoutNode): LayoutNode[] {
    switch (node.kind) {
        case "hstack":
        case "vstack":
            return node.elements.map((el) => el.element);
        case "tabs":
            return node.children;
        case "view":
            return [];
    }
}

function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
}

export function findNode(root: WorkspaceLayout, id: NodeId): LayoutNode | null {
    if (root === null) return null;
    if (root.id === id) return root;
    for (const child of childrenOf(root)) {
        const found = findNode(child, id);
        if (found !== null) return found;
    }
    return null;
}

export function findParent(root: WorkspaceLayout, id: NodeId): LayoutNode | null {
    if (root === null) return null;
    for (const child of childrenOf(root)) {
        if (child.id === id) return root;
        const found = findParent(child, id);
        if (found !== null) return found;
    }
    return null;
}

/** Strict: a node is not its own ancestor. */
export function isAncestor(
    root: WorkspaceLayout,
    ancestorId: NodeId,
    descendantId: NodeId,
): boolean {
    if (ancestorId === descendantId) return false;
    const ancestor = findNode(root, ancestorId);
    if (ancestor === null) return false;
    return findNode(ancestor, descendantId) !== null;
}

export function collectViewIds(root: WorkspaceLayout): NodeId[] {
    if (root === null) return [];
    if (root.kind === "view") return [root.id];
    return childrenOf(root).flatMap(collectViewIds);
}

export function countViews(root: WorkspaceLayout): number {
    return collectViewIds(root).length;
}

/**
 * Rescales a stack's ratios to sum to 1. Degenerate input (a sum of ~0, or a
 * negative or non-finite ratio) falls back to an equal split.
 */
export function normalizeRatios(elements: StackElement[]): StackElement[] {
    if (elements.length === 0) return [];

    const safe = elements.map((el) =>
        Number.isFinite(el.ratio) && el.ratio > 0 ? el.ratio : 0,
    );
    const sum = safe.reduce((acc, ratio) => acc + ratio, 0);
    if (sum < RATIO_EPSILON) {
        const even = 1 / elements.length;
        return elements.map((el) => ({ ratio: even, element: el.element }));
    }
    return elements.map((el, i) => ({ ratio: safe[i] / sum, element: el.element }));
}

/**
 * One bottom-up pass: collapses single-child and empty containers, splices
 * same-axis stacks into their parent, clamps `active` and fixes ratios.
 * Returns `null` when the node carries no content at all.
 */
function normalizePass(node: LayoutNode): LayoutNode | null {
    switch (node.kind) {
        case "view":
            return node;
        case "tabs": {
            const children: LayoutNode[] = [];
            for (const child of node.children) {
                const next = normalizePass(child);
                if (next !== null) children.push(next);
            }
            if (children.length === 0) return null;
            // The sole child inherits this node's slot, ratio included.
            if (children.length === 1) return children[0];
            return {
                ...node,
                children,
                active: clamp(Math.trunc(node.active), 0, children.length - 1),
            };
        }
        case "hstack":
        case "vstack": {
            const axis = axisOf(node.kind);
            const elements: StackElement[] = [];
            for (const el of node.elements) {
                const next = normalizePass(el.element);
                if (next === null) continue;
                if (isStack(next) && axisOf(next.kind) === axis) {
                    // Same axis: the child's slot is split among its elements.
                    for (const inner of next.elements) {
                        elements.push({
                            ratio: inner.ratio * el.ratio,
                            element: inner.element,
                        });
                    }
                } else {
                    elements.push({ ratio: el.ratio, element: next });
                }
            }
            if (elements.length === 0) return null;
            if (elements.length === 1) return elements[0].element;
            return { ...node, elements: normalizeRatios(elements) };
        }
    }
}

function viewsEqual(a: View, b: View): boolean {
    if (a.type !== b.type) return false;
    if (a.type === "assembly" && b.type === "assembly") {
        return a.primary === b.primary;
    }
    return true;
}

function nodesEqual(a: LayoutNode, b: LayoutNode): boolean {
    if (a.kind !== b.kind || a.id !== b.id) return false;
    if (a.kind === "view") {
        return b.kind === "view" && viewsEqual(a.view, b.view);
    }
    if (a.kind === "tabs") {
        if (b.kind !== "tabs") return false;
        if (a.active !== b.active) return false;
        if (a.children.length !== b.children.length) return false;
        return a.children.every((child, i) => nodesEqual(child, b.children[i]));
    }
    if (!isStack(b)) return false;
    if (a.elements.length !== b.elements.length) return false;
    return a.elements.every((el, i) => {
        const other = b.elements[i];
        if (Math.abs(el.ratio - other.ratio) > RATIO_EPSILON) return false;
        return nodesEqual(el.element, other.element);
    });
}

function treesEqual(a: WorkspaceLayout, b: WorkspaceLayout): boolean {
    if (a === null || b === null) return a === b;
    return nodesEqual(a, b);
}

/** Applies the normalization rules until the tree stops changing. */
export function normalize(root: WorkspaceLayout): WorkspaceLayout {
    let current = root;
    // One pass is bottom-up and already idempotent in practice; the loop is the
    // cheap guarantee that a hand-written or persisted tree reaches a fixpoint.
    for (let i = 0; i < 32; i += 1) {
        if (current === null) return null;
        const next = normalizePass(current);
        if (next === null) return null;
        if (treesEqual(next, current)) return next;
        current = next;
    }
    return current;
}

function stackOf(axis: Axis, elements: StackElement[]): StackNode {
    return { kind: kindOfAxis(axis), id: newId(), elements };
}

/** The target's slot becomes a container holding both nodes. */
function wrapTarget(
    target: LayoutNode,
    incoming: LayoutNode,
    position: DropPosition,
): LayoutNode {
    const axis = axisOfPosition(position);
    if (axis === null) {
        if (target.kind === "tabs") {
            return {
                ...target,
                children: [...target.children, incoming],
                active: target.children.length,
            };
        }
        return { kind: "tabs", id: newId(), children: [target, incoming], active: 1 };
    }
    const pair: StackElement[] = isBefore(position)
        ? [{ ratio: 0.5, element: incoming }, { ratio: 0.5, element: target }]
        : [{ ratio: 0.5, element: target }, { ratio: 0.5, element: incoming }];
    return stackOf(axis, pair);
}

/** The newcomer takes `1/n`; the siblings keep their proportions in `(n-1)/n`. */
function insertIntoStack(stack: StackNode, index: number, incoming: LayoutNode): StackNode {
    const n = stack.elements.length + 1;
    const scaled = stack.elements.map((el) => ({
        ratio: el.ratio * ((n - 1) / n),
        element: el.element,
    }));
    scaled.splice(index, 0, { ratio: 1 / n, element: incoming });
    return { ...stack, elements: scaled };
}

function insertAt(
    node: LayoutNode,
    targetId: NodeId,
    incoming: LayoutNode,
    position: DropPosition,
): LayoutNode {
    const axis = axisOfPosition(position);

    // Axis-matching drop beside a direct child: widen this stack, do not nest.
    if (axis !== null && isStack(node) && axisOf(node.kind) === axis) {
        const index = node.elements.findIndex((el) => el.element.id === targetId);
        if (index >= 0) {
            return insertIntoStack(node, isBefore(position) ? index : index + 1, incoming);
        }
    }

    switch (node.kind) {
        case "view":
            return node;
        case "tabs":
            return {
                ...node,
                children: node.children.map((child) =>
                    child.id === targetId
                        ? wrapTarget(child, incoming, position)
                        : insertAt(child, targetId, incoming, position),
                ),
            };
        case "hstack":
        case "vstack":
            return {
                ...node,
                elements: node.elements.map((el) => ({
                    ratio: el.ratio,
                    element:
                        el.element.id === targetId
                            ? wrapTarget(el.element, incoming, position)
                            : insertAt(el.element, targetId, incoming, position),
                })),
            };
    }
}

function insertNode(
    root: WorkspaceLayout,
    targetId: NodeId | null,
    incoming: LayoutNode,
    position: DropPosition,
): WorkspaceLayout {
    if (root === null) return normalize(incoming);
    // A missing target on a non-empty tree means "the whole workspace", which
    // for an edge position is the same thing as `insertAtRoot`.
    const id = targetId ?? root.id;
    if (findNode(root, id) === null) return root;
    const next =
        root.id === id
            ? wrapTarget(root, incoming, position)
            : insertAt(root, id, incoming, position);
    return normalize(next);
}

export function insertView(
    root: WorkspaceLayout,
    targetId: NodeId | null,
    view: View,
    position: DropPosition,
): WorkspaceLayout {
    return insertNode(root, targetId, viewNode(view), position);
}

/** Drops the node's slot; `normalize` then rescales the survivors by `1/(1-r)`. */
function removeFrom(node: LayoutNode, id: NodeId): LayoutNode | null {
    if (node.id === id) return null;
    switch (node.kind) {
        case "view":
            return node;
        case "tabs": {
            const children: LayoutNode[] = [];
            let active = node.active;
            node.children.forEach((child, i) => {
                const next = removeFrom(child, id);
                if (next === null) {
                    // Keep pointing at the same tab, not the same index.
                    if (i < node.active) active -= 1;
                } else {
                    children.push(next);
                }
            });
            return { ...node, children, active };
        }
        case "hstack":
        case "vstack": {
            const elements: StackElement[] = [];
            for (const el of node.elements) {
                const next = removeFrom(el.element, id);
                if (next !== null) elements.push({ ratio: el.ratio, element: next });
            }
            return { ...node, elements };
        }
    }
}

export function removeNode(root: WorkspaceLayout, id: NodeId): WorkspaceLayout {
    if (root === null) return null;
    if (findNode(root, id) === null) return root;
    const next = removeFrom(root, id);
    if (next === null) return null;
    return normalize(next);
}

/**
 * What a drop would actually do, judged against the tree as it stands.
 *
 * A drop cannot be understood by performing it and comparing trees: `moveNode`
 * is remove-then-insert, and that round trip is lossy. Removing an element
 * rescales its siblings by `1/(1-r)` and re-inserting hands the newcomer `1/n`,
 * so putting a frame back where it already was turns a hand-tuned 0.4/0.3/0.3
 * split into an even one -- a visible change that would read as a real move.
 * Collapsing and re-wrapping a stack of two also mints a fresh container id.
 * So the destination is resolved up front instead.
 */
type DropOutcome =
    | { kind: "noop" }
    /** Lands in a stack the dragged node is already in: shuffle, keep ratios. */
    | { kind: "reorder"; stackId: NodeId; from: number; to: number }
    /** Everything else: a new split, a tab merge, or a move across stacks. */
    | { kind: "insert" };

function resolveDrop(
    root: LayoutNode,
    draggedId: NodeId,
    targetId: NodeId | null,
    position: DropPosition,
): DropOutcome {
    const axis = axisOfPosition(position);

    if (axis === null) {
        // There is no root-level merge zone.
        if (targetId === null) return { kind: "noop" };
        const target = findNode(root, targetId);
        if (target === null) return { kind: "noop" };
        // Already in this group: the drop could only raise the tab, which is
        // `setActiveTab`'s job, and for a group of two it would collapse the
        // group and rebuild it identically.
        if (target.kind === "tabs" && target.children.some((c) => c.id === draggedId)) {
            return { kind: "noop" };
        }
        return { kind: "insert" };
    }

    // Which existing stack does an edge drop land in, and before which element?
    let stack: StackNode | null = null;
    let index = 0;
    if (targetId === null) {
        if (root.id === draggedId) return { kind: "noop" };
        if (isStack(root) && axisOf(root.kind) === axis) {
            stack = root;
            index = isBefore(position) ? 0 : root.elements.length;
        }
    } else {
        const parent = findParent(root, targetId);
        if (parent !== null && isStack(parent) && axisOf(parent.kind) === axis) {
            stack = parent;
            const targetIndex = parent.elements.findIndex(
                (el) => el.element.id === targetId,
            );
            index = isBefore(position) ? targetIndex : targetIndex + 1;
        }
    }
    // No matching stack: the target's slot gets wrapped in a new one.
    if (stack === null) return { kind: "insert" };

    const from = stack.elements.findIndex((el) => el.element.id === draggedId);
    if (from < 0) return { kind: "insert" };
    // `index` counts slots with the dragged node still in place.
    const to = index > from ? index - 1 : index;
    if (to === from) return { kind: "noop" };
    return { kind: "reorder", stackId: stack.id, from, to };
}

/** Shuffles one element within its stack. Ratios travel with their elements. */
function reorderStack(
    root: LayoutNode,
    stackId: NodeId,
    from: number,
    to: number,
): WorkspaceLayout {
    const stack = findNode(root, stackId);
    if (stack === null || !isStack(stack)) return root;
    const elements = [...stack.elements];
    const [moved] = elements.splice(from, 1);
    elements.splice(to, 0, moved);
    return normalize(replaceById(root, stackId, { ...stack, elements }));
}

export function moveNode(
    root: WorkspaceLayout,
    id: NodeId,
    targetId: NodeId,
    position: DropPosition,
): WorkspaceLayout {
    if (root === null) return root;
    if (id === targetId) return root;
    const node = findNode(root, id);
    if (node === null) return root;
    if (findNode(root, targetId) === null) return root;
    // The one case that corrupts rather than annoys.
    if (isAncestor(root, id, targetId)) return root;

    const drop = resolveDrop(root, id, targetId, position);
    // A drop that resolves to where the node already sits must not touch the
    // tree: `canDrop` is advisory, so this is where it is actually enforced.
    if (drop.kind === "noop") return root;
    if (drop.kind === "reorder") {
        return reorderStack(root, drop.stackId, drop.from, drop.to);
    }

    const detached = removeNode(root, id);
    // Removal can collapse the target away, e.g. a tab group of two.
    if (findNode(detached, targetId) === null) return root;
    return insertNode(detached, targetId, node, position);
}

export function insertAtRoot(
    root: WorkspaceLayout,
    node: LayoutNode,
    position: Exclude<DropPosition, "center">,
): WorkspaceLayout {
    if (root === null) return normalize(node);
    const axis = axisOfPosition(position);
    if (axis === null) return root;
    const pair: StackElement[] = isBefore(position)
        ? [{ ratio: 0.5, element: node }, { ratio: 0.5, element: root }]
        : [{ ratio: 0.5, element: root }, { ratio: 0.5, element: node }];
    return normalize(stackOf(axis, pair));
}

export function moveToRoot(
    root: WorkspaceLayout,
    id: NodeId,
    position: Exclude<DropPosition, "center">,
): WorkspaceLayout {
    if (root === null) return root;
    const node = findNode(root, id);
    if (node === null) return root;

    const drop = resolveDrop(root, id, null, position);
    if (drop.kind === "noop") return root;
    if (drop.kind === "reorder") {
        return reorderStack(root, drop.stackId, drop.from, drop.to);
    }

    const detached = removeNode(root, id);
    // The dragged node was the whole workspace.
    if (detached === null) return root;
    return insertAtRoot(detached, node, position);
}

export function resizeStack(
    root: WorkspaceLayout,
    stackId: NodeId,
    dividerIndex: number,
    deltaRatio: number,
    minRatio: number,
): WorkspaceLayout {
    if (root === null) return root;
    const stack = findNode(root, stackId);
    if (stack === null || !isStack(stack)) return root;
    if (dividerIndex < 0 || dividerIndex >= stack.elements.length - 1) return root;
    if (!Number.isFinite(deltaRatio)) return root;

    const before = stack.elements[dividerIndex];
    const after = stack.elements[dividerIndex + 1];
    const total = before.ratio + after.ratio;
    // A min larger than half the pair cannot hold on both sides; split instead.
    const min = Math.min(Math.max(minRatio, 0), total / 2);
    const ratio = clamp(before.ratio + deltaRatio, min, total - min);

    const elements = stack.elements.map((el, i) => {
        if (i === dividerIndex) return { ratio, element: el.element };
        if (i === dividerIndex + 1) return { ratio: total - ratio, element: el.element };
        return el;
    });
    return replaceById(root, stackId, { ...stack, elements });
}

export function setActiveTab(
    root: WorkspaceLayout,
    tabsId: NodeId,
    active: number,
): WorkspaceLayout {
    if (root === null) return root;
    const tabs = findNode(root, tabsId);
    if (tabs === null || tabs.kind !== "tabs") return root;
    if (tabs.children.length === 0) return root;
    const clamped = clamp(Math.trunc(active), 0, tabs.children.length - 1);
    if (clamped === tabs.active) return root;
    return replaceById(root, tabsId, { ...tabs, active: clamped });
}

/** Structure-preserving swap: no normalization, no ratio changes. */
/** Swaps the view a node shows, keeping its place, ratio and id. */
export function setView(
    root: WorkspaceLayout,
    nodeId: NodeId,
    view: View,
): WorkspaceLayout {
    if (root === null) return root;
    const node = findNode(root, nodeId);
    if (node === null || node.kind !== "view") return root;
    if (viewsEqual(node.view, view)) return root;
    return replaceById(root, nodeId, { ...node, view });
}

function replaceById(
    node: LayoutNode,
    id: NodeId,
    replacement: LayoutNode,
): LayoutNode {
    if (node.id === id) return replacement;
    switch (node.kind) {
        case "view":
            return node;
        case "tabs":
            return {
                ...node,
                children: node.children.map((child) =>
                    replaceById(child, id, replacement),
                ),
            };
        case "hstack":
        case "vstack":
            return {
                ...node,
                elements: node.elements.map((el) => ({
                    ratio: el.ratio,
                    element: replaceById(el.element, id, replacement),
                })),
            };
    }
}

export function canDrop(
    root: WorkspaceLayout,
    draggedId: NodeId,
    targetId: NodeId | null,
    position: DropPosition,
): boolean {
    if (root === null) return false;
    if (findNode(root, draggedId) === null) return false;

    if (targetId === null) {
        // Root-level edge zones only, and never on a workspace of one frame.
        if (position === "center") return false;
        if (root.kind === "view") return false;
        return moveToRoot(root, draggedId, position) !== root;
    }

    if (draggedId === targetId) return false;
    if (findNode(root, targetId) === null) return false;
    if (isAncestor(root, draggedId, targetId)) return false;

    // Everything else -- no-op drops included -- is whatever the operation
    // itself decides: it returns the input tree, unchanged and identical, for
    // every drop it refuses to act on.
    return moveNode(root, draggedId, targetId, position) !== root;
}
