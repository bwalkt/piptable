import init, { 
  PipTableParser, 
  get_examples, 
  get_sample_data 
} from '../wasm/piptable_wasm';

let wasmInitialized = false;
let parser: PipTableParser | null = null;

export async function initializeWasm() {
  if (wasmInitialized) return;
  
  try {
    await init();
    parser = new PipTableParser();
    wasmInitialized = true;
    console.log('WASM initialized successfully');
  } catch (error) {
    console.error('Failed to initialize WASM:', error);
    throw error;
  }
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