import "./style.css";
import "./layout/layout.css";
import initialAsm from "./initialAsm.S?raw";
import type { Trace } from "./types.ts";
import type { WorkerReply, WorkerRequest } from "./wasm/trace-worker.ts";

import { ScrubberCanvas } from "./scrubber.ts";
import { pipelineSvg } from "./svg.ts";
import { Workspace } from "./layout/workspace.ts";
import type { CycleRange } from "./layout/types.ts";
import { assemblyText, setAssemblyText, setElfOpener } from "./views/assembly.ts";
import { anyPipelineView } from "./views/pipeline.ts";
import { registerPopover } from "./menu.ts";

let pending = 0;
let cycles: CycleRange = { start: 0, end: 0 };

let timeoutId: number | null = null;

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const central = el<HTMLDivElement>("central");
const scrubberContainer = el<HTMLDivElement>("scrubberContainer");
const numCyclesInput = el<HTMLInputElement>("numCyclesInput");
const procSelect = el<HTMLSelectElement>("procSelect");
const simStatusElem = el<HTMLDivElement>("simStatus");
const procConfigDetail = el<HTMLDivElement>("procConfigDetail");
const chipMenuBtn = el<HTMLButtonElement>("chipMenuBtn");
const chipMenuPanel = el<HTMLDivElement>("chipMenuPanel");
const chipMenuName = el<HTMLSpanElement>("chipMenuName");
const errorBar = el<HTMLDivElement>("errorBar");
const errorMessage = el<HTMLSpanElement>("errorMessage");

let scrubber: ScrubberCanvas | null = null;
let currentTrace: Trace | null = null;

/**
 * Read live by every view, so the fields are updated in place rather than
 * replaced. `setTrace`/`setCurrentCycle` below keep them current.
 */
const viewCtx = {
    trace: null as Trace | null,
    cycles,
    goToCycle: (cycle: number) => setCurrentCycle(cycle),
    selectCycles: (range: CycleRange) => setCycleRange(range),
    requestRun: (immediate: boolean) =>
        immediate ? unstaggerRunSim() : staggerRunSim(),
};

const workspace = new Workspace(central, viewCtx);

function setTrace(trace: Trace) {
    currentTrace = trace;
    viewCtx.trace = trace;
    workspace.broadcastTrace(trace);
    if (scrubber === null) {
        scrubber = new ScrubberCanvas(
            scrubberContainer,
            trace,
            cycles,
            setCurrentCycle,
        );
    } else {
        scrubber.setTrace(trace);
    }
    onShareTrace();
}

function setCurrentCycle(cycle: number) {
    setCycleRange({ start: cycle, end: cycle });
}

/** The selected cycles. Collapsed (`start === end`) is a single cycle. */
function setCycleRange(next: CycleRange) {
    const start = Math.min(next.start, next.end);
    const end = Math.max(next.start, next.end);
    cycles = { start, end };
    viewCtx.cycles = cycles;
    if (scrubber !== null) scrubber.setSelection(cycles);
    workspace.broadcastCycle(cycles);
}

const worker = new Worker(new URL("./wasm/trace-worker.ts", import.meta.url), {
    type: "module",
});
function postToWorker(request: WorkerRequest) {
    worker.postMessage(request);
}
worker.onmessage = (e: MessageEvent<WorkerReply>) => {
    const reply = e.data;
    if (reply.kind === "disassemble") {
        onDisassembled(reply);
        return;
    }

    pending -= 1;
    if ("error" in reply) {
        // Keep showing the last good trace; the message says why it is stale.
        showError(reply.error);
        if (pending == 0) simStatusElem.innerHTML = "";
        return;
    }

    clearError();
    setTrace(reply.trace as Trace);
    if (pending == 0) {
        simStatusElem.innerHTML = `<img src="check.svg"/>`;
    }
};

// The worker died, e.g. while loading the wasm module, and will not answer
// the runs still pending.
worker.onerror = (e) => {
    e.preventDefault();
    pending = 0;
    simStatusElem.innerHTML = "";
    showError(`The simulator stopped working: ${e.message || "unknown error"}`);
};

function showError(message: string) {
    errorMessage.innerText = message;
    errorBar.hidden = false;
}
function clearError() {
    errorBar.hidden = true;
    errorMessage.innerText = "";
}
document.getElementById("errorDismissBtn")!.addEventListener("click", clearError);

/**
 * Turns an ELF executable into assembly in the editor, laid out for the
 * selected core. The result arrives in `onDisassembled`.
 */
function openElf(name: string, elf: Uint8Array) {
    const proc = selectedProc();
    postToWorker({ kind: "disassemble", proc, config: readProcConfig(proc), elf, name });
    simStatusElem.innerHTML = `<img src="spinner.svg" class="spinner"/>`;
}

