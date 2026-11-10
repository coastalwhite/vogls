import { describe, expect, it } from "vitest";

import type { LayoutNode, NodeId, StackElement, WorkspaceLayout } from "./types.ts";
import { RATIO_EPSILON } from "./types.ts";
import {
    axisOf,
    canDrop,
    collectViewIds,
    countViews,
    defaultLayout,
    findNode,
    findParent,
    insertAtRoot,
    insertView,
    isAncestor,
    moveNode,
    moveToRoot,
    newId,
    normalize,
    normalizeRatios,
    removeNode,
    resizeStack,
    setActiveTab,
    setView,
    viewNode,
} from "./tree.ts";

/* Fixtures. Only three distinct view payloads exist, so leaves that have to be
   told apart structurally are built from `pipe`, `asmA` and `asmB`. */

function pipe(id: string): LayoutNode {
    return { kind: "view", id, view: { type: "pipeline" } };
}

function asmA(id: string): LayoutNode {
    return { kind: "view", id, view: { type: "assembly", primary: true } };
}

function asmB(id: string): LayoutNode {
    return { kind: "view", id, view: { type: "assembly", primary: false } };
}

function at(ratio: number, element: LayoutNode): StackElement {
    return { ratio, element };
}

function hstack(id: string, ...elements: StackElement[]): LayoutNode {
    return { kind: "hstack", id, elements };
}

function vstack(id: string, ...elements: StackElement[]): LayoutNode {
    return { kind: "vstack", id, elements };
}

function tabs(id: string, active: number, ...children: LayoutNode[]): LayoutNode {
    return { kind: "tabs", id, children, active };
}

function elementsOf(node: LayoutNode | null): StackElement[] {
    if (node === null || (node.kind !== "hstack" && node.kind !== "vstack")) {
        throw new Error(`expected a stack, got ${node === null ? "null" : node.kind}`);
    }
    return node.elements;
}

function ratios(node: LayoutNode | null): number[] {
    return elementsOf(node).map((el) => el.ratio);
}

function ids(node: LayoutNode | null): NodeId[] {
    return elementsOf(node).map((el) => el.element.id);
}

/** The whole-tree invariant. Called after every mutation in these tests. */
function checkTree(root: WorkspaceLayout): void {
    if (root === null) return;
    const seen = new Set<NodeId>();

    const walk = (node: LayoutNode): void => {
        expect(seen.has(node.id)).toBe(false);
        seen.add(node.id);

        if (node.kind === "view") return;

        if (node.kind === "tabs") {
            expect(node.children.length).toBeGreaterThanOrEqual(2);
            expect(Number.isInteger(node.active)).toBe(true);
            expect(node.active).toBeGreaterThanOrEqual(0);
            expect(node.active).toBeLessThan(node.children.length);
            node.children.forEach(walk);
            return;
        }

        expect(node.elements.length).toBeGreaterThanOrEqual(2);
        let sum = 0;
        for (const el of node.elements) {
            expect(Number.isFinite(el.ratio)).toBe(true);
            expect(el.ratio).toBeGreaterThan(0);
            // No same-axis nesting.
            expect(axisOf(el.element.kind)).not.toBe(axisOf(node.kind));
            sum += el.ratio;
            walk(el.element);
        }
        expect(Math.abs(sum - 1)).toBeLessThan(RATIO_EPSILON);
    };

    walk(root);
}

/** Asserts `op` left its input untouched, and returns the result. */
function pure<T>(input: WorkspaceLayout, op: () => T): T {
    const before = JSON.stringify(input);
    const result = op();
    expect(JSON.stringify(input)).toBe(before);
    return result;
}

describe("ids and constructors", () => {
    it("hands out unique ids", () => {
        const minted = new Set([newId(), newId(), newId()]);
        expect(minted.size).toBe(3);
    });

    it("builds a view node", () => {
        const node = viewNode({ type: "assembly", primary: true });
        expect(node.kind).toBe("view");
        expect(node.view).toEqual({ type: "assembly", primary: true });
    });

    it("reproduces the 1/3 - 2/3 split", () => {
        const root = defaultLayout();
        checkTree(root);
        expect(root.kind).toBe("hstack");
        const elements = elementsOf(root);
        expect(elements[0].ratio).toBeCloseTo(1 / 3, 10);
        expect(elements[1].ratio).toBeCloseTo(2 / 3, 10);
        expect(elements[0].element).toMatchObject({
            kind: "view",
            view: { type: "assembly", primary: true },
        });
        expect(elements[1].element).toMatchObject({
            kind: "view",
            view: { type: "pipeline" },
        });
        expect(countViews(root)).toBe(2);
    });

    it("maps kinds to axes", () => {
        expect(axisOf("hstack")).toBe("horizontal");
        expect(axisOf("vstack")).toBe("vertical");
        expect(axisOf("tabs")).toBeNull();
        expect(axisOf("view")).toBeNull();
    });
});

