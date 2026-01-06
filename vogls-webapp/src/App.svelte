<script lang="ts">
  import Highlight, { LineNumbers } from 'svelte-highlight'
  import verilog from 'svelte-highlight/languages/verilog'
  import github from "svelte-highlight/styles/github";

  import EventInfo from './EventInfo.svelte'
  import VCDViewer from './VCDViewer.svelte'
  import { parseTrace, type Trace, type TEvent } from './lib/trace.ts'

  let fileName = $state("");
  let error = $state(null);
  let trace = $state(null);
  let file_focus = $state(0);
  let event_ptr = $state(0);
  let last_file_focus = $state(null);
  let manual_file_selection = $state(false);
  let last_event_ptr = $state(-1);
  let highlighted_signal = $state<number | null>(null);
  let vcdPanelOpen = $state(false);
  let vcdPanelHeight = $state(300);
  let isDragging = $state(false);

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

  // Auto-switch file when event pointer changes
  $effect(() => {
	// When event pointer changes, always switch to the file containing the event
	// (manual file selection only prevents auto-switching during continuous navigation)
	if (event_ptr !== last_event_ptr) {
		// Always determine and switch to the file containing the current event
		determine_current_focus_file();
		// Reset manual selection when event changes (user clicked on a specific event)
		manual_file_selection = false;
		// Clear signal highlighting when event changes
		highlighted_signal = null;
		last_event_ptr = event_ptr;
	}
	
	// Always scroll to highlight when file_focus or event_ptr changes
	if (file_focus !== null && trace !== null) {
		// Find the currently visible file container
		const fileContainers = document.querySelectorAll('[data-file-container]');
		if (fileContainers[file_focus]) {
			scroll_to_current_highlight(fileContainers[file_focus]);
		}
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

  function get_current_event_file(): number | null {
    if (trace === null) {
      return null;
    }
    const e = trace.events[event_ptr];
    if (e.type !== "eval") {
      return null;
    }
    const process = trace.processes[e.process];
    if (process.span === null) {
      return null;
    }
    return process.span.file;
  }

  function navigateToSignalLocation(signalIdx: number) {
    if (trace === null) {
      return;
    }
    
    const signal = trace.signals[signalIdx];
    if (signal.span === null || signal.span.file === undefined) {
      return;
    }
    
    // Set highlighted signal
    highlighted_signal = signalIdx;
    
    // Switch to the file containing the signal
    file_focus = signal.span.file;
    manual_file_selection = true;
    
    // Scroll to the signal location
    setTimeout(() => {
      const fileContainers = document.querySelectorAll('[data-file-container]');
      if (fileContainers[signal.span.file]) {
        const el = fileContainers[signal.span.file];
        if (signal.span.line_range) {
          const rows = el.querySelectorAll('div table tbody tr');
          if (rows[signal.span.line_range[0]]) {
            rows[signal.span.line_range[0]].scrollIntoView({
              behavior: 'smooth',
              block: 'center'
            });
          }
        }
      }
    }, 0);
  }

  function get_highlighted_lines_for_signal(signalIdx: number | null): int[] {
    if (signalIdx === null || trace === null) {
      return [];
    }
    
    const signal = trace.signals[signalIdx];
    if (signal.span === null || signal.span.line_range === undefined) {
      return [];
    }
    
    let lines = [];
    let i;
    for(i = signal.span.line_range[0]; i < signal.span.line_range[1]; i += 1) {
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
      const { trace: parsedTrace, error: parseError } = parseTrace(buffer);
      
      if (parseError) {
        error = parseError;
        return;
      }
      
      if (parsedTrace) {
        trace = parsedTrace;
        determine_current_focus_file();
      }
    } catch (err) {
      error = "Failed to read file.";
      console.error(err);
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

  function handleMouseDown(e: MouseEvent) {
    isDragging = true;
    e.preventDefault();
  }

  function handleMouseMove(e: MouseEvent) {
    if (!isDragging) return;
    const newHeight = window.innerHeight - e.clientY;
    vcdPanelHeight = Math.max(200, Math.min(600, newHeight));
  }

  function handleMouseUp() {
    isDragging = false;
  }

  $effect(() => {
    if (isDragging) {
      window.addEventListener('mousemove', handleMouseMove);
      window.addEventListener('mouseup', handleMouseUp);
      return () => {
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
      };
    }
  });
</script>

<svelte:head>
  {@html github}
</svelte:head>

<main class="w-screen h-screen flex flex-col">
  <div class="flex-1 flex flex-row" style="height: {vcdPanelOpen ? `calc(100vh - ${vcdPanelHeight}px)` : '100vh'};">
    <div class="w-[calc(100vw-400px)] h-full border-r-2 border-gray-800 flex flex-col">
    {#if trace}
      <!-- File tabs -->
      {#if trace.files && trace.files.length > 0}
        <div class="flex border-b border-gray-300 bg-gray-50 overflow-x-auto scrollbar-hide flex-shrink-0">
          {#each trace.files as f, i}
            <button
              onclick={() => {
                file_focus = i;
                manual_file_selection = true;
              }}
              class="px-4 py-2 text-sm font-medium transition-colors whitespace-nowrap border-b-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-1"
              class:border-blue-600={i === file_focus}
              class:text-blue-600={i === file_focus}
              class:bg-white={i === file_focus}
              class:border-transparent={i !== file_focus}
              class:text-gray-600={i !== file_focus}
              class:hover:text-gray-900={i !== file_focus}
              class:hover:bg-gray-100={i !== file_focus}
            >
              {f.name || `File ${i}`}
            </button>
          {/each}
        </div>
      {/if}
      
      <!-- Source code viewer -->
      <div class="flex-1 overflow-hidden">
        {#each trace.files as f, i}
          <div 
            data-file-container
            class="w-full h-full overflow-y-scroll scrollbar-hide" 
            class:hidden={i != file_focus} 
            use:init={i}
          >
            <Highlight language={verilog} code={f.content} let:highlighted>
              <LineNumbers {highlighted}
                highlightedLines={(() => {
                  const eventFile = get_current_event_file();
                  const eventLines = eventFile !== null && i === eventFile ? get_highlighted_lines(trace.events[event_ptr]) : [];
                  const signalLines = highlighted_signal !== null && trace.signals[highlighted_signal].span?.file === i 
                    ? get_highlighted_lines_for_signal(highlighted_signal) 
                    : [];
                  // Combine both, removing duplicates
                  return [...new Set([...eventLines, ...signalLines])];
                })()}
              />
            </Highlight>
          </div>
        {/each}
      </div>
    {/if}
    </div>
    <div class="w-[400px] h-full flex flex-col">
	    <div class="flex-1 overflow-y-scroll scrollbar-hide">
	    {#if trace}
		  <EventInfo trace={trace} bind:ptr={event_ptr} />
	    {:else}
		  <div class="p-6 space-y-4">
		    <label for="file-upload" class="inline-block px-5 py-2.5 bg-orange-600 text-white rounded-md cursor-pointer font-semibold hover:bg-orange-700 transition-colors focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2">
		      {fileName ? 'Change File' : 'Select Local File'}
		    </label>
		    <input 
		      id="file-upload" 
		      type="file" 
		      onchange={handleFileChange}
		      class="hidden"
		    />

		    {#if fileName}
		      <p class="mt-2.5 text-gray-600">Selected: <strong class="font-semibold text-gray-900">{fileName}</strong></p>
		    {/if}

		    {#if error}
		      <p class="mt-2.5 text-red-600 font-medium">{error}</p>
		    {/if}
		  </div>
	    {/if}
	    </div>
	    
    </div>
  </div>
  
  {#if trace && vcdPanelOpen}
    <!-- Resizable drag handle -->
    <div 
      class="h-2 bg-gray-300 hover:bg-gray-400 cursor-ns-resize flex items-center justify-center transition-colors"
      onmousedown={handleMouseDown}
      style="height: 8px;"
    >
      <div class="w-12 h-1 bg-gray-500 rounded"></div>
    </div>
    
    <!-- VCD Panel -->
    <div 
      class="border-t-2 border-gray-800 bg-white"
      style="height: {vcdPanelHeight}px;"
    >
      <VCDViewer {trace} bind:ptr={event_ptr} onNavigateToSignal={navigateToSignalLocation} />
    </div>
  {/if}
  
  <!-- Toggle button -->
  {#if trace}
    <button
      onclick={() => vcdPanelOpen = !vcdPanelOpen}
      class="fixed bottom-4 right-4 px-4 py-2 bg-blue-600 text-white rounded-md font-medium hover:bg-blue-700 transition-colors shadow-lg z-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
    >
      {vcdPanelOpen ? '▼ Hide VCD' : '▲ Show VCD'}
    </button>
  {/if}
</main>

<style>
  :global(.scrollbar-hide) {
    /* Hide scrollbar for Chrome, Safari and Opera */
    -ms-overflow-style: none;  /* IE and Edge */
    scrollbar-width: none;     /* Firefox */
  }
  :global(.scrollbar-hide::-webkit-scrollbar) {
    display: none;
  }
  
  :global([data-highlighted-background]) {
    --highlighted-background: #88CCCC;
  }
</style>
