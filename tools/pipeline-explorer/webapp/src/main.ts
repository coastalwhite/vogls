import "./style.css";
import { render as renderEditor } from "./editor.ts";
import { colors } from "./colors.ts";
import initialAsm from "./initialAsm.S?raw";
import type { Trace } from "./types.ts";

import { PipelineCanvas } from "./pipeline.ts";
import { ScrubberCanvas } from "./scrubber.ts";
import { pipelineSvg } from "./svg.ts";

let pending = 0;
let currentCycle = 0;

let timeoutId = null;

const pipelineStages: HTMLDivElement = document.getElementById(
    "pipelineStages",
)!;

const workspace: HTMLDivElement = document.getElementById(
    "workspace",
)!;
const scrubberContainer: HTMLDivElement = document.getElementById(
    "scrubberContainer",
)!;
const assemblyTextarea: HTMLTextAreaElement = document.getElementById(
    "assemblyTextarea",
)!;
const numCyclesInput: HTMLInputElement = document.getElementById(
    "numCyclesInput",
)!;
const procSelect: HTMLSelectElement = document.getElementById(
    "procSelect",
)!;
const simStatusElem: HTMLDivElement = document.getElementById(
    "simStatus",
)!;
const currentCycleElem: HTMLAnchorElement = document.getElementById(
    "currentCycle",
)!;
const totalCyclesElem: HTMLAnchorElement = document.getElementById(
    "totalCycles",
)!;
const procConfigDetail: HTMLAnchorElement = document.getElementById(
    "procConfigDetail",
)!;
const chipMenuBtn: HTMLButtonElement = document.getElementById(
    "chipMenuBtn",
)!;
const chipMenuPanel: HTMLDivElement = document.getElementById(
    "chipMenuPanel",
)!;
const chipMenuName: HTMLSpanElement = document.getElementById(
    "chipMenuName",
)!;
const editorElem: HTMLDivElement = document.querySelector("#left .editor")!;
const elfUploadBtn: HTMLButtonElement = document.getElementById(
    "elfUploadBtn",
)! as HTMLButtonElement;
const elfFileInput: HTMLInputElement = document.getElementById(
    "elfFileInput",
)! as HTMLInputElement;
const elfNameElem: HTMLSpanElement = document.getElementById("elfName")!;
const elfClearBtn: HTMLButtonElement = document.getElementById(
    "elfClearBtn",
)! as HTMLButtonElement;
const errorBar: HTMLDivElement = document.getElementById("errorBar")! as HTMLDivElement;
const errorMessage: HTMLSpanElement = document.getElementById("errorMessage")!;

let scrubber: ScrubberCanvas | null = null;
let currentTrace: Trace | null = null;
let pipeline: PipelineCanvas | null = null;

function percentForStage(trace: Trace, stageIdx: number): number {
    const numOccurances = trace.pipeline.traces[stageIdx].reduce(
        (a, v) => (v !== 0 ? a + 1 : a),
        0,
    );
    const fraction = numOccurances / trace.pipeline.cycles;
    return Math.round(fraction * 10000) / 100;
}

function setTrace(trace: Trace) {
    currentTrace = trace;
    if (pipeline === null) {
        pipeline = new PipelineCanvas(
            workspace,
            trace,
            currentCycle,
            setCurrentCycle,
        );
    } else {
        pipeline.setTrace(trace);
    }
    if (scrubber === null) {
        scrubber = new ScrubberCanvas(
            scrubberContainer,
            trace,
            currentCycle,
            setCurrentCycle,
        );
    } else {
        scrubber.setTrace(trace);
    }

    let s = "";
    for (let i = 0; i < trace.pipeline.keys.length; i++) {
        s += `
		<div class="stage-pipeline">
			${trace.pipeline.keys[i]}
			<div style="background-color: ${colors[i]};"></div>
			: ${percentForStage(trace, i)}%
		</div>
		`;
    }
    pipelineStages.innerHTML = s;
    totalCyclesElem.innerText = trace.pipeline.cycles.toString();
    onShareTrace();
}

function setCurrentCycle(cycle: number) {
    currentCycle = cycle;
    currentCycleElem.innerText = cycle.toString();
    if (scrubber !== null) scrubber.setCurrentCycle(cycle);
    if (pipeline !== null) {
        pipeline.setCurrentCycle(cycle);
        pipeline.moveToCycle(cycle, 0.25);
    }
}

