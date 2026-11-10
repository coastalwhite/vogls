/**
 * A view popped out into a window of its own.
 *
 * The view's root element is *moved* into the new document with `adoptNode`,
 * not rebuilt: a canvas keeps its context and its drawing, listeners stay
 * bound, and its `ResizeObserver` keeps firing. The registry still holds the
 * instance, so it goes on receiving cycle and trace updates from here.
 */
export class DetachedWindow {
    private win: Window;
    private root: HTMLElement;
    private poll: number;
    private closed = false;
    private onClose: () => void;

    constructor(opts: { title: string; root: HTMLElement; onClose: () => void }) {
        const win = window.open("", "_blank", "width=720,height=520");
        if (win === null) throw new Error("the browser blocked the window");
        this.win = win;
        this.root = opts.root;
        this.onClose = opts.onClose;

        win.document.write("<!doctype html><html><head></head><body></body></html>");
        win.document.close();
        win.document.title = opts.title;

        // The popup starts with no styles of its own.
        for (const node of document.querySelectorAll("style, link[rel=stylesheet]")) {
            win.document.head.appendChild(win.document.importNode(node, true));
        }
        const body = win.document.body;
        body.style.cssText = "margin:0;height:100vh;display:flex";
        const host = win.document.createElement("div");
        host.className = "ws-detached";
        host.appendChild(win.document.adoptNode(opts.root));
        body.appendChild(host);

        // `pagehide` is the prompt signal; the poll is the one that can be
        // relied on, since a popup's unload events are not guaranteed.
        win.addEventListener("pagehide", () => this.finish());
        this.poll = window.setInterval(() => {
            if (this.win.closed) this.finish();
        }, 250);
    }

    /** Hands the root back to the opener and reports the window as gone. */
    private finish() {
        if (this.closed) return;
        this.closed = true;
        window.clearInterval(this.poll);
        // Pull the element back before the popup's document is torn down.
        document.adoptNode(this.root);
        this.onClose();
    }

    close() {
        if (this.closed) return;
        this.win.close();
        this.finish();
    }
}
