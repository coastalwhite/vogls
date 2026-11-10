import type { DropPosition, NodeId } from "./types.ts";

/** Pointer travel that turns a press into a drag. Below it, clicks still work. */
const THRESHOLD_PX = 4;
/** Fraction of a frame taken by each of its four edge bands. */
const EDGE_FRACTION = 0.25;
/** Inset of the workspace's root-level edge zones. */
const ROOT_ZONE_PX = 24;
/** Keeps the chip out from under the cursor. */
const CHIP_OFFSET_X = 14;
const CHIP_OFFSET_Y = 18;

export interface DragContext {
    /** The workspace container. Drop geometry is computed within its rect. */
    container: HTMLElement;
    /** Node id of the frame under a viewport point, or null. */
    nodeIdAt(x: number, y: number): NodeId | null;
    /** Viewport rect of a node's frame element. */
    rectOf(nodeId: NodeId): DOMRect | null;
    /** `targetId === null` means a root-level edge drop. */
    canDrop(targetId: NodeId | null, position: DropPosition): boolean;
    onDrop(targetId: NodeId | null, position: DropPosition): void;
    /** Shown in the chip that follows the cursor. */
    title: string;
    /** Called with true on drag start and false on end, for `.is-dragging`. */
    setDragging(dragging: boolean): void;
}

/** A viewport-space box. A `DOMRect` satisfies this, so rects pass straight in. */
interface Region {
    readonly left: number;
    readonly top: number;
    readonly width: number;
    readonly height: number;
}

interface Resolution {
    targetId: NodeId | null;
    position: DropPosition;
    /** The region the dragged node would occupy, in viewport coordinates. */
    region: Region;
    valid: boolean;
}

/** The two overlay elements, alive only between threshold and teardown. */
interface Feedback {
    zone: HTMLElement;
    chip: HTMLElement;
    /** The zone has been given real geometry at least once. */
    positioned: boolean;
}

interface Drag {
    ctx: DragContext;
    pointerId: number;
    captureEl: HTMLElement;
    startX: number;
    startY: number;
    /** Non-null once the threshold is crossed, i.e. the drag is really live. */
    feedback: Feedback | null;
    /** What the last pointer position resolved to; what `pointerup` commits. */
    resolution: Resolution | null;
    done: boolean;
    detach(): void;
}

/** At most one drag at a time; a new one abandons whatever was running. */
let active: Drag | null = null;

/** Call from a titlebar/tab `pointerdown`. Nothing happens until the pointer
 *  moves past the threshold, so plain clicks are unaffected. */
export function beginDrag(e: PointerEvent, ctx: DragContext): void {
    if (!e.isPrimary || e.button !== 0) return;
    if (active !== null) teardown(active);

    const captureEl = e.currentTarget instanceof HTMLElement
        ? e.currentTarget
        : e.target instanceof HTMLElement
        ? e.target
        : null;
    if (captureEl === null) return;

    const drag: Drag = {
        ctx,
        pointerId: e.pointerId,
        captureEl,
        startX: e.clientX,
        startY: e.clientY,
        feedback: null,
        resolution: null,
        done: false,
        detach: () => {},
    };

    const onMove = (ev: PointerEvent) => {
        if (ev.pointerId !== drag.pointerId) return;
        const feedback = drag.feedback ?? maybeStart(drag, ev);
        if (feedback === null) return;
        update(drag, feedback, ev.clientX, ev.clientY);
    };

    const onUp = (ev: PointerEvent) => {
        if (ev.pointerId !== drag.pointerId) return;
        const feedback = drag.feedback;
        if (feedback === null) {
            // Released under the threshold: this was a click, not a drag.
            teardown(drag);
            return;
        }
        // Resolve the release point so what commits is what was last drawn.
        update(drag, feedback, ev.clientX, ev.clientY);
        const resolution = drag.resolution;
        try {
            if (resolution !== null && resolution.valid) {
                drag.ctx.onDrop(resolution.targetId, resolution.position);
            }
        } finally {
            teardown(drag);
        }
    };

    const onPointerCancel = (ev: PointerEvent) => {
        if (ev.pointerId === drag.pointerId) teardown(drag);
    };

    const onLostCapture = (ev: PointerEvent) => {
        if (ev.pointerId === drag.pointerId) teardown(drag);
    };

    const onKeyDown = (ev: KeyboardEvent) => {
        if (ev.key !== "Escape") return;
        // Swallow it, so a cancelled drag does not also close a popover.
        ev.preventDefault();
        ev.stopPropagation();
        teardown(drag);
    };

    const onBlur = () => teardown(drag);

    drag.detach = () => {
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);
        document.removeEventListener("pointercancel", onPointerCancel);
        document.removeEventListener("keydown", onKeyDown, true);
        window.removeEventListener("blur", onBlur);
        captureEl.removeEventListener("lostpointercapture", onLostCapture);
    };

    // Capture happens at the threshold, but listening starts now, on the
    // document, so a fast flick off the handle still arms the drag.
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    document.addEventListener("pointercancel", onPointerCancel);
    document.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("blur", onBlur);
    captureEl.addEventListener("lostpointercapture", onLostCapture);

    active = drag;
}

/** Starts the drag if the pointer has travelled far enough, else null. */
function maybeStart(drag: Drag, e: PointerEvent): Feedback | null {
    const travel = Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY);
    if (travel < THRESHOLD_PX) return null;

    if (drag.captureEl.isConnected) {
        try {
            drag.captureEl.setPointerCapture(drag.pointerId);
        } catch {
            // The pointer is already gone; the drag ends on the next event.
        }
    }

    const zone = overlay("ws-dropzone");
    zone.style.display = "none";
    const chip = overlay("ws-dragchip");
    chip.textContent = drag.ctx.title;

    drag.feedback = { zone, chip, positioned: false };
    drag.ctx.setDragging(true);
    return drag.feedback;
}

