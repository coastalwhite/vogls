import { openMenu } from "../menu.ts";
import type { MenuItem } from "../menu.ts";

export interface FrameContext {
    /** One entry per child. A view node has exactly one. */
    titles: string[];
    /** Index into `titles`. Always 0 for a view node. */
    activeIndex: number;
    /** One per child, in the same order as `titles`. The view roots. */
    bodies: HTMLElement[];
    /** Items for the ⋯ menu, built fresh each time it opens. */
    menuItems(): MenuItem[];
    onSelectTab(index: number): void;
    /** Titlebar/tab pointerdown, for the drag subsystem. */
    onGrab(childIndex: number, e: PointerEvent): void;
    /** Chrome-background pointerdown: drags the frame, tab group and all. */
    onGrabFrame(e: PointerEvent): void;
    /** False when there is nothing to drag onto (single-frame workspace). */
    draggable: boolean;
    /** Opens a chooser as a new tab. Only shown on tab groups. */
    onAddTab?(): void;
}

/** Distinguishes one frame's generated ids from the next frame's. */
let frameSeq = 0;

/**
 * The chrome around a view: a titlebar with a ⋯ menu, or a tab strip that
 * replaces the titlebar when the frame holds more than one child.
 *
 * `element` and every child element are created once and mutated in place, so
 * the reconciler can re-parent a frame and the views living inside it without
 * either losing state.
 */
export class Frame {
    readonly element: HTMLElement;

    private ctx: FrameContext;
    private readonly idPrefix = `ws-frame-${frameSeq++}`;
    private readonly chrome = elem("div", "ws-frame__chrome");
    private readonly titleEl = elem("span", "ws-frame__title");
    private readonly tablist = elem("div", "ws-frame__tabs");
    private readonly addButton = elem("button", "ws-tab-add");
    private readonly menuButton = elem("button", "ws-frame__menu");
    private readonly bodyEl = elem("div", "ws-frame__body");
    /** Index-stable: entry `i` is always the tab and panel for child `i`. */
    private readonly tabs: HTMLButtonElement[] = [];
    private readonly panels: HTMLElement[] = [];

    constructor(ctx: FrameContext) {
        this.ctx = ctx;
        this.element = elem("div", "ws-frame");
        this.element.setAttribute("role", "region");

        this.titleEl.addEventListener("pointerdown", (e) => {
            this.grab(0, e);
        });

        // The chrome's own background -- the gap beside the tabs -- moves the
        // whole frame. Tabs, title and the menu button are descendants, so
        // their own pointerdowns never reach this.
        this.chrome.addEventListener("pointerdown", (e) => {
            if (e.target !== this.chrome) return;
            if (!this.ctx.draggable || e.button !== 0 || !e.isPrimary) return;
            this.ctx.onGrabFrame(e);
        });

        this.tablist.setAttribute("role", "tablist");
        this.tablist.addEventListener("keydown", (e) => this.onTabKey(e));

        this.menuButton.type = "button";
        this.menuButton.textContent = "⋯";
        this.menuButton.setAttribute("aria-haspopup", "menu");
        this.menuButton.setAttribute("aria-expanded", "false");
        this.menuButton.setAttribute("aria-label", "Frame options");
        // Built at click time: which actions apply depends on the live tree.
        this.menuButton.addEventListener("click", () => {
            openMenu(this.menuButton, this.ctx.menuItems());
        });

        this.addButton.type = "button";
        this.addButton.textContent = "+";
        this.addButton.setAttribute("aria-label", "Add a view");
        this.addButton.title = "Add a view";
        this.addButton.addEventListener("click", () => this.ctx.onAddTab?.());

        this.chrome.appendChild(this.addButton);
        this.chrome.appendChild(this.menuButton);
        this.element.appendChild(this.chrome);
        this.element.appendChild(this.bodyEl);

        this.update(ctx);
    }

    update(ctx: FrameContext) {
        this.ctx = ctx;
        const count = this.count();
        const active = this.activeIndex();
        // A single child means a view node: the tree collapses one-tab groups.
        const tabGroup = count > 1;

        setAttr(this.element, "aria-label", ctx.titles[active] ?? "");

        this.syncTabs(tabGroup ? count : 0, active);
        this.syncPanels(count, active, tabGroup);

        const chromeLabel = tabGroup ? this.tablist : this.titleEl;
        const unused = tabGroup ? this.titleEl : this.tablist;
        if (!tabGroup) setText(this.titleEl, ctx.titles[0] ?? "");
        if (unused.parentNode !== null) unused.remove();
        if (chromeLabel.parentNode === null) {
            this.chrome.insertBefore(chromeLabel, this.addButton);
        }
        this.addButton.hidden = !tabGroup || ctx.onAddTab === undefined;
    }

