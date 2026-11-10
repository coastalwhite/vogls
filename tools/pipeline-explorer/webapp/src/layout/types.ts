import type { Trace } from "../types.ts";

export type NodeId = string;

/** A single pane's content. Payloads hold configuration, never ephemeral state. */
export type View =
    | { type: "pipeline" }
    | { type: "assembly"; primary: boolean }
    | { type: "chooser" };

export type ViewType = View["type"];

export type StackElement = { ratio: number; element: LayoutNode };

export type LayoutNode =
    | { kind: "vstack"; id: NodeId; elements: StackElement[] }
    | { kind: "hstack"; id: NodeId; elements: StackElement[] }
    | { kind: "tabs"; id: NodeId; children: LayoutNode[]; active: number }
    | { kind: "view"; id: NodeId; view: View };

export type StackNode = Extract<LayoutNode, { kind: "vstack" | "hstack" }>;
export type TabsNode = Extract<LayoutNode, { kind: "tabs" }>;
export type ViewNode = Extract<LayoutNode, { kind: "view" }>;

/** `null` is an empty workspace, which renders the view picker. */
export type WorkspaceLayout = LayoutNode | null;

export type Axis = "horizontal" | "vertical";

/** Where a node lands relative to a target. "center" merges into a tab group. */
export type DropPosition = "left" | "right" | "top" | "bottom" | "center";

/** Smallest pane extent along a stack's axis. */
export const MIN_PX = 160;

/** Ratios are floats; never compare them exactly. */
export const RATIO_EPSILON = 1e-6;

/**
 * A live view. It owns `root` for its whole life: the renderer re-parents that
 * element as the tree changes rather than rebuilding it, so scroll position,
 * canvas pan and editor undo survive every move.
 */
export interface ViewInstance {
    readonly root: HTMLElement;
    /** Shown in the titlebar or tab. May change over the view's life. */
    readonly title: string;
    onCycleChange?(cycle: number): void;
    onTraceChange?(trace: Trace): void;
    destroy(): void;
}

/** What the app gives a view at construction. */
export interface ViewContext {
    readonly trace: Trace | null;
    readonly currentCycle: number;
    goToCycle(cycle: number): void;
    /** Re-run the simulation; `immediate` skips the debounce. */
    requestRun(immediate: boolean): void;
}

export interface ViewSpec {
    type: ViewType;
    /** Label in the view pickers. */
    label: string;
    /** False keeps it out of the pickers; it is only created by the app. */
    pickable?: boolean;
    create(view: View, ctx: ViewContext, nodeId: NodeId): ViewInstance;
}
