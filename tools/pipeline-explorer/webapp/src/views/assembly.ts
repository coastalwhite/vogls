import { openMenu } from "../menu.ts";
import type { MenuItem } from "../menu.ts";
import type {
    View,
    ViewContext,
    ViewInstance,
    ViewSpec,
} from "../layout/types.ts";

const INDENT = "    ";

/* ---------- token tables ----------
   Module-level: expensive to build, and identical for every instance. */

const REGS: string[] = (() => {
    const r: string[] = [];
    for (let i = 0; i < 32; i++) {
        r.push("x" + i);
        r.push("f" + i);
    }
    r.push(
        "zero", "ra", "sp", "gp", "tp", "fp",
        "t0", "t1", "t2", "t3", "t4", "t5", "t6",
        "s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11",
        "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7",
        "ft0", "ft1", "ft2", "ft3", "ft4", "ft5", "ft6", "ft7", "ft8", "ft9", "ft10", "ft11",
        "fs0", "fs1", "fs2", "fs3", "fs4", "fs5", "fs6", "fs7", "fs8", "fs9", "fs10", "fs11",
        "fa0", "fa1", "fa2", "fa3", "fa4", "fa5", "fa6", "fa7",
        // a few common CSRs
        "mstatus", "mtvec", "mepc", "mcause", "mie", "mip", "mscratch", "mhartid", "misa",
        "satp", "sstatus", "stvec", "sepc", "scause", "sie", "sip", "sscratch",
    );
    return r.sort((x, y) => y.length - x.length); // longest-first: s11 before s1
})();

const INSTR: string[] = `
  lui auipc jal jalr
  beq bne blt bge bltu bgeu
  lb lh lw lbu lhu sb sh sw
  addi slti sltiu xori ori andi slli srli srai
  add sub sll slt sltu xor srl sra or and
  fence fence.i ecall ebreak
  csrrw csrrs csrrc csrrwi csrrsi csrrci
  lwu ld sd addiw slliw srliw sraiw addw subw sllw srlw sraw
  mul mulh mulhsu mulhu div divu rem remu
  mulw divw divuw remw remuw
  lr.w sc.w amoswap.w amoadd.w amoxor.w amoand.w amoor.w amomin.w amomax.w amominu.w amomaxu.w
  lr.d sc.d amoswap.d amoadd.d amoxor.d amoand.d amoor.d amomin.d amomax.d amominu.d amomaxu.d
  flw fsw fld fsd
  fmadd.s fmsub.s fnmsub.s fnmadd.s fadd.s fsub.s fmul.s fdiv.s fsqrt.s
  fsgnj.s fsgnjn.s fsgnjx.s fmin.s fmax.s feq.s flt.s fle.s fclass.s
  fcvt.w.s fcvt.wu.s fcvt.s.w fcvt.s.wu fmv.x.w fmv.w.x
  fadd.d fsub.d fmul.d fdiv.d fsqrt.d fcvt.s.d fcvt.d.s
  nop li la mv not neg negw sext.w seqz snez sltz sgtz
  beqz bnez blez bgez bltz bgtz bgt ble bgtu bleu
  j jr ret call tail
  csrr csrw csrs csrc csrwi csrsi csrci
  rdcycle rdtime rdinstret
  mret sret wfi sfence.vma
`.trim().split(/\s+/).sort((x, y) => y.length - x.length); // fence.i before fence

/* ---------- highlighting ---------- */

const HTML_ESCAPES: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
};

const esc = (s: string): string =>
    s.replace(/[&<>]/g, (c: string): string => HTML_ESCAPES[c] ?? c);

const reEsc = (s: string): string => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const RULES = new RegExp(
    String.raw`(#[^\n]*|\/\/[^\n]*|\/\*[\s\S]*?\*\/)` // 1 comment
    + String.raw`|("(?:[^"\\\n]|\\.)*")` // 2 string
    + String.raw`|((?:\d+|[.A-Za-z_$][\w$.]*):)` // 3 label
    + String.raw`|(\.[A-Za-z_][\w.]*)` // 4 directive
    + String.raw`|\b(` + REGS.map(reEsc).join("|") + String.raw`)\b` // 5 register
    + String.raw`|\b(` + INSTR.map(reEsc).join("|") + String.raw`)\b` // 6 instruction
    + String.raw`|\b(0[xX][0-9a-fA-F]+|\d+)\b`, // 7 number
    "g",
);

