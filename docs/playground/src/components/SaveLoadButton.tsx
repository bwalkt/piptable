import { useState, useEffect } from 'preact/hooks';
import { 
  saveScript, 
  deleteScript, 
  getRecentScripts, 
  getAllScripts, 
  searchScripts, 
  markScriptAsAccessed,
  type SavedScript 
} from '../lib/storage';
import { cn } from '../lib/utils';

interface SaveLoadButtonProps {
  code: string;
  onLoadCode: (code: string, scriptName: string) => void;
  className?: string;
}

export function SaveLoadButton({ code, onLoadCode, className }: SaveLoadButtonProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<'save' | 'load'>('load');
  const [searchQuery, setSearchQuery] = useState('');
  const [scripts, setScripts] = useState<SavedScript[]>([]);
  const [recentScripts, setRecentScripts] = useState<SavedScript[]>([]);
  
  // Save form state
  const [saveName, setSaveName] = useState('');
  const [saveDescription, setSaveDescription] = useState('');
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'success' | 'error'>('idle');

  // Load scripts when modal opens
  useEffect(() => {
    if (isOpen) {
      loadScripts();
    }
  }, [isOpen, searchQuery]);

  // Close modal on Escape key
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setIsOpen(false);
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen]);

  const loadScripts = () => {
    try {
      const allScripts = searchQuery ? searchScripts(searchQuery) : getAllScripts();
      const recent = getRecentScripts();
      setScripts(allScripts);
      setRecentScripts(recent);
    } catch (error) {
      console.error('Failed to load scripts:', error);
    }
  };

  const handleSave = async () => {
    if (!saveName.trim()) {
      setSaveStatus('error');
      return;
    }

    setSaveStatus('saving');
    
    try {
      await new Promise(resolve => setTimeout(resolve, 300)); // Brief delay for UX
      saveScript(saveName.trim(), code, saveDescription.trim() || undefined);
      
      setSaveStatus('success');
      setSaveName('');
      setSaveDescription('');
      
      // Refresh the scripts list
      loadScripts();
      
      // Auto-close after success
      setTimeout(() => {
        setSaveStatus('idle');
        setIsOpen(false);
      }, 1500);
    } catch (error) {
      console.error('Failed to save script:', error);
      setSaveStatus('error');
    }
  };

  const handleLoad = (script: SavedScript) => {
    markScriptAsAccessed(script.id);
    onLoadCode(script.code, script.name);
    setIsOpen(false);
  };

  const handleDelete = (scriptId: string, event: Event) => {
    event.stopPropagation();
    
    if (confirm('Are you sure you want to delete this script?')) {
      try {
        deleteScript(scriptId);
        loadScripts();
      } catch (error) {
        console.error('Failed to delete script:', error);
      }
    }
  };

  const formatDate = (date: Date) => {
    return new Intl.DateTimeFormat('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(date);
  };

  const getCodePreview = (code: string) => {
    const firstLine = code.split('\n').find(line => line.trim()) || '';
    return firstLine.length > 50 ? firstLine.substring(0, 47) + '...' : firstLine;
  };

  return (
    <>
      <button
        onClick={() => setIsOpen(true)}
        className={cn(
          "inline-flex items-center justify-center rounded-md text-sm font-medium",
          "h-9 px-3 hover:bg-accent hover:text-accent-foreground",
          "ring-offset-background transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
          className
        )}
        aria-label="Save or load scripts"
        title="Save or load scripts"
      >
        <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} 
                d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a.997.997 0 01-.707.293H7a4 4 0 01-4-4V7a4 4 0 014-4z" />
        </svg>
        Scripts
      </button>

      {/* Modal */}
      {isOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-3 sm:p-4">
          <div className="bg-card border rounded-lg shadow-lg max-w-2xl w-full max-h-[95vh] sm:max-h-[90vh] overflow-hidden mx-2 sm:mx-0">
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b">
              <h2 className="text-lg font-semibold">Scripts</h2>
              <button
                onClick={() => setIsOpen(false)}
                className="h-8 w-8 rounded-md hover:bg-accent hover:text-accent-foreground flex items-center justify-center"
                aria-label="Close modal"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            {/* Tabs */}
            <div className="flex border-b" role="tablist">
              <button
                onClick={() => setActiveTab('load')}
                role="tab"
                aria-selected={activeTab === 'load'}
                id="tab-load"
                aria-controls="panel-load"
                className={cn(
                  "flex-1 px-4 py-3 text-sm font-medium transition-colors",
                  activeTab === 'load' 
                    ? "text-primary border-b-2 border-primary bg-primary/5" 
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                Load Script
              </button>
              <button
                onClick={() => setActiveTab('save')}
                role="tab"
                aria-selected={activeTab === 'save'}
                id="tab-save"
                aria-controls="panel-save"
                className={cn(
                  "flex-1 px-4 py-3 text-sm font-medium transition-colors",
                  activeTab === 'save' 
                    ? "text-primary border-b-2 border-primary bg-primary/5" 
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                Save Script
              </button>
            </div>

            {/* Content */}
            <div className="flex flex-col min-h-0 flex-1">
              {activeTab === 'load' ? (
                <div className="flex flex-col min-h-0 flex-1" role="tabpanel" id="panel-load" aria-labelledby="tab-load">
                  {/* Search */}
                  <div className="p-4 border-b">
                    <input
                      type="text"
                      placeholder="Search scripts..."
                      value={searchQuery}
                      onChange={(e) => setSearchQuery((e.target as HTMLInputElement).value)}
                      className="w-full px-3 py-2 text-sm bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                    />
                  </div>

                  {/* Scripts List */}
                  <div className="flex-1 overflow-auto p-4">
                    {/* Recent Scripts */}
                    {!searchQuery && recentScripts.length > 0 && (
                      <div className="mb-6">
                        <h3 className="text-sm font-medium text-muted-foreground mb-3">Recent</h3>
                        <div className="space-y-2">
                          {recentScripts.map((script) => (
                            <ScriptItem 
                              key={script.id}
                              script={script}
                              onLoad={handleLoad}
                              onDelete={handleDelete}
                              formatDate={formatDate}
                              getCodePreview={getCodePreview}
                            />
                          ))}
                        </div>
                      </div>
                    )}

                    {/* All Scripts */}
                    <div>
                      <h3 className="text-sm font-medium text-muted-foreground mb-3">
                        {searchQuery ? 'Search Results' : 'All Scripts'}
                      </h3>
                      {(() => {
                        // Filter out recent scripts from all scripts list when not searching
                        const recentIds = new Set(recentScripts.map(s => s.id));
                        const filteredScripts = searchQuery 
                          ? scripts 
                          : scripts.filter(s => !recentIds.has(s.id));
                        
                        return filteredScripts.length === 0 ? (
                          <div className="text-center py-8 text-muted-foreground">
                            {searchQuery ? 'No scripts found matching your search.' : 'No additional scripts.'}
                          </div>
                        ) : (
                          <div className="space-y-2">
                            {filteredScripts.map((script) => (
                              <ScriptItem 
                                key={script.id}
                                script={script}
                                onLoad={handleLoad}
                                onDelete={handleDelete}
                                formatDate={formatDate}
                                getCodePreview={getCodePreview}
                              />
                            ))}
                          </div>
                        );
                      })()}
                    </div>
                  </div>
                </div>
              ) : (
                /* Save Tab */
                <div className="p-4 space-y-4" role="tabpanel" id="panel-save" aria-labelledby="tab-save">
                  <div>
                    <label htmlFor="script-name" className="block text-sm font-medium mb-2">
                      Script Name *
                    </label>
                    <input
                      id="script-name"
                      type="text"
                      placeholder="Enter a name for your script"
                      value={saveName}
                      onChange={(e) => setSaveName((e.target as HTMLInputElement).value)}
                      className="w-full px-3 py-2 text-sm bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                    />
                  </div>

                  <div>
                    <label htmlFor="script-description" className="block text-sm font-medium mb-2">
                      Description (optional)
                    </label>
                    <textarea
                      id="script-description"
                      placeholder="Add a description..."
                      value={saveDescription}
                      onChange={(e) => setSaveDescription((e.target as HTMLTextAreaElement).value)}
                      rows={3}
                      className="w-full px-3 py-2 text-sm bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-ring resize-none"
                    />
                  </div>

                  <div>
                    <label className="block text-sm font-medium mb-2">Code Preview</label>
                    <div className="p-3 bg-muted rounded-md text-sm">
                      <div className="text-muted-foreground">
                        {code.split('\n').length} lines • {new Date().toLocaleDateString()}
                      </div>
                      <div className="mt-1 text-foreground">
                        {getCodePreview(code) || 'No code to save'}
                      </div>
                    </div>
                  </div>

                  <div className="flex gap-2 pt-2">
                    <button
                      onClick={handleSave}
                      disabled={saveStatus === 'saving' || !saveName.trim() || !code.trim()}
                      className={cn(
                        "flex-1 px-4 py-2 text-sm font-medium rounded-md transition-colors",
                        "focus:outline-none focus:ring-2 focus:ring-ring",
                        saveStatus === 'success' 
                          ? "bg-green-600 text-white" 
                          : saveStatus === 'error'
                          ? "bg-red-600 text-white"
                          : "bg-primary text-primary-foreground hover:bg-primary/90",
                        "disabled:pointer-events-none disabled:opacity-50"
                      )}
                    >
                      {saveStatus === 'saving' ? (
                        <div className="flex items-center justify-center gap-2">
                          <div className="h-3 w-3 animate-spin rounded-full border border-current border-t-transparent" />
                          Saving...
                        </div>
                      ) : saveStatus === 'success' ? (
                        <div className="flex items-center justify-center gap-2">
                          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                            <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                          </svg>
                          Saved!
                        </div>
                      ) : saveStatus === 'error' ? (
                        'Error - Try Again'
                      ) : (
                        'Save Script'
                      )}
                    </button>
                  </div>

                  {saveStatus === 'error' && (
                    <div className="text-sm text-red-600 dark:text-red-400">
                      {!saveName.trim() ? 'Please enter a script name.' : 'Failed to save script. Please try again.'}
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}

interface ScriptItemProps {
  script: SavedScript;
  onLoad: (script: SavedScript) => void;
  onDelete: (id: string, event: Event) => void;
  formatDate: (date: Date) => string;
  getCodePreview: (code: string) => string;
}

function ScriptItem({ script, onLoad, onDelete, formatDate, getCodePreview }: ScriptItemProps) {
  return (
    <div
      onClick={() => onLoad(script)}
      className="p-3 border rounded-md hover:bg-accent hover:text-accent-foreground cursor-pointer transition-colors group"
    >
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <h4 className="text-sm font-medium truncate">{script.name}</h4>
            <span className="text-xs text-muted-foreground">{formatDate(script.timestamp)}</span>
          </div>
          
          {script.description && (
            <p className="text-xs text-muted-foreground mb-1 line-clamp-2">
              {script.description}
            </p>
          )}
          
          <div className="text-xs text-muted-foreground font-mono">
            {getCodePreview(script.code)}
          </div>
        </div>
        
        <button
          onClick={(e) => onDelete(script.id, e)}
          className="opacity-0 group-hover:opacity-100 ml-2 p-1 rounded hover:bg-red-500/10 hover:text-red-500 transition-all"
          aria-label="Delete script"
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
}