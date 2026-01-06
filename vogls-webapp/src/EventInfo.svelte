<script lang="ts">
	import display_bits from './lib/bits.ts'

	let { ptr = $bindable(), trace } = $props();
	let cells = $derived(document.querySelectorAll('[data-event-cell]'))
	let cells_open = $state({});

    function num_driven(e): int {
        return e.driven[1] - e.driven[0];
    }
    function num_woken_up(e): int {
        let num_woken = 0
		if (e.type == "eval") {
			let i;
			let d;
			for (i = e.driven[0]; i < e.driven[1]; i += 1) {
				d = trace.driven[i];
				num_woken += d.woken_range[1] - d.woken_range[0];
			}
		} else if (e.type == "drive") {
			if (e.drive !== null) {
				let d = trace.driven[e.drive];
				num_woken += d.woken_range[1] - d.woken_range[0];
			}
		}
        return num_woken;
    }

  	function handleKeydown(event) {
	  if (event.key === 'ArrowRight') {
	  	if (ptr + 1 >= trace.events.length) {
	  		return;
	  	}
	  	ptr += 1;
	  } else if (event.key === 'ArrowLeft') {
	  	if (ptr == 0) {
	  		return;
	  	}
	  	ptr -= 1;
	  } else if (event.key === 'Enter') {
		toggle_cells_open(ptr);
	  }

	  if (cells[ptr]) {
		  cells[ptr].scrollIntoView({ block: 'center' });
		  const xs = cells[ptr].querySelector('[data-event-title]')
		  if (xs !== null) {
			  xs.focus();
		  }
	  }
	}
	function toggle_cells_open(p: number) {
		if (p in cells_open) {
			delete cells_open[p];
		} else {
			cells_open[p] = true;
		}
	}

	// Auto-scroll to current event when ptr changes (e.g., from VCD viewer click)
	$effect(() => {
		if (cells[ptr]) {
			cells[ptr].scrollIntoView({ block: 'center', behavior: 'smooth' });
		}
	});

</script>

<svelte:window onkeydown={handleKeydown} />
<div class="h-full overflow-y-scroll scrollbar-hide">
    <div class="sticky top-0 bg-white z-50 pb-3 pt-2 px-2 mb-3 border-b border-gray-300 shadow-sm">
      <div class="flex items-center justify-between mb-2">
        <p class="text-sm font-semibold text-gray-700">Events</p>
        <p class="text-sm text-gray-600">Event {ptr + 1} of {trace.events.length}</p>
      </div>
      <div class="flex gap-2">
        <button
          onclick={() => {
            if (ptr > 0) {
              ptr -= 1;
              if (cells[ptr]) {
                cells[ptr].scrollIntoView({ block: 'center' });
                const xs = cells[ptr].querySelector('[data-event-title]');
                if (xs !== null) {
                  xs.focus();
                }
              }
            }
          }}
          disabled={ptr === 0}
          class="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md font-medium hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed disabled:text-gray-500 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
        >
          ← Previous
        </button>
        <button
          onclick={() => {
              if (ptr + 1 < trace.events.length) {
              ptr += 1;
              if (cells[ptr]) {
                cells[ptr].scrollIntoView({ block: 'center' });
                const xs = cells[ptr].querySelector('[data-event-title]');
                if (xs !== null) {
                  xs.focus();
                }
              }
            }
          }}
          disabled={ptr + 1 >= trace.events.length}
          class="flex-1 px-4 py-2 bg-blue-600 text-white rounded-md font-medium hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed disabled:text-gray-500 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
        >
          Next →
        </button>
      </div>
    </div>
    <div class="px-2">
    {#each trace.events as e, ei}
	<a 
		data-event-cell
		class="block text-left text-black border-y border-gray-300 px-3 py-2 cursor-pointer transition-colors" 
		class:bg-cyan-200={ei == ptr}
		class:hover:bg-blue-100={ei != ptr}
		onclick={() => ptr = ei}
	>
		<div data-event-title class="flex items-center gap-2" onclick={() => ptr = ei}>
			<div class="flex-shrink-0">
				<button 
					onclick={(e) => { e.stopPropagation(); toggle_cells_open(ei); }}
					class="px-2 py-1 text-xs font-semibold bg-gray-200 hover:bg-gray-300 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-gray-400"
				>
					I
				</button>
			</div>
			<div class="flex-grow min-w-0" onclick={() => ptr = ei}>
			{#if e.type == "eval"}
				<span class="italic">{trace.processes[e.process].name}</span> <span class="text-gray-600">({e.process})</span>
			{:else if e.type == "drive"}
				Drive <span class="italic">{trace.signals[e.signal].name}</span>
			{:else if e.type == "time"}
				Timestep <span class="italic">{e.time}</span>
			{/if}
			</div>
			<div class="flex-shrink-0 text-sm text-gray-600" onclick={() => ptr = ei}>
			{#if e.type == "eval"}
				D {num_driven(e)} W {num_woken_up(e)}
			{:else if e.type == "drive"}
				W {num_woken_up(e)}
			{:else if e.type == "time"}
			{/if}
			</div>
		</div>

		<div class="mt-2 pl-6 text-sm" class:hidden={!(ei in cells_open)} onclick={() => ptr = ei}>
			{#if e.type == "eval"}
			<div class="space-y-2">
				<div class="font-semibold">Driven:</div>
				<ul class="list-disc list-inside space-y-1 ml-2">
				{#each [...Array(e.driven[1] - e.driven[0])] as _, i}
				{@const drive = trace.driven[e.driven[0] + i]}
				<li class="text-gray-800">
					<span class="font-medium">{trace.signals[drive.signal].name}</span> = {drive.value.slice.length} {display_bits(drive.value.size, drive.value.slice)}
					<ul class="list-circle list-inside ml-4 mt-1 space-y-0.5">
					{#each [...Array(drive.woken_range[1] - drive.woken_range[0])] as _, j}
					{@const woke = trace.woken[drive.woken_range[0] + j]}
					{@const process = trace.processes[woke]}
					<li class="text-gray-600"><span class="italic">{process.name}</span> ({woke})</li>
					{/each}
					</ul>
				</li>
				{/each}
				</ul>
				<div class="mt-2"><span class="font-semibold">Stop-Reason:</span> {e.stop_reason.type}</div>
			</div>
			{:else if e.type == "drive"}
				{#if e.drive}
				{@const drive = trace.driven[e.drive]}
				<div class="space-y-1">
					<div class="text-gray-800">
						<span class="font-medium">{trace.signals[drive.signal].name}</span> = {drive.value.slice.length} {display_bits(drive.value.size, drive.value.slice)}
					</div>
					<ul class="list-circle list-inside ml-4 space-y-0.5">
					{#each [...Array(drive.woken_range[1] - drive.woken_range[0])] as _, j}
					{@const woke = trace.woken[drive.woken_range[0] + j]}
					{@const process = trace.processes[woke]}
					<li class="text-gray-600"><span class="italic">{process.name}</span> ({woke})</li>
					{/each}
					</ul>
				</div>
				{/if}
			{:else if e.type == "time"}
				<div>Timestep <span class="italic">{e.time}</span></div>
			{/if}
		</div>
	</a>
    {/each}
    </div>
</div>

<style>
  :global(.scrollbar-hide) {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
  :global(.scrollbar-hide::-webkit-scrollbar) {
    display: none;
  }
</style>
