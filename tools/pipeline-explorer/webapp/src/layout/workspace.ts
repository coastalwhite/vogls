import type { Trace } from "../types.ts";
import type { MenuItem } from "../menu.ts";
import { DetachedWindow } from "./detached.ts";
import { beginDrag } from "./dnd.ts";
import { Frame } from "./frame.ts";
import type { FrameContext } from "./frame.ts";
import { renderColumn, renderLayout } from "./render.ts";
import type { RenderContext } from "./render.ts";
import { ViewRegistry, newView, pickableSpecs } from "./registry.ts";
import { renderPicker, setChooserHandler } from "../views/chooser.ts";
import {
    canDrop,
    collectViewIds,
    countViews,
    defaultLayout,
    findNode,
    insertAtRoot,
    insertView,
    moveNode,
    moveToRoot,
    normalize,
    removeNode,
    resizeStack,
    setActiveTab,
    setView,
} from "./tree.ts";
import { MIN_PX } from "./types.ts";
import type {
    CycleRange,
    DropPosition,
    LayoutNode,
    NodeId,
    TabsNode,
    ViewContext,
    ViewNode,
    ViewType,
    WorkspaceLayout,
} from "./types.ts";

/** The nodes that get frame chrome. Stacks are invisible scaffolding. */
type FrameNode = ViewNode | TabsNode;

function isFrameNode(node: LayoutNode): node is FrameNode {
    return node.kind === "view" || node.kind === "tabs";
}

/** The frames of a tree in reading order: depth first, stacks flattened. */
function frameNodesOf(node: LayoutNode, out: FrameNode[] = []): FrameNode[] {
    if (isFrameNode(node)) out.push(node);
    else for (const el of node.elements) frameNodesOf(el.element, out);
    return out;
}

/** Too narrow to tile: views are stacked in one scrolling column instead. */
const COLUMN_QUERY = "(max-width: 700px)";

/**
 * Owns the layout tree and wires the tree operations, the reconciler, the frame
 * chrome, the drag subsystem and the view registry together. `main.ts` talks
 * only to this.
 */
export class Workspace {
    private container: HTMLElement;
    private registry: ViewRegistry;
    private layout: WorkspaceLayout;
    private frames = new Map<NodeId, Frame>();
    /** Reverse lookup for hit-testing during a drag. */
    private frameNodes = new WeakMap<HTMLElement, NodeId>();
    private placeholder: HTMLElement | null = null;
    /** Views living in a window of their own, keyed by their node id. */
    private detached = new Map<NodeId, { window: DetachedWindow; node: ViewNode }>();
    private columnQuery = window.matchMedia(COLUMN_QUERY);
    private addButton: HTMLButtonElement | null = null;

    constructor(container: HTMLElement, ctx: ViewContext) {
        this.container = container;
        this.registry = new ViewRegistry(ctx);
        // Set before any layout exists, so the first chooser can already build.
        setChooserHandler({
            options: () =>
                pickableSpecs().map((spec) => ({ type: spec.type, label: spec.label })),
            pick: (nodeId, type) =>
                this.commit(setView(this.layout, nodeId, newView(type))),
        });
        this.layout = normalize(defaultLayout());
        this.columnQuery.addEventListener("change", () => this.render());
        this.sync();
    }

    /** Stacked in a column rather than tiled; see `COLUMN_QUERY`. */
    private get columnMode(): boolean {
        return this.columnQuery.matches;
    }

    broadcastCycle(cycles: CycleRange) {
        this.registry.broadcastCycle(cycles);
    }

    broadcastTrace(trace: Trace) {
        this.registry.broadcastTrace(trace);
    }

    /* ---------- mutation ---------- */

    private commit(next: WorkspaceLayout) {
        this.layout = normalize(next);
        this.sync();
    }

    /** Drops views and frames whose nodes are gone, then redraws. */
    private sync() {
        const live = new Set(collectViewIds(this.layout));
        for (const id of this.detached.keys()) live.add(id);
        this.registry.retain(live);
        for (const id of [...this.frames.keys()]) {
            if (findNode(this.layout, id) === null) this.frames.delete(id);
        }
        this.render();
    }

