<script lang="ts">
	import display_bits from './lib/bits.ts'

	let { ptr = $bindable(), trace, onNavigateToSignal } = $props();
	let searchQuery = $state("");
	let selectedSignals = $state<Set<number>>(new Set());
	let zoomLevel = $state(1); // pixels per time unit
	let scrollX = $state(0);
	let containerElement: HTMLElement | null = $state(null);
	let initialZoomSet = $state(false);
	
	// Build event index to timestamp mapping
	let eventToTime = $derived.by(() => {
		let mapping: number[] = [];
		let currentTime = 0;
		for (let i = 0; i < trace.events.length; i += 1) {
			const e = trace.events[i];
			if (e.type === "time") {
				currentTime = e.time;
			}
			mapping.push(currentTime);
		}
		return mapping;
	});

	// Find max time for X-axis range
	let maxTime = $derived(Math.max(...eventToTime, 0));
	
	// Calculate time step for grid lines
	let timeStep = $derived(Math.max(1, Math.floor(maxTime / 20)));
	
	// Current time for the event pointer
	let currentTime = $derived(eventToTime[ptr]);

	// Track signal values over time
	let signal_values = $derived.by(() => {
		let values = [];
		let i, j, drive;
		for (i = 0; i < trace.signals.length; i += 1) {
			const s = trace.signals[i];
			values.push([{ time: 0, bits: s.initial }]);
		}

		for (i = 0; i < trace.events.length; i += 1) {
			const e = trace.events[i];
			const time = eventToTime[i];
			switch (e.type) {
				case "eval": {
					for (j = e.driven[0]; j < e.driven[1]; j += 1) {
						const d = trace.driven[j];
						if (d.value !== null) {
							values[d.signal].push({ time, bits: d.value });
						}
					}
					break;
				};
				case "drive": {
					if (e.drive !== null) {
						const d = trace.driven[e.drive];
						if (d.value !== null) {
							values[d.signal].push({ time, bits: d.value });
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

	// Create array of selected signals with their row indices
	let selectedSignalsWithRows = $derived(
		Array.from(selectedSignals)
			.sort((a, b) => a - b)
			.map((signalIdx, rowIdx) => ({ signalIdx, rowIdx }))
	);

	function get_signal_value(signal: number, ptr: number): { size: number, slice: Uint8Array } {
		let values = signal_values[signal];
		const time = eventToTime[ptr];
		// Binary search would be better, but linear for now
		let i;
		for (i = 0; i < values.length; i += 1) {
			if (values[i].time > time) {
				return values[i - 1].bits;
			}
		}
		return values[values.length - 1].bits;
	}

	function toggleSignal(signalIdx: number) {
		if (selectedSignals.has(signalIdx)) {
			selectedSignals.delete(signalIdx);
		} else {
			selectedSignals.add(signalIdx);
		}
		selectedSignals = new Set(selectedSignals); // Trigger reactivity
	}

	function handleWaveformClick(event: MouseEvent, container: HTMLElement) {
		const rect = container.getBoundingClientRect();
		const x = event.clientX - rect.left + scrollX - signalNameWidth;
		const clickedTime = x / zoomLevel;
		
		// Find the event index closest to this time
		let closestIdx = 0;
		let minDiff = Math.abs(eventToTime[0] - clickedTime);
		for (let i = 1; i < trace.events.length; i += 1) {
			const diff = Math.abs(eventToTime[i] - clickedTime);
			if (diff < minDiff) {
				minDiff = diff;
				closestIdx = i;
			}
		}
		ptr = closestIdx;
	}

	function scrollToPointer() {
		const container = document.querySelector('.waveform-container') as HTMLElement;
		if (container) {
			const currentTime = eventToTime[ptr];
			const pointerX = currentTime * zoomLevel + signalNameWidth;
			const centerX = pointerX - container.clientWidth / 2;
			container.scrollLeft = Math.max(0, centerX);
		}
	}

	function zoomIn() {
		zoomLevel = Math.min(zoomLevel * 1.5, 100);
		// Center scroll on current pointer position after zoom
		setTimeout(scrollToPointer, 0);
	}

	function zoomOut() {
		zoomLevel = Math.max(zoomLevel / 1.5, 1);
		// Center scroll on current pointer position after zoom
		setTimeout(scrollToPointer, 0);
	}

	// Auto-scroll to keep pointer visible when it changes
	$effect(() => {
		const container = document.querySelector('.waveform-container') as HTMLElement;
		if (container) {
			const currentTime = eventToTime[ptr];
			const pointerX = currentTime * zoomLevel + signalNameWidth;
			const containerLeft = container.scrollLeft;
			const containerRight = containerLeft + container.clientWidth;
			
			if (pointerX < containerLeft || pointerX > containerRight) {
				container.scrollLeft = Math.max(0, pointerX - container.clientWidth / 2);
			}
		}
	});

	// Auto-calculate initial zoom level to fit ~80% of screen
	$effect(() => {
		if (maxTime > 0 && containerElement && !initialZoomSet) {
			const containerWidth = containerElement.clientWidth;
			if (containerWidth > 0) {
				// Calculate zoom to make waveform take up 80% of available width
				// Available width = containerWidth - signalNameWidth (for signal names)
				const availableWidth = containerWidth - signalNameWidth;
				const targetWidth = availableWidth * 0.8;
				const calculatedZoom = targetWidth / maxTime;
				zoomLevel = Math.max(0.1, Math.min(calculatedZoom, 100));
				initialZoomSet = true;
			}
		}
	});

	const waveformHeight = 40;
	const signalNameWidth = 200;
</script>

<div class="h-full flex flex-col bg-white">
	<!-- Header with controls -->
	<div class="flex items-center justify-between p-3 border-b border-gray-300 bg-gray-50">
		<div class="flex items-center gap-4">
			<h3 class="text-lg font-semibold text-gray-800">VCD Waveform Viewer</h3>
			<span class="text-sm text-gray-600">{selectedSignals.size} of {trace.signals.length} signals</span>
		</div>
		<div class="flex items-center gap-2">
			<button
				onclick={zoomOut}
				class="px-4 py-2 bg-blue-600 text-white rounded-md font-medium hover:bg-blue-700 active:bg-blue-800 transition-colors shadow-sm hover:shadow focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 flex items-center gap-2"
				title="Zoom Out"
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM13 10H7" />
				</svg>
				<span>Zoom Out</span>
			</button>
			<button
				onclick={zoomIn}
				class="px-4 py-2 bg-blue-600 text-white rounded-md font-medium hover:bg-blue-700 active:bg-blue-800 transition-colors shadow-sm hover:shadow focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 flex items-center gap-2"
				title="Zoom In"
			>
				<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0zM10 7v6m3-3H7" />
				</svg>
				<span>Zoom In</span>
			</button>
		</div>
	</div>

	<div class="flex-1 flex overflow-hidden">
		<!-- Signal selection sidebar -->
		<div class="w-[250px] border-r border-gray-300 flex flex-col">
			<div class="p-3 border-b border-gray-300">
				<input 
					type="text" 
					bind:value={searchQuery}
					placeholder="Search signals..."
					class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent text-sm"
				/>
			</div>
			<div class="flex-1 overflow-y-auto scrollbar-hide">
				<ul class="space-y-0">
					{#each filteredSignals as [s, i]}
						{@const currentValue = get_signal_value(i, ptr)}
						<li 
							class="flex items-center justify-between border-b border-gray-200 px-3 py-2 transition-colors cursor-pointer"
							class:bg-blue-50={selectedSignals.has(i)}
							class:hover:bg-blue-100={!selectedSignals.has(i)}
							onclick={() => toggleSignal(i)}
						>
							<div class="flex items-center gap-2 flex-1 min-w-0">
								<div class="flex-1 min-w-0">
									<div class="text-sm font-medium text-gray-800 truncate">{s.name}</div>
									<div class="text-xs font-mono text-gray-500 truncate">{display_bits(currentValue.size, currentValue.slice)}</div>
								</div>
							</div>
							{#if s.span && s.span.file !== undefined && onNavigateToSignal}
								<button
									onclick={(e) => {
										e.stopPropagation();
										onNavigateToSignal(i);
									}}
									class="ml-2 p-1.5 text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors flex-shrink-0"
									title="Locate signal in source code"
								>
									<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
									</svg>
								</button>
							{/if}
						</li>
					{/each}
				</ul>
			</div>
		</div>

		<!-- Waveform display area -->
		<div class="flex-1 flex flex-col overflow-hidden" bind:this={containerElement}>
			<!-- Timeline header -->
			<div 
				class="h-8 border-b border-gray-300 bg-gray-50 overflow-x-auto scrollbar-hide timeline-header"
				onscroll={(e) => {
					const waveformContainer = e.currentTarget.parentElement?.querySelector('.waveform-container');
					if (waveformContainer) {
						waveformContainer.scrollLeft = e.currentTarget.scrollLeft;
					}
				}}
				style="scrollbar-width: thin;"
			>
				<svg 
					class="block"
					width={maxTime * zoomLevel + signalNameWidth}
					height="32"
					style="min-width: 100%;"
				>
					<!-- Signal name column background -->
					<rect 
						x="0" 
						y="0" 
						width={signalNameWidth} 
						height="32" 
						fill="#f9fafb"
					/>
					<!-- Grid lines at time intervals -->
					{#each Array(Math.ceil(maxTime / timeStep) + 1) as _, i}
						{@const time = i * timeStep}
						{#if time <= maxTime}
							<line 
								x1={time * zoomLevel + signalNameWidth} 
								y1="0" 
								x2={time * zoomLevel + signalNameWidth} 
								y2="32" 
								stroke="#e5e7eb" 
								stroke-width="1"
							/>
							<text 
								x={time * zoomLevel + signalNameWidth + 4} 
								y="20" 
								fill="#6b7280" 
								font-size="10"
							>
								{time}
							</text>
						{/if}
					{/each}
					<!-- Current event pointer line -->
					<line 
						x1={currentTime * zoomLevel + signalNameWidth} 
						y1="0" 
						x2={currentTime * zoomLevel + signalNameWidth} 
						y2="32" 
						stroke="#3b82f6" 
						stroke-width="2"
					/>
					<!-- Current time label -->
					<rect 
						x={currentTime * zoomLevel + signalNameWidth - 20} 
						y="0" 
						width="40" 
						height="16" 
						fill="#3b82f6" 
						rx="2"
					/>
					<text 
						x={currentTime * zoomLevel + signalNameWidth} 
						y="12" 
						fill="white" 
						font-size="10"
						font-weight="bold"
						text-anchor="middle"
					>
						{currentTime}
					</text>
				</svg>
			</div>

			<!-- Waveforms -->
			<div 
				class="flex-1 overflow-auto scrollbar-hide waveform-container"
				onscroll={(e) => {
					scrollX = (e.target as HTMLElement).scrollLeft;
					const timelineHeader = e.currentTarget.parentElement?.querySelector('.timeline-header');
					if (timelineHeader) {
						timelineHeader.scrollLeft = (e.target as HTMLElement).scrollLeft;
					}
				}}
				style="scrollbar-width: thin;"
			>
				<div 
					class="relative"
					onclick={(e) => handleWaveformClick(e, e.currentTarget)}
					style="width: {maxTime * zoomLevel + signalNameWidth}px; min-width: 100%;"
				>
					<svg 
						class="block"
						width={maxTime * zoomLevel + signalNameWidth}
						height={selectedSignals.size * waveformHeight}
						style="min-width: 100%;"
					>
						{#each selectedSignalsWithRows as { signalIdx, rowIdx } (signalIdx)}
							{@const signal = trace.signals[signalIdx]}
							{@const values = signal_values[signalIdx]}
							
							<!-- Signal name background -->
							<rect 
								x="0" 
								y={rowIdx * waveformHeight} 
								width={signalNameWidth} 
								height={waveformHeight} 
								fill={rowIdx % 2 === 0 ? "#f9fafb" : "#ffffff"}
							/>
							
							<!-- Signal name text -->
							<text 
								x="8" 
								y={rowIdx * waveformHeight + 24} 
								fill="#1f2937" 
								font-size="12"
								font-weight="500"
							>
								{signal.name}
							</text>

							<!-- Waveform -->
							{#if values.length > 0}
								{#each values as value, valueIdx}
									{@const startX = value.time * zoomLevel + signalNameWidth}
									{@const nextTime = valueIdx < values.length - 1 ? values[valueIdx + 1].time : maxTime}
									{@const endX = valueIdx < values.length - 1 ? (nextTime * zoomLevel + signalNameWidth) : (maxTime * zoomLevel + signalNameWidth + Math.max(1, zoomLevel * 0.1))}
								{@const isBinary = value.bits.size === 1}
								{@const yHigh = rowIdx * waveformHeight + 12}
								{@const yLow = rowIdx * waveformHeight + waveformHeight - 12}
								{@const yMid = rowIdx * waveformHeight + waveformHeight / 2}
								
								{#if isBinary}
									<!-- Binary signal: show high/low transitions -->
									{@const bitValue = (value.bits.slice[0] & 1) !== 0}
									{@const y = bitValue ? yHigh : yLow}
									
									<!-- Horizontal line for this value -->
									<line 
										x1={startX} 
										y1={y} 
										x2={endX} 
										y2={y} 
										stroke="#3b82f6" 
										stroke-width="2"
									/>
									
									<!-- Vertical transition line -->
									{#if valueIdx > 0}
										{@const prevValue = values[valueIdx - 1]}
										{@const prevBitValue = (prevValue.bits.slice[0] & 1) !== 0}
										{@const prevY = prevBitValue ? yHigh : yLow}
										{#if prevBitValue !== bitValue}
											<line 
												x1={startX} 
												y1={prevY} 
												x2={startX} 
												y2={y} 
												stroke="#3b82f6" 
												stroke-width="2"
											/>
										{/if}
									{/if}
								{:else}
									<!-- Multi-bit signal: show as horizontal line with value -->
									<line 
										x1={startX} 
										y1={yMid} 
										x2={endX} 
										y2={yMid} 
										stroke="#3b82f6" 
										stroke-width="2"
									/>
									
									<!-- Vertical transition line -->
									{#if valueIdx > 0}
										<line 
											x1={startX} 
											y1={(rowIdx * waveformHeight) + 10} 
											x2={startX} 
											y2={(rowIdx * waveformHeight) + waveformHeight - 10} 
											stroke="#3b82f6" 
											stroke-width="1"
										/>
									{/if}

									<!-- Value label for multi-bit signals -->
									{#if endX - startX > 80}
										<text 
											x={startX + 4} 
											y={yMid - 4} 
											fill="#6b7280" 
											font-size="10"
											font-family="monospace"
										>
											{display_bits(value.bits.size, value.bits.slice)}
										</text>
									{/if}
								{/if}
								{/each}
							{/if}
						{/each}

						<!-- Current event pointer line across all waveforms -->
						<line 
							x1={eventToTime[ptr] * zoomLevel + signalNameWidth} 
							y1="0" 
							x2={eventToTime[ptr] * zoomLevel + signalNameWidth} 
							y2={selectedSignals.size * waveformHeight} 
							stroke="#ef4444" 
							stroke-width="2"
							stroke-dasharray="4,4"
						/>
					</svg>
				</div>
			</div>
		</div>
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