/** `String.replace` callback for `RULES`: one capture group per span class. */
function spanToken(
    m: string,
    c?: string,
    s?: string,
    l?: string,
    d?: string,
    r?: string,
    ins?: string,
    n?: string,
): string {
    const cls = c ? "c" : s ? "s" : l ? "l" : d ? "d" : r ? "r" : ins ? "i" : n ? "n" : "";
    return cls ? `<span class="${cls}">${m}</span>` : m;
}

/* ---------- line helpers ---------- */

const lineStartAt = (v: string, i: number): number => v.lastIndexOf("\n", i - 1) + 1;
const lineEndAt = (v: string, i: number): number => (v.indexOf("\n", i) + 1 || v.length + 1) - 1;

/** The leading run of spaces and tabs. Empty when the line has none. */
function indentOf(line: string): string {
    const m = /^[ \t]*/.exec(line);
    return m ? m[0] : "";
}

const OUTDENT_RE = new RegExp("^(\\t| {1," + INDENT.length + "})");

/* ---------- module state ---------- */

type ElfOpener = (name: string, bytes: Uint8Array) => void;

const instances = new Set<AssemblyView>();

/**
 * The program itself, held here rather than in any one view's textarea: views
 * come and go, and the program has to outlive them. Every view renders this
 * string, and editing one writes back to it.
 */
let program = "";

/** What the simulation runs. Valid with no assembly view open at all. */
export function assemblyText(): string {
    return program;
}

/** Replaces the program and re-renders every open assembly view. */
export function setAssemblyText(text: string): void {
    if (text === program) return;
    program = text;
    for (const instance of instances) instance.showProgram();
}

let elfOpener: ElfOpener | null = null;

function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    className?: string,
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (className !== undefined) node.className = className;
    return node;
}

/**
 * The assembly editor: a transparent-text `<textarea>` over a `<pre><code>`
 * highlight overlay. Instantiable more than once; every instance edits the
 * one shared program, so an edit in any of them re-runs the simulation.
 */
export class AssemblyView implements ViewInstance {
    readonly root: HTMLElement;
    readonly title: string = "Assembly";
    readonly primary: boolean;

    private readonly ctx: ViewContext;
    private readonly textarea: HTMLTextAreaElement;
    private readonly overlay: HTMLElement;
    private readonly menuButtons: HTMLButtonElement[] = [];
    private readonly fileInput: HTMLInputElement;

    constructor(view: Extract<View, { type: "assembly" }>, ctx: ViewContext) {
        this.ctx = ctx;
        this.primary = view.primary;

        this.root = el("div", "ws-assembly");

        const editor = el("div", "editor");
        const inner = el("div", "editor-inner");
        const pre = el("pre");
        pre.setAttribute("aria-hidden", "true");
        this.overlay = el("code");
        pre.append(this.overlay);

        this.textarea = el("textarea");
        this.textarea.spellcheck = false;
        this.textarea.setAttribute("autocapitalize", "off");
        this.textarea.setAttribute("autocomplete", "off");
        this.textarea.setAttribute("aria-label", "RISC-V assembly editor");

        inner.append(pre, this.textarea);
        editor.append(inner);

        const bar = el("div", "ws-assembly__menubar");
        bar.setAttribute("role", "menubar");
        bar.setAttribute("aria-label", "Assembly");
        for (const menu of this.menus()) {
            const btn = el("button", "ws-menubar__item");
            btn.type = "button";
            btn.setAttribute("role", "menuitem");
            btn.setAttribute("aria-haspopup", "menu");
            btn.setAttribute("aria-expanded", "false");
            btn.tabIndex = this.menuButtons.length === 0 ? 0 : -1;
            btn.textContent = menu.label;
            // Built at click time so the items can depend on current state.
            btn.addEventListener("click", () => openMenu(btn, menu.items()));
            bar.append(btn);
            this.menuButtons.push(btn);
        }
        bar.addEventListener("keydown", this.onMenubarKey);

        this.fileInput = el("input");
        this.fileInput.type = "file";
        this.fileInput.accept = ".elf,application/x-elf,application/x-executable";
        this.fileInput.hidden = true;
        bar.append(this.fileInput);

        this.root.append(bar, editor);

        this.textarea.addEventListener("input", this.onInput);
        this.textarea.addEventListener("keydown", this.onKeyDown);
        this.fileInput.addEventListener("change", this.onElfChange);

        instances.add(this);
        this.textarea.value = program;
        this.highlight();
    }


