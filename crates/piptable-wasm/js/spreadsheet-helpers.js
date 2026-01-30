/**
 * JavaScript helpers for TOON/JSON switching based on payload size and debug mode
 * 
 * Automatically selects the most efficient format for WASM boundary crossing
 */

import { compile_many, eval_many, apply_range, validate_formula } from "../pkg/piptable_wasm.js";
import msgpack from "@msgpack/msgpack"; // For TOON encoding (MessagePack)

// Threshold for switching to TOON (4KB)
const TOON_THRESHOLD = 4 * 1024;

/**
 * Determine whether to use TOON based on payload size and debug flag
 */
function shouldUseToon(payload, debug = false) {
  if (debug) return false; // Always use JSON in debug mode
  
  const jsonSize = JSON.stringify(payload).length;
  return jsonSize > TOON_THRESHOLD;
}

/**
 * Encode payload to bytes (TOON or JSON)
 */
function encodePayload(payload, useToon) {
  if (useToon) {
    // Use MessagePack for TOON encoding
    return msgpack.encode(payload);
  } else {
    // Use JSON for debugging/small payloads
    const jsonStr = JSON.stringify(payload);
    return new TextEncoder().encode(jsonStr);
  }
}

/**
 * Decode response bytes (TOON or JSON)
 */
function decodeResponse(bytes, useToon) {
  if (useToon) {
    // Decode MessagePack
    return msgpack.decode(bytes);
  } else {
    // Decode JSON
    const jsonStr = new TextDecoder().decode(bytes);
    return JSON.parse(jsonStr);
  }
}

/**
 * Compile multiple formulas with automatic format selection
 * 
 * @param {CompileRequest} request - Formulas to compile
 * @param {boolean} debug - Force JSON mode for debugging
 * @returns {Promise<CompileResponse>}
 */
export async function wasmCompileMany(request, debug = false) {
  const useToon = shouldUseToon(request, debug);
  
  if (debug) {
    console.log("Compile request:", request);
    console.log("Using format:", useToon ? "TOON" : "JSON");
  }
  
  const bytes = encodePayload(request, useToon);
  const responseBytes = compile_many(bytes);
  const response = decodeResponse(responseBytes, useToon);
  
  if (debug) {
    console.log("Compile response:", response);
  }
  
  return response;
}

/**
 * Evaluate multiple formulas with automatic format selection
 * 
 * @param {EvalRequest} request - Compiled formulas and sheet data
 * @param {boolean} debug - Force JSON mode for debugging
 * @returns {Promise<EvalResponse>}
 */
export async function wasmEvalMany(request, debug = false) {
  const useToon = shouldUseToon(request, debug);
  
  if (debug) {
    console.log("Eval request:", request);
    console.log("Using format:", useToon ? "TOON" : "JSON");
    if (request.sheet.values) {
      console.log("Sheet size:", request.sheet.values.length, "cells");
    } else if (request.sheet.items) {
      console.log("Sheet sparse items:", request.sheet.items.length);
    }
  }
  
  const bytes = encodePayload(request, useToon);
  const responseBytes = eval_many(bytes);
  const response = decodeResponse(responseBytes, useToon);
  
  if (debug) {
    console.log("Eval response:", response);
  }
  
  return response;
}

/**
 * Apply updates to a sheet range
 * 
 * @param {RangeUpdateRequest} request - Sheet and updates
 * @param {boolean} debug - Force JSON mode for debugging
 * @returns {Promise<RangeUpdateResponse>}
 */
export async function wasmApplyRange(request, debug = false) {
  const useToon = shouldUseToon(request, debug);
  
  if (debug) {
    console.log("Update request:", request);
    console.log("Using format:", useToon ? "TOON" : "JSON");
    console.log("Updates:", request.updates.length);
  }
  
  const bytes = encodePayload(request, useToon);
  const responseBytes = apply_range(bytes);
  const response = decodeResponse(responseBytes, useToon);
  
  if (debug) {
    console.log("Update response:", response);
  }
  
  return response;
}

/**
 * Validate a formula for syntax highlighting
 * 
 * @param {string} formula - Formula text to validate
 * @param {boolean} debug - Force JSON mode for debugging
 * @returns {Promise<{valid: boolean, msg: string}>}
 */
export async function wasmValidateFormula(formula, debug = false) {
  const request = {
    kind: "text",
    f: formula
  };
  
  // Validation is typically small, always use JSON
  const bytes = encodePayload(request, false);
  const responseBytes = validate_formula(bytes);
  const response = decodeResponse(responseBytes, false);
  
  if (debug) {
    console.log("Validate:", formula, "->", response);
  }
  
  return response;
}

