import { signal } from '@preact/signals';
import DOMPurify from 'dompurify';
import { parseCode, validateCode, getExamples, initializeWasm } from './wasm';
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
  sql: {
    description: "SQL queries on sheet data",
    code: `' SQL Query on Data
dim users = import "users.csv" into sheet

' Query the data
dim adults = query("
  SELECT name, age, city
  FROM users
  WHERE age >= 18
")

' Export results
export adults to "adult_users.csv"
print("Query results exported")`
  },
  join: {
    description: "Join operations between sheets",
    code: `' Join Operations
dim customers = import "customers.csv" into sheet
dim orders = import "orders.csv" into sheet

' Inner join
dim result = customers join orders on "id" = "customer_id"

export result to "customer_orders.csv"
print("Join complete!")`
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
    description: "Formula functions and sheet ranges",
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

' Sheet range helpers
dim sales = import "sales.csv" into sheet
dim range_total = sum(sales, "A1:A10")
dim range_avg = avg(sales, "A1:A10")
print("Total: " + str(total))`
  },
  sheet_helpers: {
    description: "Sheet helpers and A1 access",
    code: `' Sheet Helpers
dim people = import "people.csv" into sheet

dim rows = sheet_row_count(people)
dim cols = sheet_col_count(people)
print("Rows: " + str(rows) + ", Cols: " + str(cols))

' Read + write by A1 notation
dim name = sheet_get_cell_value(people, "B2")
dim updated = sheet_set_a1(people, "C2", "active")
print("Name: " + str(name))`
  },
  sheet_range: {
    description: "Sheet ranges and filtering",
    code: `' Sheet Ranges
dim sales = import "sales.csv" into sheet

' Range extract
dim first_block = sheet_get_range(sales, "A1:C5")

' Filter rows by column value
dim high_value = sheet_filter_rows(sales, "status", "paid")
print("Filtered rows: " + str(len(high_value)))
`
  },
  sheet_map: {
    description: "Sheet map transformation",
    code: `' Sheet Map
dim data = import "people.csv" into sheet

' Uppercase all string cells
dim upper = sheet_map(data, "upper")
print("Mapped sheet rows: " + str(len(upper)))`
  },
  array_filter: {
    description: "Array filter built-in",
    code: `' Array FILTER
dim names = ["Alice", "Bob", "Charlie", "Dana"]
dim scores = [88, 0, 92, 75]
dim passing = filter(names, scores)
print(passing)`
  },
  formulas_extended: {
    description: "Formula functions and ranges",
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

' Range formulas
dim sheet = import "sales.csv" into sheet
dim range_total = sum(sheet, "A1:A10")
dim range_avg = avg(sheet, "A1:A10")

print("Total: " + str(total) + ", Avg: " + str(average))`
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
    // Initialize WASM if needed
    await initializeWasm();
    
    // Parse the code with real parser
    const parseResult = await parseCode(code.value);
    
    let result = '';
    
    if (parseResult.success) {
      result += '<div class="text-green-600 dark:text-green-400 mb-2">✓ Code parsed successfully!</div>\n';
      
      // Show AST in collapsible section
      result += '<details class="mt-4">';
      result += '<summary class="cursor-pointer text-sm font-medium">View Abstract Syntax Tree</summary>';
      result += '<pre class="mt-2 p-3 bg-gray-100 dark:bg-gray-800 rounded-md overflow-x-auto text-xs">';
      result += escapeHtml(parseResult.ast || 'No AST available');
      result += '</pre>';
      result += '</details>';
      
      result += '<div class="mt-4 text-gray-500 dark:text-gray-400 text-sm">';
      result += 'Note: Full execution coming soon. Currently showing parsing validation only.';
      result += '</div>';
    } else {
      result += '<div class="text-red-600 dark:text-red-400 mb-2">❌ Parse error:</div>\n';
      result += '<pre class="mt-2 p-3 bg-red-50 dark:bg-red-900/20 rounded-md overflow-x-auto">';
      result += escapeHtml(parseResult.error || 'Unknown parse error');
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
