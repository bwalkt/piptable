import init, {
  PipTableParser,
  get_examples,
  get_sample_data,
  run_code
} from '../wasm/piptable_wasm';

let wasmInitialized = false;
let parser: PipTableParser | null = null;
let initPromise: Promise<void> | null = null;

export async function initializeWasm() {
  if (wasmInitialized) return;
  if (initPromise) return initPromise;
  
  initPromise = (async () => {
    try {
      await init();
      parser = new PipTableParser();
      wasmInitialized = true;
      console.log('WASM initialized successfully');
    } catch (error) {
      initPromise = null; // Allow retry on failure
      console.error('Failed to initialize WASM:', error);
      throw error;
    }
  })();
  
  return initPromise;
}

export function getParser(): PipTableParser | null {
  return parser;
}

export async function parseCode(code: string): Promise<any> {
  await initializeWasm();
  if (!parser) throw new Error('Parser not initialized');
  
  try {
    const result = parser.parse(code);
    return result;
  } catch (error) {
    console.error('Parse error:', error);
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

export async function validateCode(code: string): Promise<any> {
  await initializeWasm();
  if (!parser) throw new Error('Parser not initialized');
  
  try {
    return parser.validate(code);
  } catch (error) {
    console.error('Validation error:', error);
    return {
      valid: false,
      errors: [{
        line: 1,
        column: 1,
        message: error instanceof Error ? error.message : String(error)
      }]
    };
  }
}

export async function getExamples(): Promise<any> {
  await initializeWasm();
  try {
    return get_examples();
  } catch (error) {
    console.error('Failed to get examples:', error);
    return {};
  }
}

export async function getSampleData(): Promise<any> {
  await initializeWasm();
  try {
    return get_sample_data();
  } catch (error) {
    console.error('Failed to get sample data:', error);
    return { csv: '', json: [] };
  }
}

/**
 * Execute PipTable source code through the WASM-based parser/interpreter, returning either execution results or a structured parse error.
 *
 * The function validates the code with the parser first; if validation fails it returns a parse error object and does not run the interpreter. On success it returns the interpreter's result shape.
 *
 * @param code - PipTable source code to validate and execute
 * @returns An object with the execution outcome:
 * - `success`: `true` if execution succeeded, `false` otherwise.
 * - `output`: an array of output lines (empty on failure).
 * - `result`: the interpreter result on success, or `null` on failure.
 * - `error`: a human-readable error message when `success` is `false`. For validation failures this is formatted as `Parse error at {line}:{column} - {message}`.
 *
 * @throws Error if the internal parser is not initialized
 */
export async function executeCode(code: string): Promise<any> {
  await initializeWasm();
  try {
    if (!parser) throw new Error('Parser not initialized');
    const validation = parser.validate(code) as {
      valid: boolean;
      errors: { line: number; column: number; message: string }[];
    };

    if (validation && validation.valid === false && validation.errors?.length > 0) {
      const { line, column, message } = validation.errors[0];
      return {
        success: false,
        output: [],
        result: null,
        error: `Parse error at ${line}:${column} - ${message}`
      };
    }

    return await run_code(code);
  } catch (error) {
    console.error('Execution error:', error);
    return {
      success: false,
      output: [],
      result: null,
      error: error instanceof Error ? error.message : String(error)
    };
  }
}