    private render() {
        if (this.columnMode) {
            renderColumn(
                this.container,
                this.layout === null
                    ? [this.emptyPlaceholder()]
                    : [
                        ...frameNodesOf(this.layout).map((n) => this.frameFor(n)),
                        this.columnAddButton(),
                    ],
            );
            return;
        }
        const ctx: RenderContext = {
            frameFor: (node) => this.frameFor(node),
            onDividerResize: (stackId, index, delta) => {
                this.commit(
                    resizeStack(this.layout, stackId, index, delta, this.minRatio()),
                );
            },
            emptyPlaceholder: () => this.emptyPlaceholder(),
        };
        renderLayout(this.container, this.layout, ctx);
    }

    /**
     * A defensive ratio floor for the tree. `render.ts` does the real clamping
     * against the stack's own measured extent; the container is at least as
     * large as any stack inside it, so this never over-clamps.
     */
    private minRatio(): number {
        const extent = Math.min(
            this.container.clientWidth,
            this.container.clientHeight,
        );
        if (extent <= 0) return 0;
        return Math.min(0.45, MIN_PX / extent);
    }

    /* ---------- frames ---------- */

    private frameFor(node: FrameNode): HTMLElement {
        const ctx = this.frameContext(node);
        let frame = this.frames.get(node.id);
        if (frame === undefined) {
            frame = new Frame(ctx);
            this.frames.set(node.id, frame);
            this.frameNodes.set(frame.element, node.id);
        } else {
            frame.update(ctx);
        }
        return frame.element;
    }

    /** The children a frame shows: itself for a view, its tabs otherwise. */
    private childrenOf(node: FrameNode): ViewNode[] {
        if (node.kind === "view") return [node];
        return node.children.filter((c): c is ViewNode => c.kind === "view");
    }

    private frameContext(node: FrameNode): FrameContext {
        const children = this.childrenOf(node);
        const active = node.kind === "tabs"
            ? Math.min(Math.max(node.active, 0), Math.max(children.length - 1, 0))
            : 0;
        // Materialise first: a title read before the instance exists is empty.
        const instances = children.map((c) => this.registry.instance(c.id, c.view));
        return {
            titles: instances.map((view) => view.title),
            activeIndex: active,
            bodies: instances.map((view) => view.root),
            menuItems: () => this.menuItems(node, children[active]),
            onSelectTab: (index) => {
                if (node.kind === "tabs") {
                    this.commit(setActiveTab(this.layout, node.id, index));
                }
            },
            onGrab: (index, e) => this.grab(children[index], e),
            onGrabFrame: (e) => this.grabFrame(node, e),
            onAddTab: () => this.addChooser(node.id, "center"),
            // The column's order is the tree's, so there is nothing to drag to.
            draggable: !this.columnMode && countViews(this.layout) > 1,
        };
    }

    private menuItems(node: FrameNode, child: ViewNode | undefined): MenuItem[] {
        const splitTarget = node.id;
        // Splits act on the frame's slot; Close acts on the visible tab only.
        const closeTarget = child?.id ?? node.id;
        // Each of these drops in a chooser, which asks what goes there, rather
        // than making the user pick a type from a submenu first.
        return [
            {
                kind: "item",
                label: "Split Right",
                onSelect: () => this.addChooser(splitTarget, "right"),
            },
            {
                kind: "item",
                label: "Split Down",
                onSelect: () => this.addChooser(splitTarget, "bottom"),
            },
            {
                kind: "item",
                label: "New Tab Here",
                onSelect: () => this.addChooser(splitTarget, "center"),
            },
            { kind: "separator" },
            {
                kind: "item",
                label: "Detach to Window",
                disabled: child === undefined,
                onSelect: () => {
                    if (child !== undefined) this.detachView(child);
                },
            },
            {
                kind: "item",
                label: "Close",
                onSelect: () => this.commit(removeNode(this.layout, closeTarget)),
            },
        ];
    }

    private addView(targetId: NodeId | null, position: DropPosition, type: ViewType) {
        this.commit(insertView(this.layout, targetId, newView(type), position));
    }

    /** Moves a view into a window of its own, and back when that closes. */
    private detachView(node: ViewNode) {
        if (this.detached.has(node.id)) return;
        const instance = this.registry.instance(node.id, node.view);
        let window: DetachedWindow;
        try {
            window = new DetachedWindow({
                title: instance.title,
                root: instance.root,
                onClose: () => this.reattach(node.id),
            });
        } catch (e) {
            console.error("could not detach the view", e);
            return;
        }
        this.detached.set(node.id, { window, node });
        // Out of the tree, but `sync` keeps the instance alive.
        this.commit(removeNode(this.layout, node.id));
    }