describe("traversal", () => {
    const tree = hstack(
        "root",
        at(0.5, pipe("a")),
        at(0.5, vstack("inner", at(0.5, asmA("b")), at(0.5, tabs("tabs", 0, asmB("c"), pipe("d"))))),
    );

    it("finds nodes and parents", () => {
        expect(findNode(tree, "root")).toBe(tree);
        expect(findNode(tree, "d")?.id).toBe("d");
        expect(findNode(tree, "nope")).toBeNull();
        expect(findNode(null, "a")).toBeNull();

        expect(findParent(tree, "root")).toBeNull();
        expect(findParent(tree, "a")?.id).toBe("root");
        expect(findParent(tree, "c")?.id).toBe("tabs");
        expect(findParent(tree, "nope")).toBeNull();
    });

    it("collects views in document order", () => {
        expect(collectViewIds(tree)).toEqual(["a", "b", "c", "d"]);
        expect(countViews(tree)).toBe(4);
        expect(collectViewIds(null)).toEqual([]);
        expect(countViews(null)).toBe(0);
    });

    it("answers isAncestor strictly", () => {
        expect(isAncestor(tree, "root", "d")).toBe(true);
        expect(isAncestor(tree, "inner", "c")).toBe(true);
        expect(isAncestor(tree, "tabs", "d")).toBe(true);
        expect(isAncestor(tree, "root", "root")).toBe(false);
        expect(isAncestor(tree, "d", "tabs")).toBe(false);
        expect(isAncestor(tree, "a", "b")).toBe(false);
        expect(isAncestor(tree, "nope", "a")).toBe(false);
        expect(isAncestor(null, "a", "b")).toBe(false);
    });
});

describe("normalizeRatios", () => {
    it("rescales to sum 1 without mutating", () => {
        const input = [at(2, pipe("a")), at(2, asmA("b"))];
        const before = JSON.stringify(input);
        expect(normalizeRatios(input).map((el) => el.ratio)).toEqual([0.5, 0.5]);
        expect(JSON.stringify(input)).toBe(before);
    });

    it("keeps proportions", () => {
        const out = normalizeRatios([at(3, pipe("a")), at(1, asmA("b"))]);
        expect(out.map((el) => el.ratio)).toEqual([0.75, 0.25]);
    });

    it("splits evenly when the sum is ~0", () => {
        const out = normalizeRatios([at(0, pipe("a")), at(0, asmA("b")), at(0, asmB("c"))]);
        for (const el of out) expect(el.ratio).toBeCloseTo(1 / 3, 10);
    });

    it("treats negative and non-finite ratios as zero", () => {
        const out = normalizeRatios([at(-1, pipe("a")), at(1, asmA("b"))]);
        expect(out.map((el) => el.ratio)).toEqual([0, 1]);
        const broken = normalizeRatios([at(NaN, pipe("a")), at(Infinity, asmA("b"))]);
        expect(broken.map((el) => el.ratio)).toEqual([0.5, 0.5]);
    });

    it("passes an empty list through", () => {
        expect(normalizeRatios([])).toEqual([]);
    });
});

