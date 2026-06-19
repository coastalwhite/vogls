import "./style.css";
import { colors } from "./colors.ts";
import initialAsm from "./initialAsm.S?raw";
import type { Trace } from "./types.ts";

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

const worker = new Worker(new URL("./wasm/trace-worker.ts", import.meta.url), {
    type: "module",
});
worker.onmessage = (e) => {
    setTrace(e.data);
    pending -= 1;
    if (pending == 0) {
        simStatusElem.innerHTML = `<img src="/check.svg"/>`;
    }
};

const procConfigFields = {
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
};

function runSim() {
    const assembly = assemblyTextarea.value;
    const procSelectValue = procSelect.value;
    const numCycles = numCyclesInput.value;

    let proc = "ibex";
    if (procSelectValue === "picorv32") {
        proc = "picorv32";
    } else if (procSelectValue === "ibex") {
        proc = "ibex";
    } else if (procSelectValue === "neorv32") {
        proc = "neorv32";
    }

    const config = {};
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

    worker.postMessage({
        "proc": proc,
        "asm": assembly,
        "config": config,
        "numCycles": numCycles,
    });
    pending += 1;
    simStatusElem.innerHTML = `<img src="/spinner.svg" class="spinner"/>`;
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
function onProcSelect() {
    const procSelectValue = procSelect.value;
    let s = `<table><colgroup><col span="1" style="width: 50%;"><col span="1" style="width: 50%;"></colgroup>`;
    for (const field of procConfigFields[procSelectValue]) {
        switch (field["type"]) {
            case "checkbox":
                s += `<tr><td>${field["title"]}</td><td><input type="checkbox" id="pcf-${field["id"]}" ${field["default"] ? 'checked' : ''} /></td></tr>`;
                break;
        }
    }
    s += '</table>'
    procConfigDetail.innerHTML = s;

    for (const field of procConfigFields[procSelectValue]) {
        const elem = document.getElementById(`pcf-${field["id"]}`);
        let f = staggerRunSim;
        switch (field["type"]) {
            case "checkbox": f = unstaggerRunSim; break;
        }
        elem.addEventListener("input", f);
    }

    unstaggerRunSim();
}
assemblyTextarea.textContent = initialAsm;
numCyclesInput.value = "500";
setCurrentCycle(0);
onProcSelect();

assemblyTextarea.addEventListener("input", staggerRunSim);
numCyclesInput.addEventListener("input", staggerRunSim);
procSelect.addEventListener("change", onProcSelect);

document.getElementById("prevCycle")!.addEventListener(
    "click",
    () => setCurrentCycle(currentCycle - 1),
);
document.getElementById("nextCycle")!.addEventListener(
    "click",
    () => setCurrentCycle(currentCycle + 1),
);