    /** Pulls this view back in line with the shared program. */
    showProgram(): void {
        if (this.textarea.value === program) return;
        this.textarea.value = program;
        this.textarea.scrollTop = 0;
        this.highlight();
    }

    destroy(): void {
        this.textarea.removeEventListener("input", this.onInput);
        this.textarea.removeEventListener("keydown", this.onKeyDown);
        this.fileInput.removeEventListener("change", this.onElfChange);
        instances.delete(this);
        this.root.remove();
    }

    private highlight(): void {
        // trailing newline so the last empty line still has height
        this.overlay.innerHTML = esc(this.textarea.value + "\n").replace(RULES, spanToken);
    }

    /* ---------- editing primitives ---------- */

    // Insert via execCommand so the browser's native undo stack survives.
    private insert(text: string): void {
        if (!document.execCommand("insertText", false, text)) {
            const ta = this.textarea;
            const { selectionStart: a, selectionEnd: b, value: v } = ta;
            ta.value = v.slice(0, a) + text + v.slice(b);
            ta.selectionStart = ta.selectionEnd = a + text.length;
            ta.dispatchEvent(new Event("input"));
        }
    }

    // One replacement = one undo step. Selection is restored afterwards.
    private replaceRange(
        from: number,
        to: number,
        text: string,
        selA?: number,
        selB?: number,
    ): void {
        this.textarea.setSelectionRange(from, to);
        this.insert(text);
        if (selA !== undefined) this.textarea.setSelectionRange(selA, selB ?? selA);
    }

    /* ---------- block indent / outdent ---------- */

    private shiftLines(dir: number): void {
        const ta = this.textarea;
        const v = ta.value;
        const a = ta.selectionStart;
        const selEnd = ta.selectionEnd;
        let b = selEnd;
        if (b > a && v[b - 1] === "\n") b--; // don't drag in the line after the selection

        const from = lineStartAt(v, a);
        const to = lineEndAt(v, b);
        const lines = v.slice(from, to).split("\n");

        let firstDelta = 0;
        let total = 0;
        const shifted = lines.map((line: string, i: number): string => {
            let delta = 0;
            if (dir > 0) {
                if (line.length || lines.length === 1) {
                    line = INDENT + line;
                    delta = INDENT.length;
                }
            } else {
                const m = OUTDENT_RE.exec(line);
                if (m) {
                    line = line.slice(m[0].length);
                    delta = -m[0].length;
                }
            }
            if (i === 0) firstDelta = delta;
            total += delta;
            return line;
        });

        if (!total) return;
        const newA = Math.max(from, a + firstDelta);
        const newB = Math.max(newA, selEnd + total);
        this.replaceRange(from, to, shifted.join("\n"), newA, newB);
    }

    /* ---------- line-comment toggle (Ctrl/Cmd + /) ---------- */

