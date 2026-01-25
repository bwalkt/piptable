import { useEffect, useRef, useState } from 'preact/hooks';
import { useSignalEffect } from '@preact/signals';
import { EditorView, basicSetup } from 'codemirror';
import { keymap } from '@codemirror/view';
import { indentWithTab } from '@codemirror/commands';
import { oneDark } from '@codemirror/theme-one-dark';
import { Compartment } from '@codemirror/state';
import DOMPurify from 'dompurify';
import { initializeWasm } from './store/wasm';
import { 
  code, 
  selectedExample, 
  theme, 
  output, 
  isRunning, 
  error,
  examples,
  selectExample,
  setTheme,
  runCode
} from './store/playground';
import { cn } from './lib/utils';

export function App() {
  const editorRef = useRef<HTMLDivElement>(null);
  const editorViewRef = useRef<EditorView | null>(null);
  const themeCompartment = useRef<Compartment>(new Compartment());
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  // Initialize WASM on mount
  useEffect(() => {
    initializeWasm().catch(console.error);
  }, []);

  // Initialize CodeMirror
  useEffect(() => {
    if (!editorRef.current || editorViewRef.current) return;

    editorViewRef.current = new EditorView({
      doc: code.value,
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
        themeCompartment.current.of(theme.value === 'dark' ? oneDark : []),
        EditorView.theme({
          '&': {
            height: '100%'
          },
          '.cm-content': {
            padding: '1rem',
            fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Consolas, "Liberation Mono", Menlo, monospace'
          }
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            code.value = update.state.doc.toString();
          }
        })
      ],
      parent: editorRef.current
    });

    return () => {
      editorViewRef.current?.destroy();
      editorViewRef.current = null;
    };
  }, []);

  // Update editor when code signal changes externally
  useSignalEffect(() => {
    if (editorViewRef.current && code.value !== editorViewRef.current.state.doc.toString()) {
      editorViewRef.current.dispatch({
        changes: {
          from: 0,
          to: editorViewRef.current.state.doc.length,
          insert: code.value
        }
      });
    }
  });

  // Apply theme to document and editor when theme signal changes
  useSignalEffect(() => {
    document.documentElement.className = theme.value;
    
    // Update CodeMirror theme
    if (editorViewRef.current && themeCompartment.current) {
      editorViewRef.current.dispatch({
        effects: themeCompartment.current.reconfigure(
          theme.value === 'dark' ? oneDark : []
        )
      });
    }
  });

  return (
    <div className="h-screen flex flex-col bg-background text-foreground">
      {/* Header */}
      <header className="flex items-center justify-between px-4 h-14 border-b bg-card">
        <div className="flex items-center gap-3">
          {/* Mobile menu button */}
          <button
            onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
            className={cn(
              "inline-flex items-center justify-center rounded-md text-sm font-medium md:hidden",
              "h-9 w-9 hover:bg-accent hover:text-accent-foreground",
              "ring-offset-background transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            )}
            aria-label="Toggle menu"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
          <span className="text-xl">🔧</span>
          <h1 className="text-lg font-semibold">PipTable Playground</h1>
        </div>
        
        <div className="flex items-center gap-2">
          <button
            onClick={() => setTheme(theme.value === 'dark' ? 'light' : 'dark')}
            className={cn(
              "inline-flex items-center justify-center rounded-md text-sm font-medium",
              "h-9 w-9 hover:bg-accent hover:text-accent-foreground",
              "ring-offset-background transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            )}
            aria-label="Toggle theme"
          >
            {theme.value === 'dark' ? '☀️' : '🌙'}
          </button>
          
          <button
            onClick={runCode}
            disabled={isRunning.value}
            className={cn(
              "inline-flex items-center justify-center rounded-md text-sm font-medium",
              "h-9 px-4 bg-primary text-primary-foreground hover:bg-primary/90",
              "ring-offset-background transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
              "disabled:pointer-events-none disabled:opacity-50"
            )}
          >
            ▶ {isRunning.value ? 'Running...' : 'Run'} 
            <span className="ml-2 text-xs opacity-70">(Ctrl+Enter)</span>
          </button>
        </div>
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Mobile menu overlay */}
        {isMobileMenuOpen && (
          <div 
            className="fixed inset-0 bg-black/50 z-40 md:hidden"
            onClick={() => setIsMobileMenuOpen(false)}
          />
        )}
        
        {/* Sidebar - hidden on mobile, visible on md+ */}
        <aside className={cn(
          "border-r bg-card flex flex-col",
          "md:relative md:w-64",
          "fixed left-0 top-14 bottom-0 w-64 z-50 transition-transform md:translate-x-0",
          isMobileMenuOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0"
        )}>
          <div className="p-4 border-b">
            <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">Examples</h2>
          </div>
          <div className="flex-1 overflow-y-auto p-2">
            {Object.entries(examples).map(([key, example]) => (
              <button
                key={key}
                onClick={() => {
                  selectExample(key);
                  setIsMobileMenuOpen(false);
                }}
                className={cn(
                  "w-full text-left px-3 py-2 rounded-md text-sm transition-colors mb-1",
                  "hover:bg-accent hover:text-accent-foreground",
                  selectedExample.value === key && "bg-accent text-accent-foreground font-medium"
                )}
                title={example.description}
              >
                {key.charAt(0).toUpperCase() + key.slice(1).replace(/([A-Z])/g, ' $1').trim()}
              </button>
            ))}
          </div>
        </aside>

        {/* Main Content */}
        <div className="flex-1 flex flex-col md:flex-row">
          {/* Editor Panel */}
          <div className="flex-1 flex flex-col min-w-0 min-h-[50vh] md:min-h-0">
            <div className="px-4 py-2 border-b bg-muted/50">
              <h3 className="text-sm font-medium">Editor</h3>
            </div>
            <div ref={editorRef} className="flex-1 overflow-auto" />
          </div>

          {/* Output Panel */}
          <div className="flex-1 flex flex-col border-t md:border-t-0 md:border-l min-w-0 min-h-[50vh] md:min-h-0">
            <div className="px-4 py-2 border-b bg-muted/50">
              <h3 className="text-sm font-medium">Output</h3>
            </div>
            <div className="flex-1 overflow-auto p-4">
              {isRunning.value && (
                <div className="flex items-center gap-2 text-muted-foreground">
                  <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
                  Running...
                </div>
              )}
              
              {error.value && (
                <div className="rounded-md bg-destructive/10 border border-destructive/20 p-3 text-destructive">
                  ✗ Error: {error.value}
                </div>
              )}
              
              {!isRunning.value && !error.value && output.value && (
                <div dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(output.value) }} />
              )}
              
              {!isRunning.value && !error.value && !output.value && (
                <div className="text-muted-foreground">
                  Press "Run" or Ctrl+Enter to execute your code
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}