    /** Adds/removes `.is-dragging` on the frame. */
    setDragging(dragging: boolean) {
        this.element.classList.toggle("is-dragging", dragging);
    }

    /** Titles and bodies are index-paired; trust neither length over the other. */
    private count(): number {
        return Math.min(this.ctx.titles.length, this.ctx.bodies.length);
    }

    private activeIndex(): number {
        const last = Math.max(this.count() - 1, 0);
        return Math.min(Math.max(this.ctx.activeIndex, 0), last);
    }

    private tabId(index: number): string {
        return `${this.idPrefix}-tab-${index}`;
    }

    private panelId(index: number): string {
        return `${this.idPrefix}-panel-${index}`;
    }

    /**
     * The ⋯ button is a sibling of the title and the tabs, never a descendant,
     * so its own pointerdown cannot start a drag.
     */
    private grab(childIndex: number, e: PointerEvent) {
        if (!this.ctx.draggable || e.button !== 0 || !e.isPrimary) return;
        this.ctx.onGrab(childIndex, e);
    }

    private syncTabs(count: number, active: number) {
        while (this.tabs.length < count) {
            const index = this.tabs.length;
            const tab = elem("button", "ws-tab");
            tab.type = "button";
            tab.setAttribute("role", "tab");
            tab.addEventListener("pointerdown", (e) => this.grab(index, e));
            tab.addEventListener("click", () => this.ctx.onSelectTab(index));
            this.tabs.push(tab);
        }
        for (const extra of this.tabs.splice(count)) extra.remove();

        for (let i = 0; i < count; i++) {
            const tab = this.tabs[i];
            setText(tab, this.ctx.titles[i]);
            setAttr(tab, "id", this.tabId(i));
            setAttr(tab, "aria-controls", this.panelId(i));
            setAttr(tab, "aria-selected", i === active ? "true" : "false");
            // Roving tabindex: the strip is one tab stop, arrows move within it.
            setAttr(tab, "tabindex", i === active ? "0" : "-1");
            tab.classList.toggle("is-active", i === active);
            if (this.tablist.children[i] !== tab) {
                this.tablist.insertBefore(tab, this.tablist.children[i] ?? null);
            }
        }
    }

    private syncPanels(count: number, active: number, tabGroup: boolean) {
        while (this.panels.length < count) {
            this.panels.push(elem("div", "ws-frame__panel"));
        }
        for (const extra of this.panels.splice(count)) extra.remove();

        for (let i = 0; i < count; i++) {
            const panel = this.panels[i];
            const body = this.ctx.bodies[i];
            setAttr(panel, "id", this.panelId(i));
            // A lone panel has no tab to be labelled by, so it stays roleless.
            setAttr(panel, "role", tabGroup ? "tabpanel" : null);
            setAttr(panel, "aria-labelledby", tabGroup ? this.tabId(i) : null);
            const hidden = tabGroup && i !== active;
            if (panel.hidden !== hidden) panel.hidden = hidden;
            // A view owns its root for life: move it in, never clone it.
            if (panel.firstChild !== body || panel.childNodes.length !== 1) {
                panel.replaceChildren(body);
            }
            if (this.bodyEl.children[i] !== panel) {
                this.bodyEl.insertBefore(panel, this.bodyEl.children[i] ?? null);
            }
        }
    }

    private onTabKey(e: KeyboardEvent) {
        const count = this.count();
        if (count === 0) return;
        const active = this.activeIndex();
        let next: number;
        switch (e.key) {
            case "ArrowLeft":
                next = (active - 1 + count) % count;
                break;
            case "ArrowRight":
                next = (active + 1) % count;
                break;
            case "Home":
                next = 0;
                break;
            case "End":
                next = count - 1;
                break;
            default:
                return;
        }
        e.preventDefault();
        if (next === active) return;
        this.ctx.onSelectTab(next);
        // Selection follows focus. The tab buttons survive the re-render that
        // `onSelectTab` triggers, so this lands on the element just activated.
        this.tabs[next]?.focus();
    }
}

function elem<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    node.className = className;
    return node;
}

function setText(node: HTMLElement, text: string) {
    if (node.textContent !== text) node.textContent = text;
}

function setAttr(node: HTMLElement, name: string, value: string | null) {
    if (value === null) {
        if (node.hasAttribute(name)) node.removeAttribute(name);
    } else if (node.getAttribute(name) !== value) {
        node.setAttribute(name, value);
    }
}
