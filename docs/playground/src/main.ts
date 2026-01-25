import { EditorView, basicSetup } from 'codemirror';
import { keymap } from '@codemirror/view';
import { indentWithTab } from '@codemirror/commands';
import { oneDark } from '@codemirror/theme-one-dark';
import { usePlaygroundStore, examples } from './store';
import DOMPurify from 'dompurify';

// Editor instance
let editor: EditorView | null = null;

// Error boundary for CodeMirror initialization
function initializeEditor() {
  try {
    const editorElement = document.getElementById('editor');
    if (!editorElement) {
      console.error('Editor element not found');
      return;
    }

    const store = usePlaygroundStore.getState();
    
    editor = new EditorView({
      doc: store.code,
      extensions: [
        basicSetup,
        keymap.of([
          indentWithTab,
          {
            key: 'Ctrl-Enter',
            mac: 'Cmd-Enter',
            run: () => {
              store.runCode();
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
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            store.setCode(update.state.doc.toString());
          }
        })
      ],
      parent: editorElement
    });
  } catch (error) {
    console.error('Failed to initialize editor:', error);
    const editorElement = document.getElementById('editor');
    if (editorElement) {
      editorElement.innerHTML = `
        <div style="padding: 2rem; color: #ff6b6b;">
          <h3>Editor Initialization Failed</h3>
          <p>Unable to load the code editor. Please refresh the page.</p>
          <pre style="margin-top: 1rem; font-size: 0.9em;">${DOMPurify.sanitize(String(error))}</pre>
        </div>
      `;
    }
  }
}

// Update editor content when store changes
function syncEditorWithStore() {
  if (!editor) return;
  
  const store = usePlaygroundStore.getState();
  const currentCode = editor.state.doc.toString();
  
  if (currentCode !== store.code) {
    editor.dispatch({
      changes: {
        from: 0,
        to: editor.state.doc.length,
        insert: store.code
      }
    });
  }
}

// Render output
function renderOutput() {
  const outputEl = document.getElementById('output');
  if (!outputEl) return;
  
  const { output, error, isRunning } = usePlaygroundStore.getState();
  
  if (isRunning) {
    outputEl.innerHTML = '<div class="spinner"></div><div>Running...</div>';
    outputEl.setAttribute('aria-busy', 'true');
  } else if (error) {
    outputEl.innerHTML = `<div class="error" role="alert">✗ Error: ${DOMPurify.sanitize(error)}</div>`;
    outputEl.setAttribute('aria-busy', 'false');
  } else if (output) {
    // Sanitize output HTML to prevent XSS
    outputEl.innerHTML = DOMPurify.sanitize(output);
    outputEl.setAttribute('aria-busy', 'false');
  } else {
    outputEl.innerHTML = '<div style="color: #666;">Click "Run" or press Ctrl+Enter (Cmd+Enter on Mac) to execute</div>';
    outputEl.setAttribute('aria-busy', 'false');
  }
}

// Render example list
function renderExamples() {
  const examplesEl = document.getElementById('examples-list');
  if (!examplesEl) return;
  
  const { selectedExample } = usePlaygroundStore.getState();
  
  examplesEl.innerHTML = Object.entries(examples).map(([key, example]) => `
    <button 
      class="example-item ${key === selectedExample ? 'active' : ''}"
      data-example="${key}"
      aria-label="Load ${example.description}"
      title="${example.description}"
    >
      ${key.charAt(0).toUpperCase() + key.slice(1)}
    </button>
  `).join('');
}

// Update theme
function applyTheme() {
  const { theme } = usePlaygroundStore.getState();
  
  if (theme === 'light') {
    document.documentElement.classList.remove('dark');
    document.documentElement.classList.add('light');
    document.body.style.background = '#ffffff';
    document.body.style.color = '#333333';
  } else {
    document.documentElement.classList.remove('light');
    document.documentElement.classList.add('dark');
    document.body.style.background = '#1e1e1e';
    document.body.style.color = '#d4d4d4';
  }
  
  // Update theme selector if it exists
  const themeSelector = document.getElementById('theme-selector') as HTMLSelectElement;
  if (themeSelector) {
    themeSelector.value = theme;
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
  const store = usePlaygroundStore.getState();
  
  // Initialize editor
  initializeEditor();
  
  // Apply saved theme
  applyTheme();
  
  // Render initial UI
  renderExamples();
  renderOutput();
  
  // Subscribe to store changes
  usePlaygroundStore.subscribe(() => {
    syncEditorWithStore();
    renderOutput();
    renderExamples();
    applyTheme();
  });
  
  // Wire up buttons
  const runBtn = document.getElementById('run-btn');
  if (runBtn) {
    runBtn.addEventListener('click', () => store.runCode());
    runBtn.setAttribute('aria-label', 'Run code (Ctrl+Enter)');
  }
  
  const formatBtn = document.getElementById('format-btn');
  if (formatBtn) {
    formatBtn.addEventListener('click', () => {
      console.log('Format code - not yet implemented');
    });
    formatBtn.setAttribute('aria-label', 'Format code');
  }
  
  // Wire up example selector with event delegation
  const examplesEl = document.getElementById('examples-list');
  if (examplesEl) {
    examplesEl.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      if (target.classList.contains('example-item')) {
        const exampleName = target.dataset.example;
        if (exampleName) {
          store.selectExample(exampleName);
        }
      }
    });
  }
  
  // Theme selector
  const themeSelector = document.getElementById('theme-selector');
  if (themeSelector) {
    themeSelector.addEventListener('change', (e) => {
      const target = e.target as HTMLSelectElement;
      store.setTheme(target.value as 'light' | 'dark');
    });
    themeSelector.setAttribute('aria-label', 'Select theme');
  }
  
  // Add keyboard shortcut help
  const helpText = document.createElement('div');
  helpText.style.cssText = 'position: absolute; top: 10px; right: 10px; font-size: 0.8em; color: #666;';
  helpText.textContent = 'Tip: Press Ctrl+Enter (Cmd+Enter on Mac) to run';
  helpText.setAttribute('role', 'note');
  document.getElementById('editor')?.appendChild(helpText);
});

export { initializeEditor };