describe("normalize", () => {
    it("rule 1: a single-child stack collapses and the child inherits the slot", () => {
        const tree = vstack(
            "root",
            at(0.25, hstack("solo", at(1, pipe("a")))),
            at(0.75, asmA("b")),
        );
        const out = pure(tree, () => normalize(tree));
        checkTree(out);
        expect(ids(out)).toEqual(["a", "b"]);
        expect(ratios(out)).toEqual([0.25, 0.75]);
    });

    it("rule 1: a single-child tabs collapses too", () => {
        const tree = hstack("root", at(0.4, tabs("t", 0, pipe("a"))), at(0.6, asmA("b")));
        const out = normalize(tree);
        checkTree(out);
        expect(ids(out)).toEqual(["a", "b"]);
        expect(ratios(out)).toEqual([0.4, 0.6]);
    });

    it("rule 2: a same-axis child splices in, scaled by its ratio", () => {
        const tree = hstack(
            "root",
            at(0.5, pipe("a")),
            at(0.5, hstack("nested", at(0.25, asmA("b")), at(0.75, asmB("c")))),
        );
        const out = normalize(tree);
        checkTree(out);
        expect(ids(out)).toEqual(["a", "b", "c"]);
        expect(ratios(out)).toEqual([0.5, 0.125, 0.375]);
    });

    it("rule 2: a cross-axis child is left nested", () => {
        const tree = hstack(
            "root",
            at(0.5, pipe("a")),
            at(0.5, vstack("nested", at(0.5, asmA("b")), at(0.5, asmB("c")))),
        );
        const out = normalize(tree);
        checkTree(out);
        expect(ids(out)).toEqual(["a", "nested"]);
    });

    it("rule 2: flattens a deeply nested same-axis chain", () => {
        const tree = hstack(
            "root",
            at(
                0.5,
                hstack(
                    "l1",
                    at(0.5, hstack("l2", at(0.5, pipe("a")), at(0.5, asmA("b")))),
                    at(0.5, asmB("c")),
                ),
            ),
            at(0.5, pipe("d")),
        );
        const out = normalize(tree);
        checkTree(out);
        expect(ids(out)).toEqual(["a", "b", "c", "d"]);
        expect(ratios(out)).toEqual([0.125, 0.125, 0.25, 0.5]);
    });

    it("rule 3: an empty container is dropped from its parent", () => {
        const tree = vstack(
            "root",
            at(0.4, hstack("empty")),
            at(0.3, pipe("a")),
            at(0.3, asmA("b")),
        );
        const out = normalize(tree);
        checkTree(out);
        expect(ids(out)).toEqual(["a", "b"]);
        expect(ratios(out)).toEqual([0.5, 0.5]);
        expect(findNode(out, "empty")).toBeNull();
    });

    it("rule 3: an empty tabs is dropped too", () => {
        const tree = hstack(
            "root",
            at(0.5, tabs("empty", 0)),
            at(0.25, pipe("a")),
            at(0.25, asmA("b")),
        );
        const out = normalize(tree);
        checkTree(out);
        expect(ids(out)).toEqual(["a", "b"]);
    });

    it("rule 4: a root with no content becomes null", () => {
        expect(normalize(hstack("root"))).toBeNull();
        expect(normalize(tabs("root", 0))).toBeNull();
        expect(normalize(hstack("root", at(1, vstack("mid", at(1, tabs("t", 0))))))).toBeNull();
        expect(normalize(null)).toBeNull();
    });

    it("rule 5: active clamps into range", () => {
        const high = normalize(tabs("t", 7, pipe("a"), asmA("b")));
        checkTree(high);
        expect(high).toMatchObject({ kind: "tabs", active: 1 });

        const low = normalize(tabs("t", -3, pipe("a"), asmA("b")));
        checkTree(low);
        expect(low).toMatchObject({ kind: "tabs", active: 0 });

        const shrunk = normalize(
            hstack("root", at(1, tabs("t", 2, pipe("a"), asmA("b"), vstack("gone")))),
        );
        checkTree(shrunk);
        expect(shrunk).toMatchObject({ kind: "tabs", active: 1 });
    });

    it("rule 6: ratios are renormalised everywhere", () => {
        const tree = hstack(
            "root",
            at(3, pipe("a")),
            at(1, vstack("inner", at(8, asmA("b")), at(2, asmB("c")))),
        );
        const out = normalize(tree);
        checkTree(out);
        expect(ratios(out)).toEqual([0.75, 0.25]);
        expect(ratios(elementsOf(out)[1].element)).toEqual([0.8, 0.2]);
    });

    it("leaves a healthy tree and a bare view alone", () => {
        const healthy = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        expect(normalize(healthy)).toEqual(healthy);
        const leaf = pipe("a");
        expect(normalize(leaf)).toEqual(leaf);
    });
});

