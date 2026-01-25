import { create } from 'zustand';
import { persist } from 'zustand/middleware';

// Example metadata
export interface Example {
  code: string;
  description: string;
}

// Store state interface
interface PlaygroundState {
  // Editor state
  code: string;
  selectedExample: string;
  theme: 'light' | 'dark';
  
  // Output state
  output: string;
  isRunning: boolean;
  error: string | null;
  
  // Actions
  setCode: (code: string) => void;
  selectExample: (name: string) => void;
  setTheme: (theme: 'light' | 'dark') => void;
  setOutput: (output: string) => void;
  setError: (error: string | null) => void;
  setIsRunning: (isRunning: boolean) => void;
  runCode: () => Promise<void>;
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
PRINT "Active: " + STR(active)

// Math operations
DIM total AS FLOAT = price * 2.0
PRINT "Total: $" + STR(total)`
  },
  import: {
    description: "Import CSV data and basic operations",
    code: `// Import CSV Data
DIM sales AS SHEET = READ("sales.csv")
DIM products AS SHEET = READ("products.csv")

// Export data
WRITE(sales, "sales_backup.csv")
WRITE(products, "products_backup.json")

PRINT "Data import/export complete!"`
  },
  sql: {
    description: "SQL queries on sheet data",
    code: `// SQL Query on Data
DIM users AS SHEET = READ("users.csv")

// Query the data
DIM adults AS SHEET = QUERY(users, 
  "SELECT name, age, city FROM users WHERE age >= 18 ORDER BY age DESC LIMIT 10")

// Aggregate query
DIM stats AS SHEET = QUERY(users,
  "SELECT city, COUNT(*) as count, AVG(age) as avg_age FROM users GROUP BY city")

WRITE(adults, "adult_users.csv")
WRITE(stats, "user_statistics.csv")
PRINT "Query results exported"`
  },
  join: {
    description: "Join operations between sheets",
    code: `// Join Operations
DIM customers AS SHEET = READ("customers.csv")
DIM orders AS SHEET = READ("orders.csv")

// Inner join
DIM customer_orders AS SHEET = JOIN INNER customers, orders ON customers.id = orders.customer_id

// Left join to include all customers
DIM all_customers AS SHEET = JOIN LEFT customers, orders ON customers.id = orders.customer_id

// Right join for all orders
DIM all_orders AS SHEET = JOIN RIGHT customers, orders ON customers.id = orders.customer_id

// Export results
WRITE(customer_orders, "customer_orders.csv")
PRINT "Join operations complete!"`
  },
  functions: {
    description: "Built-in functions for string and math operations",
    code: `// Built-in String Functions
DIM text AS STRING = "  Hello, PipTable!  "
DIM trimmed AS STRING = TRIM(text)

PRINT "Original: '" + text + "'"
PRINT "Trimmed: '" + trimmed + "'"
PRINT "Uppercase: " + UPPER(trimmed)
PRINT "Lowercase: " + LOWER(trimmed)
PRINT "Length: " + STR(LEN(trimmed))

// Math Functions
DIM value AS FLOAT = -42.7
PRINT "Absolute: " + STR(ABS(value))
PRINT "Square root of 16: " + STR(SQRT(16))
PRINT "Round 3.7: " + STR(ROUND(3.7))

// Type Conversion
DIM num_str AS STRING = "123.45"
DIM number AS FLOAT = VAL(num_str)
DIM integer AS INT = INT(number)
PRINT "String to float: " + STR(number)
PRINT "Float to int: " + STR(integer)`
  },
  loops: {
    description: "Loops and control flow structures",
    code: `// Loops and Control Flow

// For loop
DIM sum AS INT = 0
FOR i = 1 TO 10
  sum = sum + i
  PRINT "i = " + STR(i) + ", sum = " + STR(sum)
NEXT

// While loop
DIM count AS INT = 5
PRINT "Countdown:"
WHILE count > 0
  PRINT STR(count) + "..."
  count = count - 1
WEND
PRINT "Blast off!"

// Conditional statements
DIM score AS INT = 85
DIM grade AS STRING

IF score >= 90 THEN
  grade = "A"
ELSEIF score >= 80 THEN
  grade = "B"
ELSEIF score >= 70 THEN
  grade = "C"
ELSE
  grade = "F"
END IF

PRINT "Score: " + STR(score) + " = Grade: " + grade`
  },
  pipeline: {
    description: "Complete ETL pipeline with append and upsert",
    code: `// Complete Data Pipeline

// 1. Import data sources
DIM master AS SHEET = READ("master_data.csv")
DIM daily_update AS SHEET = READ("daily_update.csv")
DIM products AS SHEET = READ("products.csv")

// 2. Append new records (avoid duplicates)
APPEND DISTINCT master FROM daily_update ON "transaction_id"
PRINT "Appended new transactions"

// 3. Upsert product updates
DIM product_updates AS SHEET = READ("product_updates.csv")
UPSERT products FROM product_updates ON "product_id"
PRINT "Updated product catalog"

// 4. Join and analyze
DIM full_data AS SHEET = JOIN INNER master, products ON master.product_id = products.product_id

// 5. Generate summary with SQL
DIM summary AS SHEET = QUERY(full_data,
  "SELECT product_name, COUNT(*) as sales, SUM(amount) as revenue " +
  "FROM full_data GROUP BY product_name ORDER BY revenue DESC LIMIT 10")

// 6. Create report book
DIM report AS BOOK = {"transactions": master, "products": products, "summary": summary}

// 7. Export results
WRITE(report, "monthly_report.xlsx")
WRITE(summary, "top_products.csv")
PRINT "Pipeline complete!"`
  }
};

// Create store with persistence
export const usePlaygroundStore = create<PlaygroundState>()(
  persist(
    (set, get) => ({
      // Initial state
      code: examples.hello.code,
      selectedExample: 'hello',
      theme: 'dark',
      output: '',
      isRunning: false,
      error: null,
      
      // Actions
      setCode: (code) => set({ code }),
      
      selectExample: (name) => {
        const example = examples[name];
        if (example) {
          set({ 
            selectedExample: name, 
            code: example.code,
            output: '',
            error: null 
          });
        }
      },
      
      setTheme: (theme) => {
        set({ theme });
        // Apply theme to document
        if (theme === 'light') {
          document.documentElement.classList.remove('dark');
          document.documentElement.classList.add('light');
        } else {
          document.documentElement.classList.remove('light');
          document.documentElement.classList.add('dark');
        }
      },
      
      setOutput: (output) => set({ output }),
      setError: (error) => set({ error }),
      setIsRunning: (isRunning) => set({ isRunning }),
      
      runCode: async () => {
        const { code } = get();
        set({ isRunning: true, error: null });
        
        try {
          // Simulate execution delay
          await new Promise(resolve => setTimeout(resolve, 500));
          
          // Parse print statements for mock output
          let output = '<div class="success">✓ Code parsed successfully</div>\n';
          output += '<div style="color: #888; margin-top: 1rem;">\n';
          output += '<strong>Note:</strong> Full execution will be available when WASM module is integrated.\n';
          output += '</div>\n';
          
          // Extract print statements
          const prints = code.match(/PRINT\s+"([^"]+)"/gi);
          if (prints && prints.length > 0) {
            output += '<div style="margin-top: 1rem;">\n';
            output += '<strong>Console Output:</strong>\n';
            output += '<pre style="background: #1e3a1e; color: #89d185; padding: 1rem; border-radius: 4px;">';
            
            prints.forEach(p => {
              const content = p.match(/PRINT\s+"([^"]+)"/i)?.[1] || '';
              output += content + '\n';
            });
            
            output += '</pre></div>';
          }
          
          set({ output, isRunning: false });
        } catch (error) {
          set({ 
            error: error instanceof Error ? error.message : 'Unknown error',
            isRunning: false 
          });
        }
      }
    }),
    {
      name: 'piptable-playground',
      partialize: (state) => ({ 
        code: state.code, 
        selectedExample: state.selectedExample,
        theme: state.theme 
      })
    }
  )
);