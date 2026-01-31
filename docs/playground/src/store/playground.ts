import { signal } from '@preact/signals';
import DOMPurify from 'dompurify';
import { executeCode } from './wasm';
import { loadSharedState, type ShareableState } from '../lib/share';

// Example metadata
export interface Example {
  code: string;
  description: string;
}

// Example code snippets  
export const examples: Record<string, Example> = {
  hello: {
    description: "Simple hello world example",
    code: `' Hello World in PipTable
print("Hello, World!")

dim name = "PipTable"
print("Welcome to " + name + " Playground!")`
  },
  variables: {
    description: "Variable declarations and basic types",
    code: `' Variables and Types
dim message = "Hello"
dim count = 42
dim price = 19.99
dim active = true

' Display values using string conversion
print("Message: " + message)
print("Count: " + str(count))
print("Price: $" + str(price))
print("Active: " + str(active))`
  },
  loops: {
    description: "Loops and control flow",
    code: `' Loops and Control Flow
dim total = 0
for i = 1 to 10
  total = total + i
  print("i = " + str(i) + ", total = " + str(total))
next

' Conditional
dim score = 85
if score >= 90 then
  print("Grade: A")
elseif score >= 80 then
  print("Grade: B")
else
  print("Grade: C")
end if`
  },
  formulas: {
    description: "Formula functions and lookups",
    code: `' Formula Functions
dim total = sum(1, 2, 3)
dim label = if(1, "yes", "no")
dim joined = concat("a", "b", "c")

' Lookup formulas
dim products = [
  ["Apple", 1.50, 100],
  ["Banana", 0.75, 200],
  ["Cherry", 2.00, 150]
]
dim price = vlookup("Banana", products, 2, false)
dim wildcard_price = xlookup("App*", ["Apple", "Apricot"], [1, 2], "N/A", 2)
dim ci_wildcard = xlookup("app*", ["Apple", "Apricot"], [1, 2], "N/A", 2, 1, true)
print("Total: " + str(total))`
  },
  array_filter: {
    description: "Array filter built-in",
    code: `' Array FILTER
dim names = ["Alice", "Bob", "Charlie", "Dana"]
dim scores = [88, 0, 92, 75]
dim passing = filter(names, scores)
print(passing)`
  },
  markdown_import: {
    description: "Import tables from markdown",
    code: `' Markdown Table Import
dim markdown_doc = "
# Project Status Report

## Task Overview
| Task | Status | Progress |
|------|--------|----------|
| Design | Complete | 100% |
| Implementation | In Progress | 75% |
| Testing | Pending | 0% |

## Team Members
| Name | Role | Active |
|------|------|--------|
| Alice | Lead | true |
| Bob | Dev | true |
| Charlie | QA | false |
"

' Import all tables from markdown
dim tables = import markdown_doc as markdown into book

' Process first table (Task Overview)
dim tasks = tables[0]
print("Task count: " + str(tasks.row_count() - 1))

' Process second table (Team Members)
dim team = tables[1]
dim active_count = query("
  SELECT COUNT(*) as count 
  FROM team 
  WHERE Active = 'true'
")
print("Active team members: " + str(active_count[0][0]))`
  },
  append_upsert: {
    description: "Append and upsert operations",
    code: `' Append + Upsert
dim users = [
  ["id", "name", "email"],
  [1, "Alice", "alice@example.com"],
  [2, "Bob", "bob@example.com"]
]

dim new_users = [
  ["id", "name", "email"],
  [2, "Bobby", "bob@example.com"],
  [3, "Cara", "cara@example.com"]
]

' Append distinct rows (by id)
users append distinct new_users on "id"

' Upsert rows by id
dim updates = [
  ["id", "name", "email"],
  [1, "Alice Smith", "alice@example.com"],
  [4, "Dan", "dan@example.com"]
]
users upsert updates on "id"

print(users)`
  },
  formulas_extended: {
    description: "Formula functions and lookups",
    code: `' Formulas
dim total = sum(1, 2, 3)
dim average = avg(10, 20, 30)
dim min_val = min(5, 3, 9)
dim max_val = max(5, 3, 9)
dim label = if(1, "yes", "no")

' Lookup + offset
dim products = [
  ["Apple", 1.50, 100],
  ["Banana", 0.75, 200],
  ["Cherry", 2.00, 150]
]
dim banana_qty = index(products, match("Banana", products, 0), 3)
dim block = offset(products, 1, 0, 1, 2)
dim bin_next = xlookup(6, [1, 3, 5, 7], ["A", "B", "C", "D"], "N/A", 1, 2)
print("Total: " + str(total) + ", Avg: " + str(average))`
  },
  markdown_tables: {
    description: "Markdown table import (Rust API)",
    code: `' Markdown table import uses the Rust API (not available in DSL yet)
print("Use piptable_markdown::extract_tables(markdown) in Rust")`
  }
};

