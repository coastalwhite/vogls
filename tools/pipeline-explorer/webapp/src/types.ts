/** Mirrors `DataPoints` in the Rust: which data a run should collect. */
export const DATA_POINTS = {
	pipeline: 1 << 0,
	registers: 1 << 1,
	symbols: 1 << 2,
} as const;

export const ALL_DATA_POINTS =
	DATA_POINTS.pipeline | DATA_POINTS.registers | DATA_POINTS.symbols;

export type PipelineData = {
	kind: "pipeline";
	traces: Uint32Array[];
	keys: string[];
	cycles: number;
	/** Address of slot 1, so a slot maps back to a symbol's address. */
	textAddr: number;
};

export type RegistersData = {
	kind: "registers";
	/** One list per register, one entry per cycle. */
	values: Uint32Array[];
	names: string[];
	cycles: number;
};

/** Where the assembler put every label. Parallel arrays, sorted by address. */
export type SymbolsData = {
	kind: "symbols";
	names: string[];
	addresses: Uint32Array;
	sections: string[];
	/** 1 when a `.global` directive exported the label. */
	globals: Uint32Array;
};

export type ReturnData = PipelineData | RegistersData | SymbolsData;

export type Trace = {
	instructions: string[];
	/** Only what the run collected: a requested point may be missing. */
	data: ReturnData[];
};

/** The requested data point, or `null` when the run did not produce it. */
export function dataOf<K extends ReturnData["kind"]>(
	trace: Trace | null,
	kind: K,
): Extract<ReturnData, { kind: K }> | null {
	if (trace === null) return null;
	for (const entry of trace.data) {
		if (entry.kind === kind) return entry as Extract<ReturnData, { kind: K }>;
	}
	return null;
}

/** Stands in for absent pipeline data, so drawing code renders nothing. */
export const EMPTY_PIPELINE: PipelineData = {
	kind: "pipeline",
	traces: [],
	keys: [],
	cycles: 0,
	textAddr: 0,
};
