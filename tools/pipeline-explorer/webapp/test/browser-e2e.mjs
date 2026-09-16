// Drives the built site in a real headless Chromium and loads a design plugin
// the way a user would: "Custom…" in the design dropdown.
//
// `plugin-e2e.ts` covers the ABI; this covers the wiring around it -- the file
// picker, the worker round trip, the new entry in the chip menu, and a trace
// rendered from an uploaded .wasm.
//
// Run it with `just test-browser` from `tools/pipeline-explorer`.

import { spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const webapp = join(here, "..");
const pluginWasm = process.argv[2] ?? join(webapp, "../plugins/dist/hazard3.wasm");
const profile = mkdtempSync(join(tmpdir(), "pipeline-explorer-chrome-"));

/** Asks the OS for a free port, so a previous run cannot collide with this one. */
function freePort() {
    return new Promise((resolve, reject) => {
        const probe = createServer();
        probe.on("error", reject);
        probe.listen(0, "127.0.0.1", () => {
            const { port } = probe.address();
            probe.close(() => resolve(port));
        });
    });
}

const port = await freePort();

const failures = [];
const children = [];

function check(what, ok, detail = "") {
    if (ok) console.log(`  ok   ${what}`);
    else {
        console.log(`  FAIL ${what}${detail ? `: ${detail}` : ""}`);
        failures.push(what);
    }
}

async function cleanup() {
    for (const child of children) {
        // Kill the whole group: `npx` spawns vite as a child, and killing only
        // the wrapper would leave a server holding its port after we exit.
        try {
            process.kill(-child.pid, "SIGKILL");
        } catch {
            child.kill("SIGKILL");
        }
    }
    // Chromium can still be flushing its profile as it dies; losing a temp
    // directory matters far less than losing the test's verdict, so this never
    // throws.
    await new Promise((r) => setTimeout(r, 500));
    try {
        rmSync(profile, { recursive: true, force: true });
    } catch {
        // Left behind in the OS temp directory.
    }
}

async function waitFor(what, probe, timeoutMs = 120000) {
    const deadline = Date.now() + timeoutMs;
    for (;;) {
        try {
            const value = await probe();
            if (value) return value;
        } catch {
            // Not up yet.
        }
        if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
        await new Promise((r) => setTimeout(r, 250));
    }
}

// --- the preview server and the browser ------------------------------------

// `--host 127.0.0.1` pins it to IPv4: vite's default `localhost` can resolve to
// ::1, which is not where the probe below looks.
const preview = spawn(
    "npx",
    ["vite", "preview", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    { cwd: webapp, stdio: ["ignore", "pipe", "pipe"], detached: true },
);
children.push(preview);
preview.stderr.on("data", (d) => process.stderr.write(`[preview] ${d}`));

const chrome = spawn(
    "chromium",
    [
        "--headless=new",
        "--no-sandbox",
        "--disable-gpu",
        "--disable-dev-shm-usage",
        // Port 0 means "pick one"; chromium writes it to DevToolsActivePort.
        "--remote-debugging-port=0",
        `--user-data-dir=${profile}`,
        "about:blank",
    ],
    { stdio: ["ignore", "pipe", "pipe"], detached: true },
);
children.push(chrome);

try {
    await waitFor("the preview server", async () => (await fetch(`http://127.0.0.1:${port}/`)).ok);
    const devtoolsPort = await waitFor("chromium to pick a port", () =>
        Number(readFileSync(join(profile, "DevToolsActivePort"), "utf8").split("\n")[0]),
    );
    const devtools = `http://127.0.0.1:${devtoolsPort}`;
    const version = await waitFor("chromium", async () =>
        (await fetch(`${devtools}/json/version`)).json(),
    );
    console.log(`browser ${version.Browser}`);

    // --- connect to a fresh tab --------------------------------------------

    const target = await (
        await fetch(
            `${devtools}/json/new?${encodeURIComponent(`http://127.0.0.1:${port}/`)}`,
            { method: "PUT" },
        )
    ).json();

    const socket = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
        socket.onopen = resolve;
        socket.onerror = reject;
    });

    let nextId = 0;
    const pending = new Map();
    const consoleErrors = [];
    const seenEvents = [];
    socket.onmessage = (event) => {
        const message = JSON.parse(event.data);
        if (message.id !== undefined) {
            const entry = pending.get(message.id);
            pending.delete(message.id);
            if (message.error) entry.reject(new Error(JSON.stringify(message.error)));
            else entry.resolve(message.result);
            return;
        }
        seenEvents.push(message.method);
        if (message.method === "Runtime.consoleAPICalled" && message.params.type === "error") {
            consoleErrors.push(message.params.args.map((a) => a.value ?? a.description).join(" "));
        }
        if (message.method === "Runtime.exceptionThrown") {
            consoleErrors.push(message.params.exceptionDetails.text);
        }
    };

    function send(method, params = {}) {
        const id = ++nextId;
        socket.send(JSON.stringify({ id, method, params }));
        return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
    }

    // `userGesture` grants transient activation: opening a file picker needs
    // it, and a synthetic event dispatched from here has none of its own.
    async function evaluate(expression, { userGesture = false } = {}) {
        const result = await send("Runtime.evaluate", {
            expression,
            returnByValue: true,
            awaitPromise: true,
            userGesture,
        });
        if (result.exceptionDetails) {
            throw new Error(result.exceptionDetails.exception?.description ?? "evaluate failed");
        }
        return result.result.value;
    }

    await send("Runtime.enable");
    await send("Page.enable");
    await send("DOM.enable");
    await send("Page.navigate", { url: `http://127.0.0.1:${port}/` });

    // --- the page comes up on a built-in design ----------------------------

    console.log("initial load");
    await waitFor("the app to render", () => evaluate("!!document.getElementById('procSelect')"));
    const options = () =>
        evaluate(
            "Array.from(document.getElementById('procSelect').options).map(o => o.value).join(',')",
        );
    check(
        "offers the built-in designs plus Custom…",
        await options() === "picorv32,ibex,neorv32,hazard3,__custom__",
        await options(),
    );
    check(
        "labels the custom entry",
        await evaluate(
            "Array.from(document.getElementById('procSelect').options).at(-1).text",
        ) === "Custom…",
    );
    await waitFor("the first simulation", () =>
        evaluate("Number(document.getElementById('totalCycles').innerText) > 0"),
    );

    // --- "Custom…" opens the dialog without becoming the selection ---------

    // Intercepted rather than shown, so choosing "Custom…" surfaces as an
    // event instead of a dialog nobody can click in headless.
    await send("Page.setInterceptFileChooserDialog", { enabled: true });

    const before = await evaluate("document.getElementById('procSelect').value");
    await evaluate(
        "(() => { const s = document.getElementById('procSelect'); s.value = '__custom__';" +
            " s.dispatchEvent(new Event('change')); })()",
        { userGesture: true },
    );
    await waitFor("the file dialog", () => seenEvents.includes("Page.fileChooserOpened"), 10000);
    check("Custom… opens the file dialog", true);
    check(
        "Custom… hands the menu back to the current design",
        await evaluate("document.getElementById('procSelect').value") === before,
        await evaluate("document.getElementById('procSelect').value"),
    );

    // --- pick the plugin ---------------------------------------------------

    console.log(`uploading ${pluginWasm}`);
    const { root } = await send("DOM.getDocument");
    const { nodeId } = await send("DOM.querySelector", {
        nodeId: root.nodeId,
        selector: "#uploadInput",
    });
    await send("DOM.setFileInputFiles", { files: [pluginWasm], nodeId });

    await waitFor("the plugin to register", () =>
        evaluate("document.getElementById('procSelect').value.startsWith('plugin:')"),
    );
    check(
        "adds the uploaded design to the chip menu",
        await evaluate("document.getElementById('procSelect').value") === "plugin:hazard3",
    );
    check(
        "inserts it above Custom…",
        await options() === "picorv32,ibex,neorv32,hazard3,plugin:hazard3,__custom__",
        await options(),
    );
    check(
        "names it from the manifest",
        await evaluate("document.getElementById('chipMenuName').innerText") === "Hazard3 (plugin)",
    );
    check(
        "renders the manifest's knobs",
        await evaluate("document.querySelectorAll('#procConfigDetail input').length") === 7,
    );
    check(
        "honours the manifest defaults",
        await evaluate("document.getElementById('pcf-extension_m').checked") === true &&
            await evaluate("document.getElementById('pcf-mul_fast').checked") === false,
    );

    // --- and it actually simulates -----------------------------------------

    await waitFor("the plugin simulation", () =>
        evaluate("document.getElementById('simStatus').innerHTML.includes('check.svg')"),
    );
    const cycles = await evaluate("Number(document.getElementById('totalCycles').innerText)");
    check("runs the uploaded design", cycles > 0, `${cycles} cycles`);
    const stages = await evaluate(
        "Array.from(document.querySelectorAll('.stage-pipeline')).map(e => e.innerText.trim().split('\\n')[0]).join(',')",
    );
    check("shows the plugin's stages", stages === "F,X,M", stages);
    check(
        "reports no error",
        await evaluate("document.getElementById('statusMessage').innerText") === "",
        await evaluate("document.getElementById('statusMessage').innerText"),
    );

    // --- switching away and back still works -------------------------------

    await evaluate(
        "(() => { const s = document.getElementById('procSelect'); s.value = 'picorv32';" +
            " s.dispatchEvent(new Event('change')); })()",
    );
    await waitFor("the built-in design to come back", () =>
        evaluate("document.getElementById('chipMenuName').innerText === 'PicoRV32'"),
    );
    await evaluate(
        "(() => { const s = document.getElementById('procSelect'); s.value = 'plugin:hazard3';" +
            " s.dispatchEvent(new Event('change')); })()",
    );
    await waitFor("the plugin to come back", () =>
        evaluate("document.getElementById('simStatus').innerHTML.includes('check.svg')"),
    );
    check(
        "keeps the plugin selectable alongside the built-ins",
        await evaluate("Number(document.getElementById('totalCycles').innerText)") > 0,
    );

    check("logged no console errors", consoleErrors.length === 0, consoleErrors.join(" | "));
} finally {
    await cleanup();
}

if (failures.length > 0) {
    console.error(`\n${failures.length} check(s) failed`);
    process.exit(1);
}
console.log("\nall checks passed");