describe("insertView", () => {
    it("makes the view the root of an empty workspace", () => {
        const out = insertView(null, null, { type: "pipeline" }, "center");
        checkTree(out);
        expect(out).toMatchObject({ kind: "view", view: { type: "pipeline" } });
    });

    it("center on a view builds a tab group with the newcomer active", () => {
        const tree = hstack("root", at(0.3, pipe("a")), at(0.7, asmA("b")));
        const out = pure(tree, () => insertView(tree, "a", { type: "assembly", primary: false }, "center"));
        checkTree(out);
        expect(ratios(out)).toEqual([0.3, 0.7]);
        const group = elementsOf(out)[0].element;
        expect(group.kind).toBe("tabs");
        if (group.kind !== "tabs") throw new Error("expected tabs");
        expect(group.children.map((c) => c.id)).toEqual(["a", group.children[1].id]);
        expect(group.active).toBe(1);
        expect(countViews(out)).toBe(3);
    });

    it("center on a tabs node appends and activates", () => {
        const tree = tabs("t", 0, pipe("a"), asmA("b"));
        const out = insertView(tree, "t", { type: "assembly", primary: false }, "center");
        checkTree(out);
        if (out === null || out.kind !== "tabs") throw new Error("expected tabs");
        expect(out.children.length).toBe(3);
        expect(out.active).toBe(2);
        expect(out.children[2]).toMatchObject({ view: { type: "assembly", primary: false } });
    });

    it("an axis-matching edge drop widens the parent stack", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const out = insertView(tree, "a", { type: "assembly", primary: false }, "left");
        checkTree(out);
        expect(elementsOf(out).length).toBe(3);
        expect(ids(out)[1]).toBe("a");
        expect(ids(out)[2]).toBe("b");
        for (const ratio of ratios(out)) expect(ratio).toBeCloseTo(1 / 3, 10);
    });

    it("an axis-matching edge drop on the right lands after the target", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const out = insertView(tree, "a", { type: "assembly", primary: false }, "right");
        checkTree(out);
        expect(ids(out)[0]).toBe("a");
        expect(ids(out)[2]).toBe("b");
    });

    it("a cross-axis edge drop nests inside the target's slot", () => {
        const tree = hstack("root", at(0.3, pipe("a")), at(0.7, asmA("b")));
        const out = insertView(tree, "a", { type: "assembly", primary: false }, "top");
        checkTree(out);
        expect(ratios(out)).toEqual([0.3, 0.7]);
        const split = elementsOf(out)[0].element;
        expect(split.kind).toBe("vstack");
        expect(ratios(split)).toEqual([0.5, 0.5]);
        expect(ids(split)[1]).toBe("a");
    });

    it("an edge drop on the root view wraps the root", () => {
        const tree = pipe("a");
        const out = insertView(tree, "a", { type: "pipeline" }, "bottom");
        checkTree(out);
        expect(out?.kind).toBe("vstack");
        expect(ids(out)[0]).toBe("a");
        expect(ratios(out)).toEqual([0.5, 0.5]);
    });

    it("an edge drop inside a tab splits that tab's content", () => {
        const tree = tabs("t", 1, pipe("a"), asmA("b"));
        const out = insertView(tree, "b", { type: "assembly", primary: false }, "right");
        checkTree(out);
        if (out === null || out.kind !== "tabs") throw new Error("expected tabs");
        expect(out.children[1].kind).toBe("hstack");
        expect(ids(out.children[1])[0]).toBe("b");
        expect(out.active).toBe(1);
    });

    it("an unknown target leaves the tree alone", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        expect(insertView(tree, "nope", { type: "pipeline" }, "left")).toBe(tree);
    });

    it("a null target on a populated tree inserts at the root", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const out = insertView(tree, null, { type: "assembly", primary: false }, "top");
        checkTree(out);
        expect(out?.kind).toBe("vstack");
        expect(ratios(out)).toEqual([0.5, 0.5]);
        expect(countViews(out)).toBe(3);
    });
});