function update(drag: Drag, feedback: Feedback, x: number, y: number) {
    const resolution = resolve(drag.ctx, x, y);
    drag.resolution = resolution;

    feedback.chip.style.left = `${x + CHIP_OFFSET_X}px`;
    feedback.chip.style.top = `${y + CHIP_OFFSET_Y}px`;

    const zone = feedback.zone;
    if (resolution === null) {
        zone.style.display = "none";
        return;
    }

    zone.classList.toggle("is-invalid", !resolution.valid);
    if (!resolution.valid) {
        // Leave the zone where it was: a refused drop has no region to show.
        if (!feedback.positioned) zone.style.display = "none";
        return;
    }

    const region = resolution.region;
    zone.style.left = `${region.left}px`;
    zone.style.top = `${region.top}px`;
    zone.style.width = `${region.width}px`;
    zone.style.height = `${region.height}px`;
    zone.style.display = "block";
    feedback.positioned = true;
}

/**
 * Hit-tests a viewport point: outside the container is nothing, the container's
 * outer 24px is a root zone, and anything further in belongs to the frame under
 * the cursor.
 */
function resolve(ctx: DragContext, x: number, y: number): Resolution | null {
    const container = ctx.container.getBoundingClientRect();
    if (
        x < container.left || x > container.right ||
        y < container.top || y > container.bottom
    ) {
        return null;
    }

    const rootEdge = rootZoneEdge(container, x, y);
    if (rootEdge !== null) return resolution(ctx, null, rootEdge, container);

    const nodeId = ctx.nodeIdAt(x, y);
    if (nodeId === null) return null;
    const rect = ctx.rectOf(nodeId);
    if (rect === null) return null;
    return resolution(ctx, nodeId, bandOf(rect, x, y), rect);
}

function resolution(
    ctx: DragContext,
    targetId: NodeId | null,
    position: DropPosition,
    frame: Region,
): Resolution {
    return {
        targetId,
        position,
        region: regionFor(frame, position),
        valid: ctx.canDrop(targetId, position),
    };
}

/** The root edge whose band the point is deepest into, or null if in none. */
function rootZoneEdge(
    container: Region & { right: number; bottom: number },
    x: number,
    y: number,
): DropPosition | null {
    const distances: [DropPosition, number][] = [
        ["left", x - container.left],
        ["right", container.right - x],
        ["top", y - container.top],
        ["bottom", container.bottom - y],
    ];

    let edge: DropPosition | null = null;
    let nearest = ROOT_ZONE_PX;
    for (const [position, distance] of distances) {
        if (distance < nearest) {
            edge = position;
            nearest = distance;
        }
    }
    return edge;
}

/** The band of `rect` a point falls in; corners go to the deeper band. */
function bandOf(rect: Region, x: number, y: number): DropPosition {
    const u = rect.width > 0 ? (x - rect.left) / rect.width : 0.5;
    const v = rect.height > 0 ? (y - rect.top) / rect.height : 0.5;

    // Penetration normalised to the band's own width, so the x and y bands are
    // comparable however oblong the frame is. Zero or less means "not in it".
    const bands: [DropPosition, number][] = [
        ["left", (EDGE_FRACTION - u) / EDGE_FRACTION],
        ["right", (u - (1 - EDGE_FRACTION)) / EDGE_FRACTION],
        ["top", (EDGE_FRACTION - v) / EDGE_FRACTION],
        ["bottom", (v - (1 - EDGE_FRACTION)) / EDGE_FRACTION],
    ];

    let band: DropPosition = "center";
    let deepest = 0;
    for (const [position, depth] of bands) {
        if (depth > deepest) {
            band = position;
            deepest = depth;
        }
    }
    return band;
}

/** The slice of `frame` the dragged node would end up occupying. */
function regionFor(frame: Region, position: DropPosition): Region {
    const halfWidth = frame.width / 2;
    const halfHeight = frame.height / 2;
    switch (position) {
        case "left":
            return {
                left: frame.left,
                top: frame.top,
                width: halfWidth,
                height: frame.height,
            };
        case "right":
            return {
                left: frame.left + halfWidth,
                top: frame.top,
                width: halfWidth,
                height: frame.height,
            };
        case "top":
            return {
                left: frame.left,
                top: frame.top,
                width: frame.width,
                height: halfHeight,
            };
        case "bottom":
            return {
                left: frame.left,
                top: frame.top + halfHeight,
                width: frame.width,
                height: halfHeight,
            };
        case "center":
            return {
                left: frame.left,
                top: frame.top,
                width: frame.width,
                height: frame.height,
            };
    }
}

function overlay(className: string): HTMLElement {
    const el = document.createElement("div");
    el.className = className;
    el.style.position = "fixed";
    // Must never answer a hit-test: `nodeIdAt` is likely `elementFromPoint`.
    el.style.pointerEvents = "none";
    document.body.appendChild(el);
    return el;
}

/** Idempotent, total, and never throws before the overlays are gone. */
function teardown(drag: Drag) {
    if (drag.done) return;
    drag.done = true;
    if (active === drag) active = null;

    drag.detach();
    if (drag.captureEl.hasPointerCapture(drag.pointerId)) {
        drag.captureEl.releasePointerCapture(drag.pointerId);
    }

    const feedback = drag.feedback;
    if (feedback === null) return;
    drag.feedback = null;
    feedback.zone.remove();
    feedback.chip.remove();
    drag.ctx.setDragging(false);
}