/**
 * Helper to create a sheet payload from a 2D array
 * 
 * @param {Array<Array<any>>} data - 2D array of cell values
 * @param {number} startRow - Starting row (0-based)
 * @param {number} startCol - Starting column (0-based)
 * @returns {SheetPayload}
 */
export function createSheetPayload(data, startRow = 0, startCol = 0) {
  return createSheetPayloadWithOptions(data, startRow, startCol, { autoSparse: false });
}

/**
 * Helper to create a sheet payload with sparse/dense options
 *
 * @param {Array<Array<any>>} data - 2D array of cell values
 * @param {number} startRow - Starting row (0-based)
 * @param {number} startCol - Starting column (0-based)
 * @param {{ sparse?: boolean, autoSparse?: boolean }} options
 * @returns {SheetPayload}
 */
export function createSheetPayloadWithOptions(data, startRow = 0, startCol = 0, options = {}) {
  const rows = data.length;
  const cols = Math.max(0, ...data.map((row) => (row ? row.length : 0)));

  if (options.sparse || (options.autoSparse && shouldUseSparse(data))) {
    const items = [];
    for (let r = 0; r < rows; r++) {
      const rowData = data[r] || [];
      for (let c = 0; c < cols; c++) {
        const cellValue = rowData[c];
        if (cellValue !== null && cellValue !== undefined && cellValue !== "") {
          items.push({
            r: startRow + r,
            c: startCol + c,
            v: convertToToonValue(cellValue),
          });
        }
      }
    }
    return {
      range: {
        s: { r: startRow, c: startCol },
        e: { r: startRow + rows - 1, c: startCol + cols - 1 },
      },
      items,
    };
  }
  
  const values = [];
  for (let r = 0; r < rows; r++) {
    const rowData = data[r] || [];
    for (let c = 0; c < cols; c++) {
      const cellValue = rowData[c];
      values.push(convertToToonValue(cellValue));
    }
  }
  
  return {
    range: {
      s: { r: startRow, c: startCol },
      e: { r: startRow + rows - 1, c: startCol + cols - 1 }
    },
    values
  };
}

function shouldUseSparse(data) {
  const rows = data.length;
  const cols = data[0]?.length || 0;
  const total = rows * cols;
  if (total === 0) return false;

  let nonEmpty = 0;
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const cellValue = data[r][c];
      if (cellValue !== null && cellValue !== undefined && cellValue !== "") {
        nonEmpty += 1;
      }
    }
  }

  const density = nonEmpty / total;
  return density < 0.2 || (total > 10000 && density < 0.5);
}

/**
 * Convert JavaScript value to TOON value
 */
function convertToToonValue(value) {
  if (value === null || value === undefined || value === "") {
    return { t: "null" };
  }
  if (typeof value === "boolean") {
    return { t: "bool", v: value ? 1 : 0 };
  }
  if (typeof value === "number") {
    if (Number.isInteger(value)) {
      return { t: "int", v: value };
    }
    return { t: "float", v: value };
  }
  if (typeof value === "string") {
    return { t: "str", v: value };
  }
  if (Array.isArray(value)) {
    return { t: "arr", v: value.map(convertToToonValue) };
  }
  if (value instanceof Date) {
    return { t: "date", v: value.getTime() };
  }
  if (typeof value === "object") {
    const obj = {};
    for (const [k, v] of Object.entries(value)) {
      obj[k] = convertToToonValue(v);
    }
    return { t: "obj", v: obj };
  }
  // Default to string
  return { t: "str", v: String(value) };
}

/**
 * Convert TOON value to JavaScript value
 */
export function convertFromToonValue(toonValue) {
  switch (toonValue.t) {
    case "null":
      return null;
    case "bool":
      return toonValue.v !== 0;
    case "int":
    case "float":
      return toonValue.v;
    case "str":
      return toonValue.v;
    case "arr":
      return toonValue.v.map(convertFromToonValue);
    case "date":
      return new Date(toonValue.v);
    case "duration":
      return toonValue.v;
    case "error":
      return new Error(`${toonValue.code}: ${toonValue.msg}`);
    case "obj":
      const obj = {};
      for (const [k, v] of Object.entries(toonValue.v)) {
        obj[k] = convertFromToonValue(v);
      }
      return obj;
    default:
      return toonValue;
  }
}

// Example usage:
/*
// Compile formulas
const compileReq = {
  formulas: [
    { kind: "text", f: "=A1+B1" },
    { kind: "text", f: "=SUM(A:A)" }
  ]
};
const compiled = await wasmCompileMany(compileReq);

// Evaluate with sheet data
const evalReq = {
  compiled: compiled.compiled,
  sheet: createSheetPayload([
    [1, 2, 3],
    [4, 5, 6]
  ])
};
const results = await wasmEvalMany(evalReq);
*/
