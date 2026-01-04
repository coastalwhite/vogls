<script lang="ts">
  import Highlight, { LineNumbers } from 'svelte-highlight'
  import verilog from 'svelte-highlight/languages/verilog'
  import github from "svelte-highlight/styles/github";

  import EventInfo from './EventInfo.svelte'
  import SignalList from './SignalList.svelte'

  let fileName = $state("");
  let error = $state(null);
  let trace = $state(null);
  let file_focus = $state(0);
  let event_ptr = $state(0);
  let last_file_focus = $state(null);

  function determine_current_focus_file() {
  	if (trace === null) {
		return;
	}

	let e = trace.events[event_ptr];
    if (e.type !== "eval") {
        return;
    }

    let process = trace.processes[e.process];
    if (process.span === null) {
        return;
    }
	file_focus = process.span.file;
  }
  function scroll_to_current_highlight(el) {
  	if (trace === null) {
		return;
	}

	let e = trace.events[event_ptr];
	if (e.type !== "eval") {
		return;
	}

	let process = trace.processes[e.process];
	if (process.span === null) {
		return;
	}

	var rows = el.querySelectorAll('div table tbody tr');
	rows[process.span.line_range[0]].scrollIntoView({
		behavior: 'smooth',
		block: 'center'
	});
  }

  $effect(() => {
   	determine_current_focus_file();
	if (file_focus !== null) {
		scroll_to_current_highlight(document.querySelectorAll('.source-code')[file_focus]);
	}
  });

  function init(el, file_idx) {
	if (file_idx !== file_focus) {
		return;
	}

  	$effect(() => {
		scroll_to_current_highlight(el);
	});
  }


  type Timestamp = int;
  type FileIdx = int;
  type SignalIdx = int;
  type ProcessIdx = int;
  type DrivenIdx = int;
  type WokenIdx = int;
  type WatchIdx = int;
  type File = {
    name?: string,
    content: string,
  };
  type Span = {
    file?: int,
    byte_range?: [int, int],
  };
  type Process = {
    name?: string,
    location?: Span,
  };
  type Bits = {
    size: int,
    slice: Uint8Array,
  };
  type Signal = {
    name?: string,
    location?: Span,
    initial: Bits,
  };
  type Driven = {
    signal: int,
    value: Bits,
    woken_range: [int, int],
  };
  type TEvent = 
    { type: "eval", process: int, driven: [int, int], stop_reason:
      { type: "halt" } | { type: "wait", time: int } | { type: "wait_region", region: int } | { type: "watch_signals", signals: [int, int] }
    } |
    { type: "drive", signal: int, drive?: int } |
    { type: "time", time: int }
  ;
  type Trace = {
    files: [File],
    processes: [Process],
    signals: [Signal],
    driven: [Driven],
    woken: [ProcessIdx],
    watches: [SignalIdx],
    events: [TEvent],
  };

  function decode_opt_str(view: DataView, ptr: int): [string | null, int] {
      if (
        view.getUint32(ptr,   true) == 0xFFFF_FFFF &&
        view.getUint32(ptr+4, true) == 0xFFFF_FFFF
      ) {
        return [null, ptr + 8];
      }
      return decode_str(view, ptr);
  }
  function decode_str(view: DataView, ptr: int): [string, int] {
      console.assert(
        view.getUint32(ptr,   true) != 0xFFFF_FFFF ||
        view.getUint32(ptr+4, true) != 0xFFFF_FFFF
      );

      const length = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const decoder = new TextDecoder("utf-8");
      const slice = new Uint8Array(
        view.buffer, 
        view.byteOffset + ptr, 
        length
      );
      const str = decoder.decode(slice);
      return [str, ptr + length]
  }
  function decode_opt_span(view: DataView, ptr: int): [Span | null, number] {
      if (
        view.getUint32(ptr,   true) == 0xFFFF_FFFF &&
        view.getUint32(ptr+4, true) == 0xFFFF_FFFF
      ) {
        return [null, ptr + 24];
      }
      
      const file       = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const line_start = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const line_end   = Number(view.getBigUint64(ptr, true)); ptr += 8;
      return [{ file, line_range: [line_start, line_end] }, ptr]
  }
  function decode_bits(view: DataView, ptr: int): [Bits | null, number] {
      const size  = view.getUint32(ptr, true); ptr += 4;
      let num_bytes = size >> 3;
      if (size % 8 != 0) {
        num_bytes += 1;
      }
      const slice = new Uint8Array(
        view.buffer, 
        view.byteOffset + ptr, 
        num_bytes
      );
      return [{ size, slice }, ptr + num_bytes]
  }

  function get_highlighted_lines(e: TEvent): int[] {
    if (e.type !== "eval") {
        return [];
    }

    let process = trace.processes[e.process];
    if (process.span === null) {
        return [];
    }

    let lines = [];
    let i;
    for(i = process.span.line_range[0]; i < process.span.line_range[1]; i += 1) {
        lines.push(i);
    }
    return lines;
  }

  async function handleFileChange(event) {
    const file = event.target.files[0];
    if (!file) return;

    fileName = file.name;
    error = null;

    try {
      const buffer = await readFileAsUint8Array(file);

      let view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);

      const magic = view.getUint32(0, false)
      if (magic != 0x56_47_54_44) { // VGTD
          error = "Missing magic bytes. Wrong file type?"
          return;
      }

      let ptr = 4
      const files_len     = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const processes_len = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const signals_len   = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const driven_len    = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const woken_len     = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const watches_len   = Number(view.getBigUint64(ptr, true)); ptr += 8;
      const events_len    = Number(view.getBigUint64(ptr, true)); ptr += 8;

      trace = {
        files: [],
        processes: [],
        signals: [],
        driven: [],
        woken: [],
        watches: [],
        events: [],
      };
    
      let i;
      for (i = 0; i < files_len; i += 1) {
          let name;
          [name, ptr] = decode_opt_str(view, ptr);
          let content;
          [content, ptr] = decode_str(view, ptr);
          trace.files.push({ name, content });
      }

      for (i = 0; i < processes_len; i += 1) {
          let name;
          [name, ptr] = decode_opt_str(view, ptr);
          let span;
          [span, ptr] = decode_opt_span(view, ptr);
          trace.processes.push({ name, span });
      }

      for (i = 0; i < signals_len; i += 1) {
          let name;
          [name, ptr] = decode_opt_str(view, ptr);
          let span;
          [span, ptr] = decode_opt_span(view, ptr);
          let initial;
          [initial, ptr] = decode_bits(view, ptr);
          trace.signals.push({ name, span, initial });
      }

      for (i = 0; i < driven_len; i += 1) {
          const signal = Number(view.getBigUint64(ptr, true)); ptr += 8;
          let value;
          [value, ptr] = decode_bits(view, ptr);
          const woken_start = Number(view.getBigUint64(ptr, true)); ptr += 8;
          const woken_end   = Number(view.getBigUint64(ptr, true)); ptr += 8;
          trace.driven.push({ signal, value, woken_range: [woken_start, woken_end] });
      }

      for (i = 0; i < woken_len; i += 1) {
          const process = Number(view.getBigUint64(ptr, true)); ptr += 8;
          trace.woken.push(process);
      }

      for (i = 0; i < watches_len; i += 1) {
          const signal = Number(view.getBigUint64(ptr, true)); ptr += 8;
          trace.watches.push(signal);
      }

      for (i = 0; i < events_len; i += 1) {
          const ty_d   = view.getUint8(ptr);                   ptr += 1;
          let e;
          switch (ty_d) {
            case 0:
                const process       = Number(view.getBigUint64(ptr, true)); ptr += 8;
                const driven_start  = Number(view.getBigUint64(ptr, true)); ptr += 8;
                const driven_end    = Number(view.getBigUint64(ptr, true)); ptr += 8;
                const stop_reason_d = view.getUint8(ptr);                   ptr += 1;
                let stop_reason;
                switch (stop_reason_d) {
                  case 0:
                      stop_reason = { type: "halt" };
                      break;
                  case 1: 
                      const t = Number(view.getBigUint64(ptr, true)); ptr += 8;
                      stop_reason = { type: "wait", time: t };
                      break;
                  case 2: 
                      const region = view.getUint8(ptr); ptr += 1;
                      stop_reason = { type: "wait_region", region };
                      break;
                  case 3: 
                      const watches_start = Number(view.getBigUint64(ptr, true)); ptr += 8;
                      const watches_end   = Number(view.getBigUint64(ptr, true)); ptr += 8;
                      stop_reason = { type: "watch_signals", range: [watches_start, watches_end] };
                      break;
                  case 4:
                      error = "Invalid event stop reason discriminant";
                      return;
                }
                e = { type: "eval", process, driven: [driven_start, driven_end], stop_reason };
                break;
            case 1:
                const signal = Number(view.getBigUint64(ptr, true)); ptr += 8;
				let drive = null;
                if (
                  view.getUint32(ptr,   true) != 0xFFFF_FFFF ||
                  view.getUint32(ptr+4, true) != 0xFFFF_FFFF
                ) {
                    drive  = Number(view.getBigUint64(ptr, true));
                }
				ptr += 8;
                e = { type: "drive", signal, drive };
                break;
            case 2:
                const time = Number(view.getBigUint64(ptr, true)); ptr += 8;
                e = { type: "time", time };
                break;
            default: 
                error = "Invalid event discriminant";
                return;
          }

          trace.events.push(e);
      }
	  determine_current_focus_file();
    } catch (error) {
      error = "Failed to read file.";
      console.error(error);
    }
  }

  function readFileAsUint8Array(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();

      reader.onload = () => {
        // reader.result is an ArrayBuffer, we convert it to Uint8Array
        resolve(new Uint8Array(reader.result));
      };

      reader.onerror = () => reject(reader.error);
      reader.readAsArrayBuffer(file);
    });
  }
