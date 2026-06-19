export type Trace = {
	instructions: string[];
	pipeline: {
		traces: Uint32Array[];
		keys: string[];
		cycles: number;
	};
};