// Signals for state management
export const code = signal(examples.hello.code);
export const selectedExample = signal('hello');
export const theme = signal<'light' | 'dark'>('dark');
export const output = signal('');
export const isRunning = signal(false);
export const error = signal<string | null>(null);

// Actions
export function selectExample(name: string) {
  const example = examples[name];
  if (example) {
    selectedExample.value = name;
    code.value = example.code;
    output.value = '';
    error.value = null;
  }
}

export function setTheme(newTheme: 'light' | 'dark') {
  theme.value = newTheme;
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('playground-theme', newTheme);
  }
}

// Share functionality
export function getCurrentShareableState(): ShareableState {
  return {
    code: code.value,
    theme: theme.value,
    selectedExample: selectedExample.value
  };
}

export function loadFromSharedState(state: ShareableState) {
  if (state.code !== undefined) {
    code.value = state.code;
  }
  if (state.theme) {
    theme.value = state.theme;
  }
  if (state.selectedExample) {
    selectedExample.value = state.selectedExample;
  }
  
  // Clear any existing output/errors when loading shared code
  output.value = '';
  error.value = null;
}

function escapeHtml(str: string): string {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

export async function runCode() {
  isRunning.value = true;
  error.value = null;
  
  try {
    let result = '';
    const execResult = await executeCode(code.value);

    if (execResult.success) {
      result += '<div class="text-green-600 dark:text-green-400 mb-2">✓ Execution succeeded</div>\n';
      if (execResult.output && execResult.output.length > 0) {
        const outputLines = execResult.output
          .map((line: string) => escapeHtml(line))
          .join('\n');
        result += '<div class="mt-2 text-sm font-medium">Output</div>';
        result += '<pre class="mt-1 p-3 bg-gray-100 dark:bg-gray-800 rounded-md overflow-x-auto text-xs">';
        result += outputLines;
        result += '</pre>';
      } else {
        result += '<div class="mt-2 text-gray-500 dark:text-gray-400 text-sm">No output.</div>';
      }

      if (execResult.result !== undefined && execResult.result !== null) {
        result += '<div class="mt-3 text-sm font-medium">Result</div>';
        result += '<pre class="mt-1 p-3 bg-gray-100 dark:bg-gray-800 rounded-md overflow-x-auto text-xs">';
        result += escapeHtml(JSON.stringify(execResult.result, null, 2));
        result += '</pre>';
      }
    } else {
      const errMsg = execResult.error || 'Unknown error';
      error.value = errMsg;
      result += '<div class="text-red-600 dark:text-red-400 mb-2">❌ Execution error:</div>\n';
      result += '<pre class="mt-2 p-3 bg-red-50 dark:bg-red-900/20 rounded-md overflow-x-auto">';
      result += escapeHtml(errMsg);
      result += '</pre>';
    }
    
    // Sanitize the output HTML before storing
    output.value = DOMPurify.sanitize(result);
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Unknown error';
    output.value = DOMPurify.sanitize(
      `<div class="text-red-600 dark:text-red-400">Error: ${error.value}</div>`
    );
  } finally {
    isRunning.value = false;
  }
}

// Initialize theme from localStorage and load shared state from URL
if (typeof window !== 'undefined') {
  // Try to load shared state from URL first
  const sharedState = loadSharedState();
  if (sharedState) {
    loadFromSharedState(sharedState);
    
    // Remove the share parameter from URL after loading to clean up the URL bar
    const url = new URL(window.location.href);
    if (url.searchParams.has('share')) {
      url.searchParams.delete('share');
      window.history.replaceState({}, '', url.toString());
    }
  } else {
    // Fall back to localStorage theme if no shared state
    try {
      const savedTheme = localStorage.getItem('playground-theme');
      if (savedTheme === 'light' || savedTheme === 'dark') {
        theme.value = savedTheme;
      }
    } catch {
      // localStorage unavailable (e.g., private browsing), use default theme
    }
  }
}
