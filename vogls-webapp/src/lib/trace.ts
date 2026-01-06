export type Timestamp = int;
export type FileIdx = int;
export type SignalIdx = int;
export type ProcessIdx = int;
export type DrivenIdx = int;
export type WokenIdx = int;
export type WatchIdx = int;

export type File = {
  name?: string,
  content: string,
};

export type Span = {
  file?: int,
  line_range?: [int, int],
};

export type Process = {
  name?: string,
  span?: Span,
};

export type Bits = {
  size: int,
  slice: Uint8Array,
};

export type Signal = {
  name?: string,
  span?: Span,
  initial: Bits,
};

export type Driven = {
  signal: int,
  value: Bits,
  woken_range: [int, int],
};

export type TEvent = 
  { type: "eval", process: int, driven: [int, int], stop_reason:
    { type: "halt" } | { type: "wait", time: int } | { type: "wait_region", region: int } | { type: "watch_signals", range: [int, int] }
  } |
  { type: "drive", signal: int, drive?: int } |
  { type: "time", time: int }
;

export type Trace = {
  files: File[],
  processes: Process[],
  signals: Signal[],
  driven: Driven[],
  woken: ProcessIdx[],
  watches: SignalIdx[],
  events: TEvent[],
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

export function parseTrace(buffer: Uint8Array): { trace: Trace | null, error: string | null } {
  let error: string | null = null;
  
  try {
    let view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);

    const magic = view.getUint32(0, false)
    if (magic != 0x56_47_54_44) { // VGTD
      return { trace: null, error: "Missing magic bytes. Wrong file type?" };
    }

    let ptr = 4
    const files_len     = Number(view.getBigUint64(ptr, true)); ptr += 8;
    const processes_len = Number(view.getBigUint64(ptr, true)); ptr += 8;
    const signals_len   = Number(view.getBigUint64(ptr, true)); ptr += 8;
    const driven_len    = Number(view.getBigUint64(ptr, true)); ptr += 8;
    const woken_len     = Number(view.getBigUint64(ptr, true)); ptr += 8;
    const watches_len   = Number(view.getBigUint64(ptr, true)); ptr += 8;
    const events_len    = Number(view.getBigUint64(ptr, true)); ptr += 8;

    const trace: Trace = {
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
        let e: TEvent;
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
                default:
                    return { trace: null, error: "Invalid event stop reason discriminant" };
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
              return { trace: null, error: "Invalid event discriminant" };
        }

        trace.events.push(e);
    }
    
    return { trace, error: null };
  } catch (err) {
    return { trace: null, error: "Failed to read file." };
  }
}

