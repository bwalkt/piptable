import { signal } from '@preact/signals';
import DOMPurify from 'dompurify';
import { parseCode, validateCode, getExamples, initializeWasm } from './wasm';

// Example metadata
export interface Example {
  code: string;
  description: string;
}

// Example code snippets  
export const examples: Record<string, Example> = {
  hello: {
    description: "Simple hello world example",
    code: `// Hello World in PipTable
PRINT "Hello, World!"

DIM name AS STRING = "PipTable"
PRINT "Welcome to " + name + " Playground!"`
  },
  variables: {
    description: "Variable declarations and basic types",
    code: `// Variables and Types
DIM message AS STRING = "Hello"
DIM count AS INT = 42
DIM price AS FLOAT = 19.99
DIM active AS BOOL = true

// Display values using string conversion
PRINT "Message: " + message
PRINT "Count: " + STR(count)
PRINT "Price: $" + STR(price)
PRINT "Active: " + STR(active)`
  },
  sql: {
    description: "SQL queries on sheet data",
    code: `// SQL Query on Data
DIM users AS SHEET = READ("users.csv")

// Query the data
DIM adults AS SHEET = QUERY(users, 
  "SELECT name, age, city FROM users WHERE age >= 18")

// Export results
WRITE(adults, "adult_users.csv")
PRINT "Query results exported"`
  },
  join: {
    description: "Join operations between sheets",
    code: `// Join Operations
DIM customers AS SHEET = READ("customers.csv")
DIM orders AS SHEET = READ("orders.csv")

// Inner join
DIM result AS SHEET = JOIN INNER customers, orders 
  ON customers.id = orders.customer_id

WRITE(result, "customer_orders.csv")
PRINT "Join complete!"`
  },
  loops: {
    description: "Loops and control flow",
    code: `// Loops and Control Flow
DIM sum AS INT = 0
FOR i = 1 TO 10
  sum = sum + i
  PRINT "i = " + STR(i) + ", sum = " + STR(sum)
NEXT

// Conditional
DIM score AS INT = 85
IF score >= 90 THEN
  PRINT "Grade: A"
ELSEIF score >= 80 THEN
  PRINT "Grade: B"
ELSE
  PRINT "Grade: C"
END IF`
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

// Initialize theme from localStorage
if (typeof localStorage !== 'undefined') {
  const savedTheme = localStorage.getItem('playground-theme');
  if (savedTheme === 'light' || savedTheme === 'dark') {
    theme.value = savedTheme;
  }
}