    /** Puts a detached view back, keeping its node id so the instance is reused. */
    private reattach(id: NodeId) {
        const entry = this.detached.get(id);
        if (entry === undefined) return;
        this.detached.delete(id);
        this.commit(
            this.layout === null
                ? entry.node
                : insertAtRoot(this.layout, entry.node, "right"),
        );
    }

    /** Closes every detached window, so none outlive the workspace. */
    closeDetached() {
        for (const { window } of [...this.detached.values()]) window.close();
    }

    /** A split or a new tab lands empty; the chooser asks what goes in it. */
    private addChooser(targetId: NodeId | null, position: DropPosition) {
        this.addView(targetId, position, "chooser");
    }

    /** Ends the column: appends a chooser below everything else. */
    private columnAddButton(): HTMLButtonElement {
        if (this.addButton === null) {
            const button = document.createElement("button");
            button.type = "button";
            button.className = "ws-column-add";
            button.textContent = "+";
            button.title = "Add view";
            button.setAttribute("aria-label", "Add view");
            button.addEventListener("click", () => this.addChooser(null, "bottom"));
            this.addButton = button;
        }
        return this.addButton;
    }

    /* ---------- empty workspace ---------- */

    private emptyPlaceholder(): HTMLElement {
        if (this.placeholder === null) {
            this.placeholder = renderPicker((type) =>
                this.addView(null, "center", type)
            );
        }
        return this.placeholder;
    }

    /* ---------- drag ---------- */

    private grab(child: ViewNode | undefined, e: PointerEvent) {
        if (child === undefined) return;
        this.beginNodeDrag(child.id, this.registry.titleOf(child.id), e);
    }

    /** Drags the frame itself: a lone view, or a whole tab group as a unit. */
    private grabFrame(node: FrameNode, e: PointerEvent) {
        if (node.kind === "view") {
            this.grab(node, e);
            return;
        }
        const children = this.childrenOf(node);
        const active = children[Math.min(Math.max(node.active, 0), children.length - 1)];
        if (active === undefined) return;
        const title = this.registry.titleOf(active.id);
        this.beginNodeDrag(
            node.id,
            children.length > 1 ? `${title} +${children.length - 1}` : title,
            e,
        );
    }

    private beginNodeDrag(draggedId: NodeId, title: string, e: PointerEvent) {
        beginDrag(e, {
            container: this.container,
            nodeIdAt: (x, y) => this.nodeIdAt(x, y),
            rectOf: (id) => this.frames.get(id)?.element.getBoundingClientRect() ?? null,
            canDrop: (targetId, position) =>
                canDrop(this.layout, draggedId, targetId, position),
            onDrop: (targetId, position) => {
                this.commit(
                    targetId === null
                        ? moveToRoot(this.layout, draggedId, position as Exclude<DropPosition, "center">)
                        : moveNode(this.layout, draggedId, targetId, position),
                );
            },
            title,
            setDragging: (dragging) => this.setDragging(draggedId, dragging),
        });
    }

    /**
     * The frame id under a viewport point. The drag overlays are
     * `pointer-events: none`, so they never shadow a frame here.
     */
    private nodeIdAt(x: number, y: number): NodeId | null {
        const hit = document.elementFromPoint(x, y);
        const frame = hit instanceof Element ? hit.closest(".ws-frame") : null;
        if (!(frame instanceof HTMLElement)) return null;
        return this.frameNodes.get(frame) ?? null;
    }

    /** The dragged node is either a frame itself, or a tab inside one. */
    private setDragging(nodeId: NodeId, dragging: boolean) {
        const own = this.frames.get(nodeId);
        if (own !== undefined) {
            own.setDragging(dragging);
            return;
        }
        for (const [id, frame] of this.frames) {
            const node = findNode(this.layout, id);
            if (node === null || !isFrameNode(node)) continue;
            if (this.childrenOf(node).some((c) => c.id === nodeId)) {
                frame.setDragging(dragging);
                return;
            }
        }
    }
}
