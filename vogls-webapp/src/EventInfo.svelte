<script lang="ts">
	import display_bits from './lib/bits.ts'

	let { ptr = $bindable(), trace } = $props();
	let cells = $derived(document.querySelectorAll('.cell'))
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

	  cells[ptr].scrollIntoView({ block: 'center' });
	  const xs = cells[ptr].querySelector('.title')
	  if (xs !== null) {
		  xs.focus();
	  }
	}
	function toggle_cells_open(p: number) {
		if (p in cells_open) {
			delete cells_open[p];
		} else {
			cells_open[p] = true;
		}
	}

</script>

<svelte:window onkeydown={handleKeydown} />
<div>
    <p>{trace.events.length} Events</p>
    {#each trace.events as e, ei}
	<a class="cell" class:cell-focus={ei == ptr} onclick={() => ptr = ei}>
		<div class="title">
			<div class="info-button">
				<button onclick={() => toggle_cells_open(ei)}>I</button>
			</div>
			<div class="name">
			{#if e.type == "eval"}
				<i>{trace.processes[e.process].name}</i> ({e.process})
			{:else if e.type == "drive"}
				Drive <i>{trace.signals[e.signal].name}</i><br/>
			{:else if e.type == "time"}
				Timestep <i>{e.time}</i><br/>
			{/if}
			</div>
			<div class="stats">
			{#if e.type == "eval"}
				D {num_driven(e)} W {num_woken_up(e)}
			{:else if e.type == "drive"}
				W {num_woken_up(e)}
			{:else if e.type == "time"}
			{/if}
			</div>
		</div>

		<div class="details" class:details-hidden={!(ei in cells_open)}>
			{#if e.type == "eval"}
			Driven:
			<ul>
			{#each [...Array(e.driven[1] - e.driven[0])] as _, i}
			{@const drive = trace.driven[e.driven[0] + i]}
			<li>{trace.signals[drive.signal].name} = {drive.value.slice.length} {display_bits(drive.value.size, drive.value.slice)}</li>
				<ul>
				{#each [...Array(drive.woken_range[1] - drive.woken_range[0])] as _, j}
				{@const woke = trace.woken[drive.woken_range[0] + j]}
				{@const process = trace.processes[woke]}
				<li><i>{process.name}</i> ({woke})</li>
				{/each}
				</ul>
			{/each}
			</ul>
			Stop-Reason: {e.stop_reason.type} <br/>
			{:else if e.type == "drive"}
				{#if e.drive}
				{@const drive = trace.driven[e.drive]}
				<li>{trace.signals[drive.signal].name} = {drive.value.slice.length} {display_bits(drive.value.size, drive.value.slice)}</li>
					<ul>
					{#each [...Array(drive.woken_range[1] - drive.woken_range[0])] as _, j}
					{@const woke = trace.woken[drive.woken_range[0] + j]}
					{@const process = trace.processes[woke]}
					<li><i>{process.name}</i> ({woke})</li>
					{/each}
					</ul>
				{/if}
			{:else if e.type == "time"}
				Timestep <i>{e.time}</i><br/>
			{/if}
		</div>
	</a>
    {/each}
</div>

<style>
.cell {
	display: block;
    text-align: left;
	color: black;
    border: 1px solid #000;
    border-left: none;
    border-right: none;
    padding: 4px;
    
}
.cell-focus {
    background-color: #88CCCC;
}
.title {
    display: flex;
    flex-direction: horizontal;
}
.info-button {
	margin: 0px 4px 0px 4px;
}
.name {
    flex-grow: 1;
}
.details-hidden {
	display: none;
}
</style>
