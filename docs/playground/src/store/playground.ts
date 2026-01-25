import { signal } from '@preact/signals';
import DOMPurify from 'dompurify';

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

export async function runCode() {
  isRunning.value = true;
  error.value = null;
  
  try {
    // Simulate execution
    await new Promise(resolve => setTimeout(resolve, 500));
    
    // Parse print statements for mock output
    let result = '<div class="text-green-600 dark:text-green-400 mb-2">✓ Code parsed successfully</div>\n';
    
    const prints = code.value.match(/PRINT\s+"([^"]+)"/gi);
    if (prints && prints.length > 0) {
      result += '<div class="mt-4"><strong>Console Output:</strong></div>\n';
      result += '<pre class="mt-2 p-3 bg-gray-100 dark:bg-gray-800 rounded-md overflow-x-auto">';
      
      prints.forEach(p => {
        const content = p.match(/PRINT\s+"([^"]+)"/i)?.[1] || '';
        result += content + '\n';
      });
      
      result += '</pre>';
    }
    
    result += '<div class="mt-4 text-gray-500 dark:text-gray-400 text-sm">';
    result += 'Note: Full execution will be available when WASM module is integrated (Issue #126)';
    result += '</div>';
    
    // Sanitize the output HTML before storing
    output.value = DOMPurify.sanitize(result);
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Unknown error';
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