describe("removeNode", () => {
    it("rescales the survivors by 1/(1 - removed)", () => {
        const tree = hstack(
            "root",
            at(0.5, pipe("a")),
            at(0.25, asmA("b")),
            at(0.25, asmB("c")),
        );
        const out = pure(tree, () => removeNode(tree, "a"));
        checkTree(out);
        expect(ids(out)).toEqual(["b", "c"]);
        expect(ratios(out)).toEqual([0.5, 0.5]);
    });

    it("keeps uneven survivors proportional", () => {
        const tree = hstack(
            "root",
            at(0.2, pipe("a")),
            at(0.2, asmA("b")),
            at(0.6, asmB("c")),
        );
        const out = removeNode(tree, "b");
        checkTree(out);
        expect(ratios(out)[0]).toBeCloseTo(0.25, 10);
        expect(ratios(out)[1]).toBeCloseTo(0.75, 10);
    });

    it("collapses the stack when one element is left, which inherits the slot", () => {
        const tree = vstack(
            "root",
            at(0.3, hstack("inner", at(0.5, pipe("a")), at(0.5, asmA("b")))),
            at(0.7, asmB("c")),
        );
        const out = removeNode(tree, "a");
        checkTree(out);
        expect(ids(out)).toEqual(["b", "c"]);
        expect(ratios(out)).toEqual([0.3, 0.7]);
    });

    it("empties the workspace when the last view goes", () => {
        expect(removeNode(pipe("a"), "a")).toBeNull();
        const pair = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const one = removeNode(pair, "a");
        checkTree(one);
        expect(one?.id).toBe("b");
        expect(removeNode(one, "b")).toBeNull();
        expect(removeNode(null, "a")).toBeNull();
    });

    it("removing a container takes its whole subtree", () => {
        const tree = hstack(
            "root",
            at(0.5, vstack("inner", at(0.5, pipe("a")), at(0.5, asmA("b")))),
            at(0.25, asmB("c")),
            at(0.25, pipe("d")),
        );
        const out = removeNode(tree, "inner");
        checkTree(out);
        expect(collectViewIds(out)).toEqual(["c", "d"]);
    });

    it("keeps a tab group pointing at the same tab", () => {
        const tree = tabs("t", 1, pipe("a"), asmA("b"), asmB("c"));
        const out = removeNode(tree, "a");
        checkTree(out);
        expect(out).toMatchObject({ kind: "tabs", active: 0 });
        expect(collectViewIds(out)).toEqual(["b", "c"]);

        const later = removeNode(tree, "c");
        checkTree(later);
        expect(later).toMatchObject({ kind: "tabs", active: 1 });
    });

    it("collapses a two-tab group down to the survivor", () => {
        const tree = hstack("root", at(0.4, tabs("t", 0, pipe("a"), asmA("b"))), at(0.6, asmB("c")));
        const out = removeNode(tree, "a");
        checkTree(out);
        expect(ids(out)).toEqual(["b", "c"]);
        expect(findNode(out, "t")).toBeNull();
    });

    it("leaves the tree alone for an unknown id", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        expect(removeNode(tree, "nope")).toBe(tree);
    });
});

describe("moveNode", () => {
    it("moves a frame and keeps its node identity", () => {
        const tree = hstack(
            "root",
            at(0.5, pipe("a")),
            at(0.25, asmA("b")),
            at(0.25, vstack("inner", at(0.5, asmB("c")), at(0.5, pipe("d")))),
        );
        const out = pure(tree, () => moveNode(tree, "a", "c", "bottom"));
        checkTree(out);
        expect(findNode(out, "a")).toBe(findNode(tree, "a"));
        expect(countViews(out)).toBe(4);
        expect(collectViewIds(out)).toEqual(["b", "c", "a", "d"]);
    });

    it("round-trips back to the original layout", () => {
        const original = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const moved = moveNode(original, "a", "b", "right");
        checkTree(moved);
        expect(ids(moved)).toEqual(["b", "a"]);
        const back = moveNode(moved, "a", "b", "left");
        checkTree(back);
        expect(ids(back)).toEqual(["a", "b"]);
        expect(ratios(back)).toEqual([0.5, 0.5]);
    });

    it("refuses to drop a node into its own subtree", () => {
        const tree = hstack(
            "root",
            at(0.5, vstack("inner", at(0.5, pipe("a")), at(0.5, asmA("b")))),
            at(0.5, asmB("c")),
        );
        expect(moveNode(tree, "inner", "a", "left")).toBe(tree);
        expect(moveNode(tree, "root", "c", "center")).toBe(tree);
    });

    it("refuses self, unknown nodes and an empty workspace", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        expect(moveNode(tree, "a", "a", "left")).toBe(tree);
        expect(moveNode(tree, "nope", "a", "left")).toBe(tree);
        expect(moveNode(tree, "a", "nope", "left")).toBe(tree);
        expect(moveNode(null, "a", "b", "left")).toBeNull();
    });

    it("bails out when removal takes the target with it", () => {
        // Dropping a tab onto its own two-tab group: the group collapses first.
        const tree = hstack("root", at(0.5, tabs("t", 0, pipe("a"), asmA("b"))), at(0.5, asmB("c")));
        expect(moveNode(tree, "a", "t", "center")).toBe(tree);
    });

    it("merges into an existing tab group", () => {
        const tree = hstack("root", at(0.5, tabs("t", 0, pipe("a"), asmA("b"))), at(0.5, asmB("c")));
        const out = moveNode(tree, "c", "t", "center");
        checkTree(out);
        expect(out?.id).toBe("t");
        if (out === null || out.kind !== "tabs") throw new Error("expected tabs");
        expect(out.children.map((child) => child.id)).toEqual(["a", "b", "c"]);
        expect(out.active).toBe(2);
    });

    it("reorders within one stack without disturbing anyone's size", () => {
        const tree = hstack(
            "root",
            at(0.4, pipe("a")),
            at(0.3, asmA("b")),
            at(0.3, asmB("c")),
        );
        const far = moveNode(tree, "a", "c", "right");
        checkTree(far);
        expect(ids(far)).toEqual(["b", "c", "a"]);
        expect(ratios(far)).toEqual([0.3, 0.3, 0.4]);
        expect(far?.id).toBe("root");

        const near = moveNode(tree, "a", "b", "right");
        checkTree(near);
        expect(ids(near)).toEqual(["b", "a", "c"]);
        expect(ratios(near)).toEqual([0.3, 0.4, 0.3]);
    });

    it("leaves an uneven split completely alone on a no-op drop", () => {
        const tree = hstack(
            "root",
            at(0.4, pipe("a")),
            at(0.3, asmA("b")),
            at(0.3, asmB("c")),
        );
        // Both of these resolve to where "a" already is.
        expect(moveNode(tree, "a", "b", "left")).toBe(tree);
        expect(moveNode(tree, "b", "a", "right")).toBe(tree);
        expect(moveToRoot(tree, "a", "left")).toBe(tree);
        expect(moveToRoot(tree, "c", "right")).toBe(tree);
    });

    it("swaps two views of the same type", () => {
        // Same payload, different instances: each pane owns its own pan and
        // scroll state, so exchanging them is a real move.
        const twins = hstack("root", at(0.5, pipe("p1")), at(0.5, pipe("p2")));
        expect(canDrop(twins, "p1", "p2", "right")).toBe(true);
        const out = moveNode(twins, "p1", "p2", "right");
        checkTree(out);
        expect(ids(out)).toEqual(["p2", "p1"]);
        expect(canDrop(twins, "p1", "p2", "left")).toBe(false);
    });

    it("reorders to a root edge keeping ratios", () => {
        const tree = hstack(
            "root",
            at(0.4, pipe("a")),
            at(0.3, asmA("b")),
            at(0.3, asmB("c")),
        );
        const out = moveToRoot(tree, "a", "right");
        checkTree(out);
        expect(ids(out)).toEqual(["b", "c", "a"]);
        expect(ratios(out)).toEqual([0.3, 0.3, 0.4]);
    });

    it("moves a whole subtree", () => {
        const tree = hstack(
            "root",
            at(0.5, vstack("inner", at(0.5, pipe("a")), at(0.5, asmA("b")))),
            at(0.25, asmB("c")),
            at(0.25, pipe("d")),
        );
        const out = moveNode(tree, "inner", "d", "bottom");
        checkTree(out);
        expect(countViews(out)).toBe(4);
        expect(collectViewIds(out)).toEqual(["c", "d", "a", "b"]);
    });
});

