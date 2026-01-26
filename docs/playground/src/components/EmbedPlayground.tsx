import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { useSignalEffect } from '@preact/signals';
import { EditorView, basicSetup } from 'codemirror';
import { keymap } from '@codemirror/view';
import { indentWithTab } from '@codemirror/commands';
import { oneDark } from '@codemirror/theme-one-dark';
import { Compartment } from '@codemirror/state';
import DOMPurify from 'dompurify';
import { signal } from '@preact/signals';
import { initializeWasm, parseCode } from '../store/wasm';
import { cn } from '../lib/utils';

interface EmbedPlaygroundProps {
  initialCode: string;
  height?: string;
  readonly?: boolean;
  showOutput?: boolean;
  title?: string;
  description?: string;
}

// Create local signals for the embedded playground
const createLocalPlaygroundState = (initialCode: string) => {
  const code = signal(initialCode);
  const output = signal('');
  const isRunning = signal(false);
  const error = signal<string | null>(null);
  const theme = signal<'light' | 'dark'>('dark');

  const runCode = async () => {
    isRunning.value = true;
    error.value = null;
    
    try {
      await initializeWasm();
      const parseResult = await parseCode(code.value);
      
      let result = '';
      
      if (parseResult.success) {
        result += '<div class="text-green-600 dark:text-green-400 mb-2">✓ Code parsed successfully!</div>\\n';
        
        // Show AST in collapsible section for embedded examples
        result += '<details class="mt-2">';
        result += '<summary class="cursor-pointer text-xs font-medium opacity-70">View Parse Tree</summary>';
        result += '<pre class="mt-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs overflow-x-auto max-h-32">';
        result += escapeHtml(parseResult.ast || 'No AST available');
        result += '</pre>';
        result += '</details>';
        
        result += '<div class="mt-2 text-gray-500 dark:text-gray-400 text-xs">';
        result += 'Validation successful. Full execution coming soon.';
        result += '</div>';
      } else {
        result += '<div class="text-red-600 dark:text-red-400 mb-2">❌ Parse error:</div>\\n';
        result += '<pre class="mt-1 p-2 bg-red-50 dark:bg-red-900/20 rounded text-xs overflow-x-auto">';
        result += escapeHtml(parseResult.error || 'Unknown parse error');
        result += '</pre>';
      }
      
      output.value = DOMPurify.sanitize(result);
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Unknown error';
      output.value = DOMPurify.sanitize(
        `<div class="text-red-600 dark:text-red-400 text-sm">Error: ${error.value}</div>`
      );
    } finally {
      isRunning.value = false;
    }
  };

  return { code, output, isRunning, error, theme, runCode };
};

function escapeHtml(str: string): string {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

export function EmbedPlayground({ 
  initialCode, 
  height = '200px', 
  readonly = false, 
  showOutput = true,
  title,
  description 
}: EmbedPlaygroundProps) {
  const editorRef = useRef<HTMLDivElement>(null);
  const editorViewRef = useRef<EditorView | null>(null);
  const themeCompartment = useRef<Compartment>(new Compartment());
  const [isExpanded, setIsExpanded] = useState(false);
  
  // Create local playground state - memoized to prevent re-creation on re-renders
  const state = useMemo(() => createLocalPlaygroundState(initialCode), [initialCode]);

  // Initialize WASM on mount
  useEffect(() => {
    initializeWasm().catch((err) => {
      console.error('WASM initialization failed:', err);
      state.error.value = 'Failed to load WebAssembly module. Please refresh the page.';
    });
  }, []);

  // Initialize CodeMirror
  useEffect(() => {
    if (!editorRef.current || editorViewRef.current) return;

    editorViewRef.current = new EditorView({
      doc: state.code.value,
      extensions: [
        basicSetup,
        keymap.of([
          indentWithTab,
          {
            key: 'Ctrl-Enter',
            mac: 'Cmd-Enter',
            run: () => {
              state.runCode();
              return true;
            }
          }
        ]),
        themeCompartment.current.of(state.theme.value === 'dark' ? oneDark : []),
        EditorView.theme({
          '&': {
            height: readonly ? 'auto' : height,
            fontSize: '14px'
          },
          '.cm-content': {
            padding: '0.75rem',
            fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Consolas, "Liberation Mono", Menlo, monospace'
          },
          '.cm-editor': {
            borderRadius: '0.375rem'
          },
          '.cm-focused': {
            outline: '2px solid rgb(59, 130, 246)',
            outlineOffset: '2px'
          }
        }),
        ...(readonly ? [EditorView.editable.of(false)] : []),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !readonly) {
            state.code.value = update.state.doc.toString();
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

  // Update editor when code signal changes
  useSignalEffect(() => {
    if (editorViewRef.current && state.code.value !== editorViewRef.current.state.doc.toString()) {
      editorViewRef.current.dispatch({
        changes: {
          from: 0,
          to: editorViewRef.current.state.doc.length,
          insert: state.code.value
        }
      });
    }
  });

  // Auto-run validation on code change for readonly examples
  useEffect(() => {
    if (readonly) {
      state.runCode();
    }
  }, [readonly, initialCode, state]);

  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden bg-white dark:bg-gray-950">
      {/* Header */}
      {(title || description || !readonly) && (
        <div className="flex items-center justify-between px-3 py-2 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700">
          <div className="flex-1">
            {title && (
              <h4 className="text-sm font-medium text-gray-900 dark:text-gray-100">{title}</h4>
            )}
            {description && (
              <p className="text-xs text-gray-600 dark:text-gray-400 mt-1">{description}</p>
            )}
          </div>
          
          {!readonly && (
            <div className="flex items-center gap-2">
              {showOutput && (
                <button
                  onClick={() => setIsExpanded(!isExpanded)}
                  className="text-xs px-2 py-1 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                >
                  {isExpanded ? 'Hide Output' : 'Show Output'}
                </button>
              )}
              
              <button
                onClick={state.runCode}
                disabled={state.isRunning.value}
                className={cn(
                  "text-xs px-3 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors",
                  "disabled:opacity-50 disabled:cursor-not-allowed"
                )}
              >
                {state.isRunning.value ? 'Running...' : 'Run'}
              </button>
            </div>
          )}
        </div>
      )}

      {/* Editor */}
      <div 
        ref={editorRef} 
        className={cn(
          "relative",
          readonly && "bg-gray-50 dark:bg-gray-900"
        )}
      />

      {/* Output Section */}
      {showOutput && (readonly || isExpanded) && (
        <div className="border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
          <div className="px-3 py-2">
            <div className="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2">Output</div>
            
            {state.isRunning.value && (
              <div className="flex items-center gap-2 text-gray-600 dark:text-gray-400 text-sm">
                <div className="h-3 w-3 animate-spin rounded-full border-2 border-blue-600 border-t-transparent" />
                Running...
              </div>
            )}
            
            {state.error.value && (
              <div className="rounded bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 p-2 text-red-700 dark:text-red-400 text-sm">
                ✗ Error: {state.error.value}
              </div>
            )}
            
            {!state.isRunning.value && !state.error.value && state.output.value && (
              <div 
                className="text-sm"
                dangerouslySetInnerHTML={{ __html: state.output.value }} 
              />
            )}
            
            {!state.isRunning.value && !state.error.value && !state.output.value && !readonly && (
              <div className="text-gray-500 dark:text-gray-400 text-sm">
                Press "Run" to validate your code
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}