const worker = new Worker(new URL("./wasm/trace-worker.ts", import.meta.url), {
    type: "module",
});
worker.onmessage = (e) => {
    pending -= 1;
    if ("error" in e.data) {
        // Keep showing the last good trace; the message says why it is stale.
        showError(e.data.error);
        if (pending == 0) simStatusElem.innerHTML = "";
        return;
    }

    clearError();
    setTrace(e.data);
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

type Elf = { name: string; bytes: Uint8Array };

let elf: Elf | null = null;
function setElf(next: Elf | null) {
    elf = next;
    elfNameElem.hidden = elfClearBtn.hidden = elf === null;
    editorElem.hidden = elf !== null;
    if (elf !== null) {
        elfNameElem.innerText = `${elf.name} (${elf.bytes.length} bytes)`;
        elfNameElem.title = elf.name;
    }
}

elfUploadBtn.addEventListener("click", () => elfFileInput.click());
elfFileInput.addEventListener("change", async () => {
    const file = elfFileInput.files?.[0];
    // Clear the input, or picking the same file again fires no `change`.
    elfFileInput.value = "";
    if (!file) return;
    setElf({ name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) });
    unstaggerRunSim();
});
elfClearBtn.addEventListener("click", () => {
    setElf(null);
    unstaggerRunSim();
});

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

function runSim() {
    const program = elf !== null ? elf.bytes : assemblyTextarea.value;
    const procSelectValue = procSelect.value;
    const numCycles = numCyclesInput.value;

    const proc = procSelectValue in procConfigFields ? procSelectValue : "ibex";

    worker.postMessage({
        "proc": proc,
        "program": program,
        "config": readProcConfig(proc),
        "numCycles": numCycles,
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

/** The program a link opens with: assembly, or an ELF file as base64url. */
type SharedProgram =
    | { kind: "asm"; text: string }
    | { kind: "elf"; name: string; data: string };

type ShareState = {
    proc: string;
    program: SharedProgram;
    config: Record<string, boolean>;
    cycles: string;
};

function isSharedProgram(p: any): p is SharedProgram {
    switch (p?.kind) {
        case "asm":
            return typeof p.text === "string";
        case "elf":
            return typeof p.name === "string" && typeof p.data === "string";
        default:
            return false;
    }
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
        program:
            elf !== null
                ? { kind: "elf", name: elf.name, data: bytesToBase64Url(elf.bytes) }
                : { kind: "asm", text: assemblyTextarea.value },
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
    const program = state?.program;
    assemblyTextarea.value = program?.kind === "asm" ? program.text : initialAsm;
    renderEditor();
    if (program?.kind === "elf") {
        setElf({ name: program.name, bytes: bytesFromBase64Url(program.data) });
    }
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

assemblyTextarea.addEventListener("input", staggerRunSim);
numCyclesInput.addEventListener("input", staggerRunSim);
procSelect.addEventListener("change", onProcSelect);

function setChipMenuOpen(open: boolean) {
    chipMenuPanel.hidden = !open;
    chipMenuBtn.setAttribute("aria-expanded", open ? "true" : "false");
}

chipMenuBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    setChipMenuOpen(chipMenuPanel.hidden);
});

// Clicks inside the panel must not reach the document handler below, or
// picking a chip or typing a cycle count would close the menu.
chipMenuPanel.addEventListener("click", (e) => e.stopPropagation());

document.addEventListener("click", () => setChipMenuOpen(false));
document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !chipMenuPanel.hidden) {
        setChipMenuOpen(false);
        chipMenuBtn.focus();
    }
});

/* ---------- share menu ---------- */

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

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

function setShareOpen(open: boolean) {
    sharePanel.hidden = !open;
    shareBtn.setAttribute("aria-expanded", open ? "true" : "false");
    if (!open) return;
    // Default to the part of the pipeline that is on screen.
    if (pipeline !== null) [svgStart, svgEnd] = pipeline.visibleCycles();
    shareLinkInput.value = shareLink();
    onShareTrace();
}

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

shareBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    setChipMenuOpen(false);
    setShareOpen(sharePanel.hidden !== false);
});
sharePanel.addEventListener("click", (e) => e.stopPropagation());
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

document.addEventListener("click", () => setShareOpen(false));
document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !sharePanel.hidden) {
        setShareOpen(false);
        shareBtn.focus();
    }
});
