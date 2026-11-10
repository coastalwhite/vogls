/** Mirrors `DataPoints` in the Rust: which data a run should collect. */
export const DATA_POINTS = {
	pipeline: 1 << 0,
	registers: 1 << 1,
} as const;

export const ALL_DATA_POINTS = DATA_POINTS.pipeline | DATA_POINTS.registers;

export type PipelineData = {
	kind: "pipeline";
	traces: Uint32Array[];
	keys: string[];
	cycles: number;
};

export type RegistersData = {
	kind: "registers";
	/** One list per register, one entry per cycle. */
	values: Uint32Array[];
	names: string[];
	cycles: number;
};

export type ReturnData = PipelineData | RegistersData;

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
};
