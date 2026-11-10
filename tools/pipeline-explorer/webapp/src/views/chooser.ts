import type {
    NodeId,
    ViewContext,
    ViewInstance,
    ViewSpec,
    ViewType,
} from "../layout/types.ts";

export type ChooserOption = { type: ViewType; label: string };

export type ChooserHandler = {
    /** The view types a chooser offers; never includes the chooser itself. */
    options(): ChooserOption[];
    /** Replaces the chooser's own node with the chosen view. */
    pick(nodeId: NodeId, type: ViewType): void;
};

let handler: ChooserHandler | null = null;

/** Set once by the workspace, before any chooser is built. */
export function setChooserHandler(next: ChooserHandler) {
    handler = next;
}

/**
 * The option list, shared by the chooser pane and the empty-workspace
 * placeholder so the two never drift apart.
 */
export function renderPicker(choose: (type: ViewType) => void): HTMLElement {
    const wrap = document.createElement("div");
    wrap.className = "ws-chooser";

    const hint = document.createElement("p");
    hint.className = "ws-chooser__hint";
    hint.innerText = "Choose a view";
    wrap.appendChild(hint);

    const options = document.createElement("div");
    options.className = "ws-chooser__options";
    for (const option of handler?.options() ?? []) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "ws-chooser__option";
        btn.innerText = option.label;
        btn.addEventListener("click", () => choose(option.type));
        options.appendChild(btn);
    }
    wrap.appendChild(options);
    return wrap;
}

/** A placeholder pane that asks what should live here. */
export class ChooserView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title = "New View";

    constructor(nodeId: NodeId) {
        this.root = renderPicker((type) => handler?.pick(nodeId, type));
    }

    destroy() {
        this.root.remove();
    }
}

export const chooserSpec: ViewSpec = {
    type: "chooser",
    label: "New View",
    // It is what a split creates, never something a split can contain.
    pickable: false,
    create: (_view: unknown, _ctx: ViewContext, nodeId: NodeId) =>
        new ChooserView(nodeId),
};