    private toggleComment(): void {
        const ta = this.textarea;
        const v = ta.value;
        const a = ta.selectionStart;
        let b = ta.selectionEnd;
        if (b > a && v[b - 1] === "\n") b--;

        const from = lineStartAt(v, a);
        const to = lineEndAt(v, b);
        const lines = v.slice(from, to).split("\n");
        const code = lines.filter((l: string): boolean => l.trim() !== "");
        const off = code.length > 0 && code.every((l: string): boolean => /^\s*#/.test(l));

        const next = lines.map((l: string): string => {
            if (!l.trim()) return l;
            if (off) return l.replace(/^(\s*)#\s?/, "$1");
            const pad = indentOf(l);
            return pad + "# " + l.slice(pad.length);
        }).join("\n");

        this.replaceRange(from, to, next, from, from + next.length);
    }

    /* ---------- listeners ---------- */

    private readonly onInput = (): void => {
        this.highlight();
        program = this.textarea.value;
        // Every view shows the same program, but not this one: rewriting the
        // textarea the user is typing in would drop the caret.
        for (const other of instances) {
            if (other !== this) other.showProgram();
        }
        this.ctx.requestRun(false);
    };

    private readonly onKeyDown = (e: KeyboardEvent): void => {
        const ta = this.textarea;
        const { selectionStart: a, selectionEnd: b, value: v } = ta;
        const collapsed = a === b;
        const from = lineStartAt(v, a);
        const beforeCare = v.slice(from, a);
        const next: string | undefined = v[a];

        /* Ctrl/Cmd + /  — toggle line comment */
        if ((e.ctrlKey || e.metaKey) && e.key === "/") {
            e.preventDefault();
            this.toggleComment();
            return;
        }

        /* Tab / Shift+Tab */
        if (e.key === "Tab") {
            e.preventDefault();
            const multiline = v.slice(a, b).includes("\n");
            if (e.shiftKey || multiline) this.shiftLines(e.shiftKey ? -1 : 1);
            else this.insert(INDENT);
            return;
        }

        /* Enter: keep indentation; add a level after a label line */
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            const indent = indentOf(beforeCare);
            const label = /:$/.test(beforeCare.trimEnd());
            this.insert("\n" + indent + (label ? INDENT : ""));
            return;
        }

        /* Quotes (for .string / .asciz): open a pair or step over the closer */
        if (collapsed && e.key === '"') {
            e.preventDefault();
            if (next === '"') {
                ta.setSelectionRange(a + 1, a + 1);
            } else {
                this.insert('""');
                ta.setSelectionRange(a + 1, a + 1);
            }
            return;
        }

        /* Parentheses for memory operands like 0(sp) */
        if (collapsed && e.key === "(" && !/[\w$]/.test(next ?? "")) {
            e.preventDefault();
            this.insert("()");
            ta.setSelectionRange(a + 1, a + 1);
            return;
        }
        if (collapsed && e.key === ")" && next === ")") {
            e.preventDefault();
            ta.setSelectionRange(a + 1, a + 1);
            return;
        }

        /* Backspace: eat a whole indent unit, or an empty pair */
        if (collapsed && e.key === "Backspace") {
            if (beforeCare.length && /^[ \t]+$/.test(beforeCare)) {
                e.preventDefault();
                const n = beforeCare.length % INDENT.length || INDENT.length;
                this.replaceRange(a - n, a, "");
                return;
            }
            const prev: string | undefined = v[a - 1];
            if ((prev === "(" && next === ")") || (prev === '"' && next === '"')) {
                e.preventDefault();
                this.replaceRange(a - 1, a + 1, "");
            }
        }
    };

    /** The menubar's fold-out menus. A new menu is one more entry here. */
    private menus(): { label: string; items: () => MenuItem[] }[] {
        return [
            {
                label: "File",
                items: () => [
                    {
                        kind: "item",
                        label: "Import Program…",
                        onSelect: () => this.fileInput.click(),
                    },
                ],
            },
        ];
    }

    /** Arrow keys walk the menubar, as a menubar is a single tab stop. */
    private readonly onMenubarKey = (e: KeyboardEvent): void => {
        const items = this.menuButtons;
        const from = items.indexOf(document.activeElement as HTMLButtonElement);
        if (from < 0 || items.length < 2) return;
        let to = from;
        if (e.key === "ArrowRight") to = (from + 1) % items.length;
        else if (e.key === "ArrowLeft") to = (from - 1 + items.length) % items.length;
        else if (e.key === "Home") to = 0;
        else if (e.key === "End") to = items.length - 1;
        else return;
        e.preventDefault();
        for (const [i, btn] of items.entries()) btn.tabIndex = i === to ? 0 : -1;
        items[to].focus();
    };

    private readonly onElfChange = async (): Promise<void> => {
        const file = this.fileInput.files?.[0];
        // Clear the input, or picking the same file again fires no `change`.
        this.fileInput.value = "";
        const open = elfOpener;
        if (!file || !open) return;
        open(file.name, new Uint8Array(await file.arrayBuffer()));
    };
}

export const assemblySpec: ViewSpec = {
    type: "assembly",
    label: "Assembly",
    create(view: View, ctx: ViewContext): ViewInstance {
        if (view.type !== "assembly") {
            throw new Error(`assemblySpec cannot create a "${view.type}" view`);
        }
        return new AssemblyView(view, ctx);
    },
};

/** The first assembly view opened, if one is still open. */
export function primaryAssembly(): AssemblyView | null {
    for (const instance of instances) {
        if (instance.primary) return instance;
    }
    return null;
}

/** How the program bar opens an ELF. Set once by main.ts. */
export function setElfOpener(open: ElfOpener): void {
    elfOpener = open;
}
