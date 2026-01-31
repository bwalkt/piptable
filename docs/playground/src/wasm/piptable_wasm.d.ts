/* tslint:disable */
/* eslint-disable */

export class PipTableParser {
    free(): void;
    [Symbol.dispose](): void;
    format(code: string): string;
    constructor();
    parse(code: string): any;
    validate(code: string): any;
}

/**
 * Apply updates to a sheet range
 *
 * Input: TOON-encoded RangeUpdateRequest
 * Output: TOON-encoded RangeUpdateResponse
 */
export function apply_range(toon_bytes: Uint8Array): Uint8Array;

/**
 * Compile multiple formulas in batch
 *
 * Input: TOON-encoded CompileRequest
 * Output: TOON-encoded CompileResponse
 */
export function compile_many(toon_bytes: Uint8Array): Uint8Array;

/**
 * Evaluate multiple compiled formulas in batch
 *
 * Input: TOON-encoded EvalRequest
 * Output: TOON-encoded EvalResponse
 */
export function eval_many(toon_bytes: Uint8Array): Uint8Array;

export function get_examples(): any;

export function get_sample_data(): any;

export function init(): void;

export function run_code(code: string): Promise<any>;

/**
 * Validate a formula for syntax highlighting
 *
 * Input: TOON-encoded FormulaText
 * Output: TOON-encoded validation result
 */
export function validate_formula(toon_bytes: Uint8Array): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_piptableparser_free: (a: number, b: number) => void;
    readonly get_examples: () => [number, number, number];
    readonly get_sample_data: () => [number, number, number];
    readonly init: () => void;
    readonly piptableparser_format: (a: number, b: number, c: number) => [number, number, number, number];
    readonly piptableparser_new: () => number;
    readonly piptableparser_parse: (a: number, b: number, c: number) => [number, number, number];
    readonly piptableparser_validate: (a: number, b: number, c: number) => [number, number, number];
    readonly run_code: (a: number, b: number) => any;
    readonly apply_range: (a: number, b: number) => [number, number, number, number];
    readonly compile_many: (a: number, b: number) => [number, number, number, number];
    readonly eval_many: (a: number, b: number) => [number, number, number, number];
    readonly validate_formula: (a: number, b: number) => [number, number, number, number];
    readonly wasm_bindgen__closure__destroy__h4db54c25ba8bc3fb: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hee7af0eb7b1ff63f: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h236b223be0ae7735: (a: number, b: number, c: any) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
