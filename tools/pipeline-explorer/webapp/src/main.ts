import "./style.css";
import { render as renderEditor } from "./editor.ts";
import { colors } from "./colors.ts";
import initialAsm from "./initialAsm.S?raw";
import type { Trace } from "./types.ts";
import type { PluginField, PluginManifest } from "./wasm/plugin.ts";

import { PipelineCanvas } from "./pipeline.ts";
import { ScrubberCanvas } from "./scrubber.ts";

let pending = 0;
let currentCycle = 0;

let timeoutId = null;

const pipelineStages: HTMLDivElement = document.getElementById(
    "pipelineStages",
)!;

const pipelineContainer: HTMLDivElement = document.getElementById(
    "pipelineContainer",
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
const uploadInput: HTMLInputElement = document.getElementById(
    "uploadInput",
)!;
const statusMessageElem: HTMLDivElement = document.getElementById(
    "statusMessage",
)!;
let scrubber: ScrubberCanvas | null = null;
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
    if (pipeline === null) {
        pipeline = new PipelineCanvas(
            pipelineContainer,
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

function setStatusMessage(message: string) {
    statusMessageElem.innerText = message;
}

const worker = new Worker(new URL("./wasm/trace-worker.ts", import.meta.url), {
    type: "module",
});
worker.onmessage = (e) => {
    const message = e.data;
    switch (message.kind) {
        case "trace":
            setStatusMessage("");
            setTrace(message.trace);
            settlePending(true);
            break;
        case "pluginLoaded":
            addPlugin(message.proc, message.manifest);
            break;
        case "error":
            setStatusMessage(message.message);
            settlePending(false);
            break;
    }
};

function settlePending(ok: boolean) {
    // A plugin that failed to load never started a simulation, so only count
    // down when one was actually outstanding.
    if (pending === 0) return;
    pending -= 1;
    if (pending == 0) {
        simStatusElem.innerHTML = ok ? `<img src="check.svg"/>` : "!";
    }
}

const procConfigFields: Record<string, PluginField[]> = {
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
    const assembly = assemblyTextarea.value;
    const proc = procSelect.value;
    const numCycles = numCyclesInput.value;

    const fields = procConfigFields[proc];
    if (fields === undefined) {
        setStatusMessage(`unknown design '${proc}'`);
        return;
    }

    const config: Record<string, boolean | number> = {};
    for (const field of fields) {
        const elem = document.getElementById(`pcf-${field["id"]}`);
        if (!(elem instanceof HTMLInputElement)) {
            throw Error("Not an input");
        }
        switch (field["type"]) {
            case "checkbox":
                config[field["id"]] = elem.checked;
                break;
            case "number":
                config[field["id"]] = elem.valueAsNumber;
                break;
        }
    }

    worker.postMessage({
        "kind": "run",
        "proc": proc,
        "asm": assembly,
        "config": config,
        "numCycles": numCycles,
    });
    pending += 1;
    simStatusElem.innerHTML = `<img src="spinner.svg" class="spinner"/>`;
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
/** The chip menu entry that opens the file dialog instead of picking a design. */
const CUSTOM_OPTION = "__custom__";

/** The design that was showing, so "Custom…" can hand the menu back to it. */
let lastProc = procSelect.value;

function onProcSelect() {
    const procSelectValue = procSelect.value;

    if (procSelectValue === CUSTOM_OPTION) {
        // "Custom…" is an action, not a design. Put the menu straight back on
        // the design that was showing before opening the dialog: the app never
        // sits on an entry it cannot simulate, and a cancelled dialog needs no
        // handling of its own.
        procSelect.value = lastProc;
        uploadInput.click();
        return;
    }
    lastProc = procSelectValue;

    chipMenuName.innerText =
        procSelect.options[procSelect.selectedIndex]?.text ?? procSelectValue;
    const fields = procConfigFields[procSelectValue] ?? [];
    let s =`<table><colgroup><col span="1" style="width: 50%;"><col span="1" style="width: 50%;"></colgroup>`;
    for (const field of fields) {
        switch (field["type"]) {
            case "checkbox":
                s += `<tr><td>${field["title"]}</td><td><input type="checkbox" id="pcf-${field["id"]}" ${field["default"] ? 'checked' : ''} /></td></tr>`;
                break;
            case "number":
                s += `<tr><td>${field["title"]}</td><td><input type="number" id="pcf-${field["id"]}" value="${field["default"]}" /></td></tr>`;
                break;
        }
    }
    s += '</table>'
    // Leave it truly empty when the chip has no options, so the panel's
    // `:not(:empty)` separator stays off.
    procConfigDetail.innerHTML = fields.length ? s : "";

    for (const field of fields) {
        const elem = document.getElementById(`pcf-${field["id"]}`);
        let f = staggerRunSim;
        switch (field["type"]) {
            case "checkbox": f = unstaggerRunSim; break;
        }
        elem.addEventListener("input", f);
    }

    unstaggerRunSim();
}

/**
 * Registers an uploaded design and switches to it.
 *
 * The worker keys plugins by the id in their manifest, so re-uploading a newer
 * build of the same design replaces it instead of piling up duplicate entries
 * in the chip menu.
 */
function addPlugin(proc: string, manifest: PluginManifest) {
    procConfigFields[proc] = manifest.fields;

    let option = Array.from(procSelect.options).find((o) => o.value === proc);
    if (option === undefined) {
        option = document.createElement("option");
        option.value = proc;
        // Before "Custom…", so that entry stays at the bottom of the list.
        const custom = Array.from(procSelect.options)
            .find((o) => o.value === CUSTOM_OPTION);
        procSelect.add(option, custom ?? null);
    }
    option.text = manifest.name;

    procSelect.value = proc;
    setStatusMessage("");
    onProcSelect();
}

uploadInput.addEventListener("change", async () => {
    const file = uploadInput.files?.[0];
    // Clear it so picking the same file again still fires a change event,
    // which is what you want while iterating on a plugin build.
    uploadInput.value = "";
    if (file === undefined) return;

    setStatusMessage(`loading ${file.name}…`);
    const bytes = await file.arrayBuffer();
    worker.postMessage(
        { "kind": "loadPlugin", "name": file.name, "bytes": bytes },
        [bytes],
    );
});
assemblyTextarea.value = initialAsm;
renderEditor();
numCyclesInput.value = "500";
setCurrentCycle(0);
onProcSelect();

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