</script>

<svelte:head>
  {@html github}
</svelte:head>

<main>
  <div class="source">
  {#if trace}
      {#each trace.files as f, i}
	  <div class="source-code" class:source-code-hidden={i != file_focus} use:init={i}>
          <Highlight language={verilog} code={f.content} let:highlighted>
              <LineNumbers {highlighted}
                highlightedLines={i === file_focus ? get_highlighted_lines(trace.events[event_ptr]) : []}
              />
          </Highlight>
      </div>
      {/each}
  {/if}
  </div>
  <div class="sidebar">
	  <div class="events">
	  {#if trace}
		<EventInfo trace={trace} bind:ptr={event_ptr} />
	  {:else}
		<label for="file-upload" class="button">
		  {fileName ? 'Change File' : 'Select Local File'}
		</label>
		<input 
		  id="file-upload" 
		  type="file" 
		  onchange={handleFileChange} 
		/>

		{#if fileName}
		  <p class="status">Selected: <strong>{fileName}</strong></p>
		{/if}

		{#if error}
		  <p class="error">{error}</p>
		{/if}
	  {/if}
	  </div>
	  {#if trace}
	  <div class="signals">
		  <SignalList {trace} ptr={event_ptr} />
	  </div>
	  {/if}
	  
  </div>
</main>

<style>
  main {
      width: 100vw;
      height: 100vh;
      display: flex;
      flex-direction: row;
  }
  .source {
	  width: calc(100vw - 400px);
      height: 100%;
      border-right: 2px solid #000;
  }
  .source-code {
      width: 100%;
      height: 100%;
      --highlighted-background: #88CCCC;
      overflow-y: scroll;

      /* Hide scrollbar for Chrome, Safari and Opera */
      &::-webkit-scrollbar {
        display: none;
      }

      /* Hide scrollbar for IE, Edge and Firefox */
      -ms-overflow-style: none;  /* IE and Edge */
      scrollbar-width: none;     /* Firefox */
  }
  .source-code-hidden {
      display: none;
  }
  .sidebar {
      margin: auto;
	  width: 400px;
	  height: 100%;
	  display: flex;
	  flex-direction: column;
  }
  .events {
      overflow-y: scroll;

      /* Hide scrollbar for Chrome, Safari and Opera */
      &::-webkit-scrollbar {
        display: none;
      }

      /* Hide scrollbar for IE, Edge and Firefox */
      -ms-overflow-style: none;  /* IE and Edge */
      scrollbar-width: none;     /* Firefox */
  }
  .signals {
	  min-height: 200px;
	  max-height: 200px;
	  border-top: 2px solid #000;
  }
  .file-decoder {
    padding: 1rem;
    border: 2px dashed #ccc;
    border-radius: 8px;
    text-align: center;
  }

  input[type="file"] {
    display: none;
  }

  .button {
    display: inline-block;
    padding: 10px 20px;
    background-color: #ff3e00;
    color: white;
    border-radius: 4px;
    cursor: pointer;
    font-weight: bold;
  }

  .status { margin-top: 10px; color: #555; }
  .error { color: red; margin-top: 10px; }
</style>