describe("insertAtRoot and moveToRoot", () => {
    it("adopts the node when the workspace is empty", () => {
        const node = pipe("a");
        expect(insertAtRoot(null, node, "left")).toBe(node);
    });

    it("wraps the whole root 50/50", () => {
        const tree = vstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const out = pure(tree, () => insertAtRoot(tree, asmB("new"), "left"));
        checkTree(out);
        expect(out?.kind).toBe("hstack");
        expect(ids(out)).toEqual(["new", "root"]);
        expect(ratios(out)).toEqual([0.5, 0.5]);
    });

    it("appends on the trailing edge", () => {
        const tree = vstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const out = insertAtRoot(tree, asmB("new"), "right");
        checkTree(out);
        expect(ids(out)).toEqual(["root", "new"]);
    });

    it("splices into a root stack of the same axis", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        const out = insertAtRoot(tree, asmB("new"), "left");
        checkTree(out);
        expect(ids(out)).toEqual(["new", "a", "b"]);
        expect(ratios(out)).toEqual([0.5, 0.25, 0.25]);
    });

    it("moves a frame out to a root edge", () => {
        const tree = hstack(
            "root",
            at(0.5, pipe("a")),
            at(0.25, asmA("b")),
            at(0.25, asmB("c")),
        );
        const out = moveToRoot(tree, "c", "top");
        checkTree(out);
        expect(out?.kind).toBe("vstack");
        expect(ids(out)[0]).toBe("c");
        expect(ratios(out)).toEqual([0.5, 0.5]);
        expect(collectViewIds(out)).toEqual(["c", "a", "b"]);
    });

    it("keeps a lone root view as the root", () => {
        const tree = pipe("a");
        const out = moveToRoot(tree, "a", "left");
        checkTree(out);
        expect(out?.id).toBe("a");
    });

    it("leaves the tree alone for unknown ids and an empty workspace", () => {
        const tree = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        expect(moveToRoot(tree, "nope", "left")).toBe(tree);
        expect(moveToRoot(null, "a", "left")).toBeNull();
    });
});

