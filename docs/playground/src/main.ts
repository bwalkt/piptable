import { EditorView, basicSetup } from 'codemirror';
import { keymap } from '@codemirror/view';
import { indentWithTab } from '@codemirror/commands';
import { oneDark } from '@codemirror/theme-one-dark';

// Example code snippets
const examples: Record<string, string> = {
  hello: `' Hello World in PipTable
print("Hello, World!")

dim name = "PipTable"
print("Welcome to " + name + " Playground!")`,

  variables: `' Variables and Types
dim message = "Hello"        ' String
dim count = 42              ' Integer
dim price = 19.99          ' Float
dim active = true          ' Boolean
dim empty = null          ' Null

' Arrays and Objects
dim numbers = [1, 2, 3, 4, 5]
dim person = {
  "name": "Alice",
  "age": 30,
  "city": "New York"
}

print("Message: " + message)
print("Count: " + str(count))
print("Numbers: " + str(numbers))`,

  import: `' Import CSV Data
' Note: In playground, this uses sample data
dim sales = import "sales.csv" into sheet

' Display first few rows
print("Sales data loaded")
print("Rows: " + str(sales.row_count()))
print("Columns: " + str(sales.col_count()))`,

  sql: `' SQL Query on Data
dim users = import "users.csv" into sheet

' Query the data
dim adults = query("
  SELECT name, age, city
  FROM users
  WHERE age >= 18
  ORDER BY age DESC
  LIMIT 10
")

print("Adult users:")
export adults to "console"`,

  join: `' Join Operations
dim customers = import "customers.csv" into sheet
dim orders = import "orders.csv" into sheet

' Inner join on customer_id
dim customer_orders = customers join orders on "id" = "customer_id"

' Show results
print("Customer orders joined")
export customer_orders to "console"`,

  functions: `' Functions and Subroutines
function calculateTotal(price, quantity)
  dim tax_rate = 0.08
  dim subtotal = price * quantity
  dim tax = subtotal * tax_rate
  return subtotal + tax
end function

sub printInvoice(item, price, qty)
  dim total = calculateTotal(price, qty)
  print("Item: " + item)
  print("Price: $" + str(price))
  print("Quantity: " + str(qty))
  print("Total: $" + str(total))
end sub

' Use the functions
printInvoice("Widget", 29.99, 3)`,

  loops: `' Loops and Control Flow
dim scores = [85, 92, 78, 95, 88]

' For loop
dim total = 0
for i = 0 to len(scores) - 1
  total = total + scores[i]
end for
dim average = total / len(scores)
print("Average score: " + str(average))

' For each loop
print("Grades:")
for each score in scores
  if score >= 90 then
    print(str(score) + " - A")
  elseif score >= 80 then
    print(str(score) + " - B")
  else
    print(str(score) + " - C")
  end if
end for

' While loop
dim countdown = 5
print("Countdown:")
while countdown > 0
  print(str(countdown) + "...")
  countdown = countdown - 1
end while
print("Blast off!")`,

  pipeline: `' Complete Data Pipeline
' 1. Import multiple data sources
dim sales_jan = import "sales_jan.csv" into sheet
dim sales_feb = import "sales_feb.csv" into sheet
dim products = import "products.csv" into sheet

' 2. Combine monthly data
sales_jan append sales_feb

' 3. Join with product information
dim full_data = sales_jan join products on "product_id" = "id"

' 4. Analyze with SQL
dim summary = query("
  SELECT 
    product_name,
    COUNT(*) as transactions,
    SUM(quantity) as total_quantity,
    AVG(price) as avg_price,
    SUM(quantity * price) as revenue
  FROM full_data
  GROUP BY product_name
  ORDER BY revenue DESC
  LIMIT 10
")

' 5. Export results
print("Top 10 Products by Revenue:")
export summary to "console"
export summary to "top_products.csv"`
};

// Initialize editor
let editor: EditorView;