function onDisassembled(reply: Extract<WorkerReply, { kind: "disassemble" }>) {
    if ("error" in reply) {
        // The editor keeps what it had.
        showError(`Cannot open ${reply.name}: ${reply.error}`);
        if (pending == 0) simStatusElem.innerHTML = "";
        return;
    }
    clearError();
    setAssemblyText(reply.assembly);
    unstaggerRunSim();
}

setElfOpener(openElf);

type ConfigField = {
    id: string;
    type: "checkbox";
    title: string;
    default: boolean;
};

const procConfigFields: Record<string, ConfigField[]> = {
    "picorv32": [
        { "id": "enable_mul", "type": "checkbox", "title": "Enable MUL", 'default': true },
        { "id": "enable_div", "type": "checkbox", "title": "Enable DIV", 'default': true },
        { "id": "two_stage_shift", "type": "checkbox", "title": "Enable Two Stage Shift", "default": false },
        { "id": "barrel_shifter", "type": "checkbox", "title": "Enable Barrel Shifter", "default": false },
        { "id": "two_cycle_compare", "type": "checkbox", "title": "Enable Two Cycle Compare", "default": false },
        { "id": "two_cycle_alu", "type": "checkbox", "title": "Enable Two Cycle ALU", "default": false },
        { "id": "enable_fast_mul", "type": "checkbox", "title": "Enable FastMul", "default": false },
    ],
    "ibex": [
        { "id": "wb_stage", "type": "checkbox", "title": "Writeback Stage", 'default': false },
    ],
    "neorv32": [
    ],
    "hazard3": [
        { "id": "extension_m", "type": "checkbox", "title": "Enable M Extension", "default": true },
        { "id": "mul_fast", "type": "checkbox", "title": "Single-Cycle Multiply", "default": false },
        { "id": "mulh_fast", "type": "checkbox", "title": "Single-Cycle Mulh", "default": false },
        { "id": "muldiv_unroll_2", "type": "checkbox", "title": "Two Muldiv Steps per Cycle", "default": false },
        { "id": "reduced_bypass", "type": "checkbox", "title": "Reduced Bypass Network", "default": false },
        { "id": "branch_predictor", "type": "checkbox", "title": "Branch Predictor", "default": false },
        { "id": "fast_branchcmp", "type": "checkbox", "title": "Fast Branch Compare", "default": true },
    ],
};

function selectedProc(): string {
    return procSelect.value in procConfigFields ? procSelect.value : "ibex";
}

function runSim() {
    const proc = selectedProc();
    postToWorker({
        kind: "trace",
        proc,
        program: assemblyText(),
        config: readProcConfig(proc),
        numCycles: numCyclesInput.valueAsNumber,
    });
    pending += 1;
    simStatusElem.innerHTML = `<img src="spinner.svg" class="spinner"/>`;
}

function readProcConfig(proc: string): Record<string, boolean> {
    const config: Record<string, boolean> = {};
    for (const field of procConfigFields[proc]) {
        const elem = document.getElementById(`pcf-${field["id"]}`);
        if (!(elem instanceof HTMLInputElement)) {
            throw Error("Not an input");
        }
        switch (field["type"]) {
            case "checkbox":
                config[field["id"]] = elem.checked;
                break;
        }
    }
    return config;
}

function staggerRunSim() {
    if (timeoutId !== null) clearTimeout(timeoutId);
    timeoutId = setTimeout(() => {
        timeoutId = null;
        runSim();
    }, 1000);
}
function unstaggerRunSim() {
    if (timeoutId !== null) clearTimeout(timeoutId);
    timeoutId = null;
    runSim();
}
/** Rebuilds the config fields for the selected processor, at their defaults. */
function buildProcConfig() {
    const procSelectValue = procSelect.value;
    chipMenuName.innerText =
        procSelect.options[procSelect.selectedIndex]?.text ?? procSelectValue;
    let s =`<table><colgroup><col span="1" style="width: 50%;"><col span="1" style="width: 50%;"></colgroup>`;
    for (const field of procConfigFields[procSelectValue]) {
        switch (field["type"]) {
            case "checkbox":
                s += `<tr><td>${field["title"]}</td><td><input type="checkbox" id="pcf-${field["id"]}" ${field["default"] ? 'checked' : ''} /></td></tr>`;
                break;
        }
    }
    s += '</table>'
    // Leave it truly empty when the chip has no options, so the panel's
    // `:not(:empty)` separator stays off.
    procConfigDetail.innerHTML = procConfigFields[procSelectValue].length ? s : "";

    for (const field of procConfigFields[procSelectValue]) {
        const elem = document.getElementById(`pcf-${field["id"]}`);
        let f = staggerRunSim;
        switch (field["type"]) {
            case "checkbox": f = unstaggerRunSim; break;
        }
        elem!.addEventListener("input", f);
    }
}
function onProcSelect() {
    buildProcConfig();
    unstaggerRunSim();
}