describe("resizeStack", () => {
    const tree = hstack(
        "root",
        at(0.25, pipe("a")),
        at(0.25, asmA("b")),
        at(0.5, asmB("c")),
    );

    it("moves weight between the two adjacent elements only", () => {
        const out = pure(tree, () => resizeStack(tree, "root", 0, 0.1, 0.05));
        checkTree(out);
        expect(ratios(out)[0]).toBeCloseTo(0.35, 10);
        expect(ratios(out)[1]).toBeCloseTo(0.15, 10);
        expect(ratios(out)[2]).toBe(0.5);
    });

    it("accepts a negative delta", () => {
        const out = resizeStack(tree, "root", 1, -0.2, 0.05);
        checkTree(out);
        expect(ratios(out)[0]).toBe(0.25);
        expect(ratios(out)[1]).toBeCloseTo(0.05, 10);
        expect(ratios(out)[2]).toBeCloseTo(0.7, 10);
    });

    it("clamps at minRatio on both sides", () => {
        const high = resizeStack(tree, "root", 0, 10, 0.1);
        checkTree(high);
        expect(ratios(high)[0]).toBeCloseTo(0.4, 10);
        expect(ratios(high)[1]).toBeCloseTo(0.1, 10);

        const low = resizeStack(tree, "root", 0, -10, 0.1);
        checkTree(low);
        expect(ratios(low)[0]).toBeCloseTo(0.1, 10);
        expect(ratios(low)[1]).toBeCloseTo(0.4, 10);
    });

    it("splits the pair evenly when minRatio cannot be met", () => {
        const out = resizeStack(tree, "root", 0, 10, 0.9);
        checkTree(out);
        expect(ratios(out)[0]).toBeCloseTo(0.25, 10);
        expect(ratios(out)[1]).toBeCloseTo(0.25, 10);
    });

    it("resizes a nested stack in place", () => {
        const nested = hstack(
            "root",
            at(0.5, pipe("a")),
            at(0.5, vstack("inner", at(0.5, asmA("b")), at(0.5, asmB("c")))),
        );
        const out = resizeStack(nested, "inner", 0, 0.25, 0.05);
        checkTree(out);
        expect(ratios(out)).toEqual([0.5, 0.5]);
        expect(ratios(elementsOf(out)[1].element)).toEqual([0.75, 0.25]);
    });

    it("rejects bad stacks, indices and deltas", () => {
        expect(resizeStack(tree, "a", 0, 0.1, 0.05)).toBe(tree);
        expect(resizeStack(tree, "nope", 0, 0.1, 0.05)).toBe(tree);
        expect(resizeStack(tree, "root", -1, 0.1, 0.05)).toBe(tree);
        expect(resizeStack(tree, "root", 2, 0.1, 0.05)).toBe(tree);
        expect(resizeStack(tree, "root", 0, NaN, 0.05)).toBe(tree);
        expect(resizeStack(null, "root", 0, 0.1, 0.05)).toBeNull();
    });
});

describe("setActiveTab", () => {
    const tree = hstack(
        "root",
        at(0.5, tabs("t", 0, pipe("a"), asmA("b"), asmB("c"))),
        at(0.5, pipe("d")),
    );

    it("switches tabs", () => {
        const out = pure(tree, () => setActiveTab(tree, "t", 2));
        checkTree(out);
        expect(findNode(out, "t")).toMatchObject({ active: 2 });
    });

    it("clamps out-of-range indices", () => {
        expect(findNode(setActiveTab(tree, "t", 99), "t")).toMatchObject({ active: 2 });
        expect(findNode(setActiveTab(tree, "t", -5), "t")).toMatchObject({ active: 0 });
        expect(findNode(setActiveTab(tree, "t", 1.9), "t")).toMatchObject({ active: 1 });
    });

    it("leaves the tree alone for non-tabs, unknown ids and no-ops", () => {
        expect(setActiveTab(tree, "t", 0)).toBe(tree);
        expect(setActiveTab(tree, "root", 1)).toBe(tree);
        expect(setActiveTab(tree, "a", 1)).toBe(tree);
        expect(setActiveTab(tree, "nope", 1)).toBe(tree);
        expect(setActiveTab(null, "t", 1)).toBeNull();
    });
});