function initializeEditor() {
  const editorElement = document.getElementById('editor');
  if (!editorElement) return;

  editor = new EditorView({
    doc: examples.hello,
    extensions: [
      basicSetup,
      keymap.of([
        indentWithTab,
        {
          key: 'Ctrl-Enter',
          mac: 'Cmd-Enter',
          run: () => {
            runCode();
            return true;
          }
        }
      ]),
      oneDark,
      EditorView.theme({
        '&': {
          fontSize: '14px',
          height: '100%'
        },
        '.cm-content': {
          padding: '1rem',
          fontFamily: 'Consolas, Monaco, "Courier New", monospace'
        },
        '.cm-focused .cm-cursor': {
          borderLeftColor: '#528bff'
        },
        '.cm-line': {
          padding: '0 2px 0 4px'
        }
      })
    ],
    parent: editorElement
  });
}

// Run code (placeholder - will be replaced with WASM execution)
function runCode() {
  const code = editor.state.doc.toString();
  const outputEl = document.getElementById('output');
  if (!outputEl) return;

  // Clear previous output
  outputEl.innerHTML = '<div class="spinner"></div>';

  // Simulate execution delay
  setTimeout(() => {
    try {
      // For now, just show the code and a placeholder message
      outputEl.innerHTML = `
        <div class="success">✓ Code parsed successfully</div>
        <div style="color: #888; margin-top: 1rem;">
          <strong>Note:</strong> Full execution will be available when WASM module is integrated.
        </div>
        <div style="margin-top: 1rem;">
          <strong>Your code:</strong>
          <pre style="background: #2d2d30; padding: 1rem; border-radius: 4px; overflow-x: auto;">${escapeHtml(code)}</pre>
        </div>
      `;

      // If it's a simple print statement, show mock output
      if (code.includes('print(')) {
        const prints = code.match(/print\("([^"]+)"\)/g);
        if (prints) {
          outputEl.innerHTML += `
            <div style="margin-top: 1rem;">
              <strong>Console Output:</strong>
              <pre style="background: #1e3a1e; color: #89d185; padding: 1rem; border-radius: 4px;">`;
          
          prints.forEach(p => {
            const content = p.match(/print\("([^"]+)"\)/)?.[1] || '';
            outputEl.innerHTML += content + '\n';
          });
          
          outputEl.innerHTML += '</pre></div>';
        }
      }
    } catch (error) {
      outputEl.innerHTML = `
        <div class="error">✗ Error: ${error}</div>
      `;
    }
  }, 500);
}

// Format code (placeholder)
function formatCode() {
  // For now, just add some basic formatting
  const code = editor.state.doc.toString();
  // This would be replaced with proper formatter
  console.log('Format code:', code);
}

// Load example
function loadExample(exampleName: string) {
  const code = examples[exampleName];
  if (code && editor) {
    editor.dispatch({
      changes: {
        from: 0,
        to: editor.state.doc.length,
        insert: code
      }
    });
  }
}

// Utility function to escape HTML
function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  initializeEditor();

  // Wire up buttons
  document.getElementById('run-btn')?.addEventListener('click', runCode);
  document.getElementById('format-btn')?.addEventListener('click', formatCode);

  // Wire up example selector
  document.querySelectorAll('.example-item').forEach(item => {
    item.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      const exampleName = target.dataset.example;
      
      if (exampleName) {
        // Update active state
        document.querySelectorAll('.example-item').forEach(el => {
          el.classList.remove('active');
        });
        target.classList.add('active');
        
        // Load example
        loadExample(exampleName);
      }
    });
  });

  // Theme selector (basic implementation)
  document.getElementById('theme-selector')?.addEventListener('change', (e) => {
    const target = e.target as HTMLSelectElement;
    if (target.value === 'light') {
      document.body.style.background = '#ffffff';
      document.body.style.color = '#333333';
      // Would need to update CodeMirror theme as well
    } else {
      document.body.style.background = '#1e1e1e';
      document.body.style.color = '#d4d4d4';
    }
  });
});

export { runCode, loadExample };