/* ---------- shareable link ---------- */

/** The program a link opens with, as assembly. */
type SharedProgram = { kind: "asm"; text: string };

type ShareState = {
    proc: string;
    program: SharedProgram;
    config: Record<string, boolean>;
    cycles: string;
};

function isSharedProgram(p: any): p is SharedProgram {
    return p?.kind === "asm" && typeof p.text === "string";
}

const SHARE_PARAM = "share";

function bytesToBase64Url(bytes: Uint8Array): string {
    let bin = "";
    for (const b of bytes) bin += String.fromCharCode(b);
    return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
function bytesFromBase64Url(b64: string): Uint8Array {
    const bin = atob(b64.replace(/-/g, "+").replace(/_/g, "/"));
    return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}
function toBase64Url(text: string): string {
    return bytesToBase64Url(new TextEncoder().encode(text));
}
function fromBase64Url(b64: string): string {
    return new TextDecoder().decode(bytesFromBase64Url(b64));
}

function shareLink(): string {
    const proc = procSelect.value;
    const state: ShareState = {
        proc,
        program: { kind: "asm", text: assemblyText() },
        config: readProcConfig(proc),
        cycles: numCyclesInput.value,
    };
    const url = new URL(window.location.href);
    url.hash = `${SHARE_PARAM}=${toBase64Url(JSON.stringify(state))}`;
    return url.toString();
}

function readShareState(): ShareState | null {
    const params = new URLSearchParams(window.location.hash.slice(1));
    const encoded = params.get(SHARE_PARAM);
    if (!encoded) return null;
    try {
        const state = JSON.parse(fromBase64Url(encoded));
        // Links from before ELF uploads carry the assembly as `asm`.
        if (state.program === undefined && typeof state.asm === "string") {
            state.program = { kind: "asm", text: state.asm };
        }
        if (!(state.proc in procConfigFields) || !isSharedProgram(state.program)) {
            return null;
        }
        return state;
    } catch (e) {
        console.warn("Ignoring malformed share link", e);
        return null;
    }
}

/** Loads a shared state, falling back to the defaults for anything missing. */
function applyShareState(state: ShareState | null) {
    setAssemblyText(state?.program.text ?? initialAsm);
    numCyclesInput.value = state?.cycles ?? "500";
    if (state) procSelect.value = state.proc;
    buildProcConfig();
    for (const field of procConfigFields[procSelect.value]) {
        const value = state?.config?.[field.id];
        const elem = document.getElementById(`pcf-${field.id}`);
        if (typeof value === "boolean" && elem instanceof HTMLInputElement) {
            elem.checked = value;
        }
    }
}

setCurrentCycle(0);
applyShareState(readShareState());
unstaggerRunSim();

numCyclesInput.addEventListener("input", staggerRunSim);
procSelect.addEventListener("change", onProcSelect);

registerPopover({ trigger: chipMenuBtn, panel: chipMenuPanel });

/* ---------- share menu ---------- */

const shareBtn = el<HTMLButtonElement>("shareBtn");
const sharePanel = el<HTMLDivElement>("sharePanel");
const shareTabs = {
    image: [el<HTMLButtonElement>("shareTabImage"), el<HTMLDivElement>("sharePaneImage")],
    link: [el<HTMLButtonElement>("shareTabLink"), el<HTMLDivElement>("sharePaneLink")],
} as const;
const svgStartRange = el<HTMLInputElement>("svgStartRange");
const svgStartInput = el<HTMLInputElement>("svgStartInput");
const svgEndRange = el<HTMLInputElement>("svgEndRange");
const svgEndInput = el<HTMLInputElement>("svgEndInput");
const svgRangeFill = el<HTMLDivElement>("svgRangeFill");
const svgLimitPc = el<HTMLInputElement>("svgLimitPc");
const svgSpacing = el<HTMLInputElement>("svgSpacing");
const svgPreview = el<HTMLDivElement>("svgPreview");
const svgInfo = el<HTMLSpanElement>("svgInfo");
const shareLinkInput = el<HTMLInputElement>("shareLinkInput");

let svgStart = 0;
let svgEnd = 0;
let svgText = "";

function selectShareTab(tab: keyof typeof shareTabs) {
    for (const [name, [btn, pane]] of Object.entries(shareTabs)) {
        btn.setAttribute("aria-selected", name === tab ? "true" : "false");
        pane.hidden = name !== tab;
    }
    if (tab === "link") shareLinkInput.value = shareLink();
}

registerPopover({
    trigger: shareBtn,
    panel: sharePanel,
    onOpenChange: (open) => {
        if (!open) return;
        // Default to the part of the pipeline that is on screen.
        const visible = anyPipelineView()?.visibleCycles();
        if (visible) [svgStart, svgEnd] = visible;
        shareLinkInput.value = shareLink();
        onShareTrace();
    },
});

/** Clamps the range to the trace, syncs every control, and redraws. */
function updateSvg() {
    if (sharePanel.hidden || currentTrace === null) return;
    const last = Math.max(0, currentTrace.pipeline.cycles - 1);
    svgStart = Math.min(Math.max(0, Math.round(svgStart) || 0), last);
    svgEnd = Math.min(Math.max(svgStart, Math.round(svgEnd) || 0), last);
    for (const range of [svgStartRange, svgEndRange]) range.max = last.toString();
    svgStartRange.value = svgStartInput.value = svgStart.toString();
    svgEndRange.value = svgEndInput.value = svgEnd.toString();
    // The thumb centre travels across the track minus one thumb width.
    const frac = (v: number) => (last === 0 ? 0 : v / last);
    svgRangeFill.style.left = `calc(0.5rem + (100% - 1rem) * ${frac(svgStart)})`;
    svgRangeFill.style.right = `calc(0.5rem + (100% - 1rem) * ${1 - frac(svgEnd)})`;
    // When the thumbs overlap, the end thumb is on top; at the far right it
    // could then only move left, so lift the start thumb above it instead.
    svgStartRange.style.zIndex = svgStart === last ? "2" : "1";
    svgEndRange.style.zIndex = svgStart === last ? "1" : "2";
    svgSpacing.disabled = !svgLimitPc.checked;

    const spacing = Math.max(0, parseInt(svgSpacing.value) || 0);
    svgText = pipelineSvg(currentTrace, {
        start: svgStart,
        end: svgEnd,
        spacing: svgLimitPc.checked ? spacing : null,
    });
    svgPreview.innerHTML = svgText;
    svgInfo.innerText = `${svgEnd - svgStart + 1} cycles`;
}

function onShareTrace() {
    if (sharePanel.hidden) return;
    updateSvg();
}

shareTabs.image[0].addEventListener("click", () => selectShareTab("image"));
shareTabs.link[0].addEventListener("click", () => selectShareTab("link"));

// The start control pushes the end along (and vice versa) rather than
// refusing to move past it.
function setStart(v: number) {
    svgStart = v;
    if (svgEnd < svgStart) svgEnd = svgStart;
    updateSvg();
}
function setEnd(v: number) {
    svgEnd = v;
    if (svgStart > svgEnd) svgStart = svgEnd;
    updateSvg();
}
svgStartRange.addEventListener("input", () => setStart(svgStartRange.valueAsNumber));
svgEndRange.addEventListener("input", () => setEnd(svgEndRange.valueAsNumber));
svgStartInput.addEventListener("change", () => setStart(svgStartInput.valueAsNumber));
svgEndInput.addEventListener("change", () => setEnd(svgEndInput.valueAsNumber));
svgLimitPc.addEventListener("change", updateSvg);
svgSpacing.addEventListener("input", updateSvg);

el<HTMLButtonElement>("svgDownloadBtn").addEventListener("click", () => {
    const url = URL.createObjectURL(new Blob([svgText], { type: "image/svg+xml" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `pipeline-${procSelect.value}-${svgStart}-${svgEnd}.svg`;
    a.click();
    URL.revokeObjectURL(url);
});

async function copyWithFeedback(btn: HTMLButtonElement, text: string) {
    const label = btn.innerText;
    try {
        await navigator.clipboard.writeText(text);
        btn.innerText = "Copied";
    } catch {
        btn.innerText = "Copy failed";
    }
    setTimeout(() => (btn.innerText = label), 1500);
}
const svgCopyBtn = el<HTMLButtonElement>("svgCopyBtn");
svgCopyBtn.addEventListener("click", () => copyWithFeedback(svgCopyBtn, svgText));
const shareLinkCopyBtn = el<HTMLButtonElement>("shareLinkCopyBtn");
shareLinkCopyBtn.addEventListener("click", () => {
    shareLinkInput.value = shareLink();
    copyWithFeedback(shareLinkCopyBtn, shareLinkInput.value);
});
shareLinkInput.addEventListener("focus", () => shareLinkInput.select());