describe("canDrop", () => {
    const tree = hstack(
        "root",
        at(0.4, pipe("a")),
        at(0.3, asmA("b")),
        at(0.3, vstack("inner", at(0.5, asmB("c")), at(0.5, pipe("d")))),
    );

    it("rejects a drop on itself", () => {
        expect(canDrop(tree, "a", "a", "left")).toBe(false);
        expect(canDrop(tree, "a", "a", "center")).toBe(false);
    });

    it("rejects a drop into its own subtree", () => {
        expect(canDrop(tree, "inner", "c", "left")).toBe(false);
        expect(canDrop(tree, "inner", "d", "center")).toBe(false);
        expect(canDrop(tree, "root", "a", "top")).toBe(false);
    });

    it("rejects a drop that resolves to the current position", () => {
        // "a" already sits immediately left of "b".
        expect(canDrop(tree, "a", "b", "left")).toBe(false);
        expect(canDrop(tree, "b", "a", "right")).toBe(false);
        // "c" already sits immediately above "d" in the nested vstack.
        expect(canDrop(tree, "c", "d", "top")).toBe(false);
        expect(canDrop(tree, "d", "c", "bottom")).toBe(false);
    });

    it("rejects a merge back into the tab group a node already belongs to", () => {
        const grouped = hstack(
            "root",
            at(0.5, tabs("t", 0, pipe("a"), asmA("b"))),
            at(0.5, asmB("c")),
        );
        expect(canDrop(grouped, "a", "t", "center")).toBe(false);
        expect(canDrop(grouped, "c", "t", "center")).toBe(true);
    });

    it("rejects unknown nodes", () => {
        expect(canDrop(tree, "nope", "a", "left")).toBe(false);
        expect(canDrop(tree, "a", "nope", "left")).toBe(false);
        expect(canDrop(null, "a", "b", "left")).toBe(false);
    });

    it("rejects root-edge drops that change nothing", () => {
        const pair = hstack("root", at(0.5, pipe("a")), at(0.5, asmA("b")));
        expect(canDrop(pair, "a", null, "left")).toBe(false);
        expect(canDrop(pair, "b", null, "right")).toBe(false);
        expect(canDrop(pair, "a", null, "center")).toBe(false);
        expect(canDrop(pipe("a"), "a", null, "top")).toBe(false);
    });

    it("accepts real moves", () => {
        expect(canDrop(tree, "a", "b", "right")).toBe(true);
        expect(canDrop(tree, "a", "c", "center")).toBe(true);
        expect(canDrop(tree, "a", "d", "bottom")).toBe(true);
        expect(canDrop(tree, "c", "a", "left")).toBe(true);
        expect(canDrop(tree, "a", null, "right")).toBe(true);
        expect(canDrop(tree, "a", null, "bottom")).toBe(true);
    });

    it("never yields a broken tree, for any pair and position", () => {
        const positions = ["left", "right", "top", "bottom", "center"] as const;
        const everyId = ["root", "a", "b", "inner", "c", "d"];
        let accepted = 0;
        for (const dragged of everyId) {
            for (const target of [...everyId, null]) {
                for (const position of positions) {
                    const allowed = canDrop(tree, dragged, target, position);
                    const out =
                        target === null
                            ? position === "center"
                                ? tree
                                : moveToRoot(tree, dragged, position)
                            : moveNode(tree, dragged, target, position);
                    checkTree(out);
                    expect(countViews(out)).toBe(4);
                    if (allowed) accepted += 1;
                }
            }
        }
        expect(accepted).toBeGreaterThan(0);
    });
});

describe("setView", () => {
    const tree = hstack("root", at(0.4, pipe("a")), at(0.6, asmA("b")));

    it("swaps the view while keeping the node's id and ratio", () => {
        const next = setView(tree, "a", { type: "chooser" });
        const node = findNode(next, "a");
        expect(node).not.toBeNull();
        expect(node!.kind).toBe("view");
        expect(node!.kind === "view" && node!.view.type).toBe("chooser");
        // The slot is untouched, so a chosen view inherits the pane's size.
        expect(next!.kind === "hstack" && next!.elements[0].ratio).toBeCloseTo(0.4);
        expect(next!.kind === "hstack" && next!.elements[1].ratio).toBeCloseTo(0.6);
        checkTree(next);
    });

    it("leaves the tree alone for an unchanged view, a stack or an unknown id", () => {
        expect(setView(tree, "a", { type: "pipeline" })).toBe(tree);
        expect(setView(tree, "root", { type: "pipeline" })).toBe(tree);
        expect(setView(tree, "nope", { type: "pipeline" })).toBe(tree);
        expect(setView(null, "a", { type: "pipeline" })).toBeNull();
    });

    it("is pure", () => {
        const before = JSON.stringify(tree);
        setView(tree, "b", { type: "chooser" });
        expect(JSON.stringify(tree)).toBe(before);
    });
});
