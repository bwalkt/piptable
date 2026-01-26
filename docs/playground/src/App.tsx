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
  runCode,
  getCurrentShareableState
} from './store/playground';
import { cn } from './lib/utils';
import { ShareButton } from './components/ShareButton';
import { ExportButton } from './components/ExportButton';

export function App() {
  const editorRef = useRef<HTMLDivElement>(null);
  const editorViewRef = useRef<EditorView | null>(null);
  const themeCompartment = useRef<Compartment>(new Compartment());
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  // Initialize WASM on mount
  useEffect(() => {
    initializeWasm().catch((err) => {
      console.error('WASM initialization failed:', err);
      error.value = 'Failed to load WebAssembly module. Please refresh the page.';
    });
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
            padding: '0.75rem 1rem',
            fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Consolas, "Liberation Mono", Menlo, monospace',
            fontSize: '0.875rem',
            lineHeight: '1.5',
            touchAction: 'manipulation'
          },
          '.cm-focused': {
            outline: 'none'
          },
          '.cm-line': {
            padding: '0 0',
            minHeight: '1.5rem'
          },
          '@media (max-width: 768px)': {
            '.cm-content': {
              padding: '0.5rem 0.75rem',
              fontSize: '0.8rem'
            }
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
      <header className="flex items-center justify-between px-3 sm:px-4 h-12 sm:h-14 border-b bg-card">
        <div className="flex items-center gap-2 sm:gap-3 min-w-0 flex-1">
          {/* Mobile menu button */}
          <button
            onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
            className={cn(
              "inline-flex items-center justify-center rounded-md text-sm font-medium md:hidden",
              "h-8 w-8 sm:h-9 sm:w-9 hover:bg-accent hover:text-accent-foreground",
              "ring-offset-background transition-colors touch-manipulation",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            )}
            aria-label="Toggle menu"
          >
            <svg className="w-4 h-4 sm:w-5 sm:h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
          <span className="text-lg sm:text-xl">🔧</span>
          <h1 className="text-base sm:text-lg font-semibold truncate">
            <span className="hidden sm:inline">PipTable Playground</span>
            <span className="sm:hidden">PipTable</span>
          </h1>
        </div>
        
        <div className="flex items-center gap-1 sm:gap-2 flex-shrink-0">
          {/* Hide share/export on very small screens */}
          <div className="hidden xs:flex items-center gap-1 sm:gap-2">
            <ShareButton getState={getCurrentShareableState} className="h-8 sm:h-9 px-2 sm:px-3 text-xs sm:text-sm" />
            
            <ExportButton 
              code={code.value}
              output={output.value}
              disabled={isRunning.value}
              className="h-8 sm:h-9 px-2 sm:px-3 text-xs sm:text-sm"
            />
          </div>
          
          <button
            onClick={() => setTheme(theme.value === 'dark' ? 'light' : 'dark')}
            className={cn(
              "inline-flex items-center justify-center rounded-md text-sm font-medium",
              "h-8 w-8 sm:h-9 sm:w-9 hover:bg-accent hover:text-accent-foreground",
              "ring-offset-background transition-colors touch-manipulation",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            )}
            aria-label="Toggle theme"
          >
            <span className="text-base sm:text-lg">{theme.value === 'dark' ? '☀️' : '🌙'}</span>
          </button>
          
          <button
            onClick={runCode}
            disabled={isRunning.value}
            className={cn(
              "inline-flex items-center justify-center rounded-md text-xs sm:text-sm font-medium",
              "h-8 sm:h-9 px-2 sm:px-4 bg-primary text-primary-foreground hover:bg-primary/90",
              "ring-offset-background transition-colors touch-manipulation",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
              "disabled:pointer-events-none disabled:opacity-50"
            )}
          >
            <span className="flex items-center gap-1">
              ▶ {isRunning.value ? 'Running...' : 'Run'} 
              <span className="hidden sm:inline ml-1 text-xs opacity-70">(Ctrl+Enter)</span>
            </span>
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
          "fixed left-0 top-12 sm:top-14 bottom-0 w-72 sm:w-64 z-50 transition-transform duration-300 md:translate-x-0",
          isMobileMenuOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0"
        )}>
          <div className="p-3 sm:p-4 border-b flex items-center justify-between">
            <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">Examples</h2>
            <button
              onClick={() => setIsMobileMenuOpen(false)}
              className="md:hidden h-7 w-7 rounded-md hover:bg-accent hover:text-accent-foreground flex items-center justify-center touch-manipulation"
              aria-label="Close menu"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>
          <div className="flex-1 overflow-y-auto p-2 sm:p-2">
            {Object.entries(examples).map(([key, example]) => (
              <button
                key={key}
                onClick={() => {
                  selectExample(key);
                  setIsMobileMenuOpen(false);
                }}
                className={cn(
                  "w-full text-left px-3 py-3 sm:py-2 rounded-md text-sm transition-colors mb-1 touch-manipulation",
                  "hover:bg-accent hover:text-accent-foreground active:bg-accent",
                  selectedExample.value === key && "bg-accent text-accent-foreground font-medium"
                )}
                title={example.description}
              >
                <div className="font-medium">
                  {key.charAt(0).toUpperCase() + key.slice(1).replace(/([A-Z])/g, ' $1').trim()}
                </div>
                <div className="text-xs text-muted-foreground mt-1 line-clamp-2 sm:hidden">
                  {example.description}
                </div>
              </button>
            ))}
          </div>
        </aside>

        {/* Main Content */}
        <div className="flex-1 flex flex-col lg:flex-row">
          {/* Editor Panel */}
          <div className="flex-1 flex flex-col min-w-0 h-1/2 lg:h-auto lg:min-h-0">
            <div className="px-3 sm:px-4 py-2 border-b bg-muted/50 flex items-center justify-between">
              <h3 className="text-sm font-medium">Editor</h3>
              <div className="flex items-center gap-2 text-xs text-muted-foreground lg:hidden">
                <span>Scroll down for output</span>
              </div>
            </div>
            <div ref={editorRef} className="flex-1 overflow-auto" />
          </div>

          {/* Output Panel */}
          <div className="flex-1 flex flex-col border-t lg:border-t-0 lg:border-l min-w-0 h-1/2 lg:h-auto lg:min-h-0">
            <div className="px-3 sm:px-4 py-2 border-b bg-muted/50 flex items-center justify-between">
              <h3 className="text-sm font-medium">Output</h3>
              <div className="flex items-center gap-2">
                {/* Show export button on mobile when hidden from header */}
                <div className="xs:hidden">
                  {!isRunning.value && !error.value && output.value && (
                    <ExportButton 
                      code={code.value}
                      output={output.value}
                      className="h-6 px-2 text-xs"
                    />
                  )}
                </div>
                {/* Always show small export button in output panel */}
                <div className="hidden xs:block lg:block">
                  {!isRunning.value && !error.value && output.value && (
                    <ExportButton 
                      code={code.value}
                      output={output.value}
                      className="h-7 px-2 text-xs"
                    />
                  )}
                </div>
              </div>
            </div>
            <div className="flex-1 overflow-auto p-3 sm:p-4">
              {isRunning.value && (
                <div className="flex items-center gap-2 text-muted-foreground">
                  <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
                  Running...
                </div>
              )}
              
              {error.value && (
                <div className="rounded-md bg-destructive/10 border border-destructive/20 p-3 text-destructive text-sm">
                  ✗ Error: {error.value}
                </div>
              )}
              
              {!isRunning.value && !error.value && output.value && (
                <div className="text-sm lg:text-base" dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(output.value) }} />
              )}
              
              {!isRunning.value && !error.value && !output.value && (
                <div className="text-muted-foreground text-sm lg:text-base text-center lg:text-left py-8 lg:py-4">
                  <div className="mb-2">Press "Run" to execute your code</div>
                  <div className="text-xs opacity-75 lg:hidden">
                    Tip: Use the keyboard shortcut Ctrl+Enter (⌘+Enter on Mac)
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}