<script lang="ts">
	import display_bits from './lib/bits.ts'

	let { ptr, trace } = $props();
	let searchQuery = $state("");
	let signal_values = $derived.by(() => {
		let values = [];
		let i, j, drive;
		for (i = 0; i < trace.signals.length; i += 1) {
			const s = trace.signals[i];
			if (s.name == "mem_rdata") {
				console.log(s.initial);
			}
			values.push([{ ptr: 0, bits: s.initial }]);
		}

		for (i = 0; i < trace.events.length; i += 1) {
			const e = trace.events[i];
			switch (e.type) {
				case "eval": {
					for (j = e.driven[0]; j < e.driven[1]; j += 1) {
						const d = trace.driven[j];
						if (d.value !== null) {
							values[d.signal].push({ ptr: i, bits: d.value });
						}
					}
					break;
				};
				case "drive": {
					if (e.drive !== null) {
						const d = trace.driven[e.drive];
						if (d.value !== null) {
							values[d.signal].push({ ptr: i, bits: d.value });
						}
					}
					break;
				};
				case "time": {
					break;
				};
				default: throw "ERROR";
			}
		}

		return values;
	});

	let filteredSignals = $derived(trace.signals.map((x, i) => [x, i]).filter(([item, _]) => 
	  item.name.toLowerCase().includes(searchQuery.toLowerCase())
	));

	function get_signal_value(signal: number, ptr: number): string {
		let values = signal_values[signal];
		// @TODO: binary search
		let i;
		for (i = 0; i < values.length; i += 1) {
			if (values[i].ptr > ptr) {
				return display_bits(values[i - 1].bits.size, values[i - 1].bits.slice);
			}
		}
		return display_bits(values[values.length - 1].bits.size, values[values.length - 1].bits.slice);
	}
</script>

<div>
    <h3>{trace.signals.length} Signals</h3>
	<input type="text" bind:value={searchQuery} />
	<ul class="items">
    {#each filteredSignals as [s, i]}
		<li>
			<div class="item-name">{i}. {s.name}</div>
			<div class="item-value">{get_signal_value(i, ptr)}</div>

		</li>
	{/each}
	</ul>
</div>

<style>
div {
	height: 100%;
	padding: 4px;
    overflow-y: scroll;

  /* Hide scrollbar for Chrome, Safari and Opera */
  &::-webkit-scrollbar {
	display: none;
  }

  /* Hide scrollbar for IE, Edge and Firefox */
  -ms-overflow-style: none;  /* IE and Edge */
  scrollbar-width: none;     /* Firefox */
}
.items li {
	list-style: none;
	border-top: 1px solid #000;
	display: flex;
}
.item-name {
	flex-grow: 1;
}
</style>
