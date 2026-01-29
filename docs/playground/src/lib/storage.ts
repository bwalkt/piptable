export interface SavedScript {
  id: string;
  name: string;
  code: string;
  timestamp: Date;
  description?: string;
}

export interface StorageState {
  scripts: Record<string, SavedScript>;
  recentScripts: string[]; // Array of script IDs in order of recent access
  settings: {
    maxRecentScripts: number;
    autoSave: boolean;
  };
}

const STORAGE_KEY = 'piptable-playground';
const DEFAULT_SETTINGS = {
  maxRecentScripts: 10,
  autoSave: true,
};

// Generate unique ID for scripts
export function generateScriptId(): string {
  return `script_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`;
}

// Load storage state from localStorage
export function loadStorageState(): StorageState {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) {
      return {
        scripts: {},
        recentScripts: [],
        settings: DEFAULT_SETTINGS,
      };
    }

    const parsed = JSON.parse(stored) as StorageState;
    
    // Validate structure
    if (!parsed.scripts || typeof parsed.scripts !== 'object') {
      parsed.scripts = {};
    }
    if (!Array.isArray(parsed.recentScripts)) {
      parsed.recentScripts = [];
    }
    
    // Convert timestamp strings back to Date objects
    Object.values(parsed.scripts).forEach(script => {
      if (typeof script.timestamp === 'string') {
        script.timestamp = new Date(script.timestamp);
      }
    });

    // Ensure settings exist with defaults
    parsed.settings = {
      ...DEFAULT_SETTINGS,
      ...parsed.settings,
    };

    return parsed;
  } catch (error) {
    console.error('Failed to load storage state:', error);
    return {
      scripts: {},
      recentScripts: [],
      settings: DEFAULT_SETTINGS,
    };
  }
}

// Save storage state to localStorage
export function saveStorageState(state: StorageState): void {
  try {
    const serialized = JSON.stringify(state);
    localStorage.setItem(STORAGE_KEY, serialized);
  } catch (error) {
    console.error('Failed to save storage state:', error);
    throw new Error('Failed to save to local storage. Storage may be full.');
  }
}

// Save a script to local storage
export function saveScript(name: string, code: string, description?: string): string {
  const state = loadStorageState();
  const id = generateScriptId();
  const script: SavedScript = {
    id,
    name: name.trim() || `Script ${Object.keys(state.scripts).length + 1}`,
    code,
    timestamp: new Date(),
    description,
  };

  state.scripts[id] = script;
  
  // Add to recent scripts (at the beginning)
  state.recentScripts = [id, ...state.recentScripts.filter(sid => sid !== id)];
  
  // Limit recent scripts
  if (state.recentScripts.length > state.settings.maxRecentScripts) {
    state.recentScripts = state.recentScripts.slice(0, state.settings.maxRecentScripts);
  }

  saveStorageState(state);
  return id;
}

// Update an existing script
export function updateScript(id: string, updates: Partial<Pick<SavedScript, 'name' | 'code' | 'description'>>): void {
  const state = loadStorageState();
  
  if (!state.scripts[id]) {
    throw new Error('Script not found');
  }

  state.scripts[id] = {
    ...state.scripts[id],
    ...updates,
    timestamp: new Date(),
  };

  // Move to front of recent scripts
  state.recentScripts = [id, ...state.recentScripts.filter(sid => sid !== id)];

  saveStorageState(state);
}

// Delete a script
export function deleteScript(id: string): void {
  const state = loadStorageState();
  
  if (!state.scripts[id]) {
    throw new Error('Script not found');
  }

  delete state.scripts[id];
  state.recentScripts = state.recentScripts.filter(sid => sid !== id);

  saveStorageState(state);
}

// Get a specific script by ID
export function getScript(id: string): SavedScript | null {
  const state = loadStorageState();
  return state.scripts[id] || null;
}

// Get all saved scripts
export function getAllScripts(): SavedScript[] {
  const state = loadStorageState();
  return Object.values(state.scripts).sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
}

// Get recent scripts in order
export function getRecentScripts(): SavedScript[] {
  const state = loadStorageState();
  return state.recentScripts
    .map(id => state.scripts[id])
    .filter(Boolean);
}

// Search scripts by name or content
export function searchScripts(query: string): SavedScript[] {
  if (!query.trim()) {
    return getAllScripts();
  }

  const state = loadStorageState();
  const searchTerm = query.toLowerCase();
  
  return Object.values(state.scripts).filter(script => 
    script.name.toLowerCase().includes(searchTerm) ||
    script.code.toLowerCase().includes(searchTerm) ||
    (script.description && script.description.toLowerCase().includes(searchTerm))
  ).sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
}

// Mark script as recently accessed
export function markScriptAsAccessed(id: string): void {
  const state = loadStorageState();
  
  if (!state.scripts[id]) {
    return;
  }

  // Move to front of recent scripts
  state.recentScripts = [id, ...state.recentScripts.filter(sid => sid !== id)];
  
  // Limit recent scripts
  if (state.recentScripts.length > state.settings.maxRecentScripts) {
    state.recentScripts = state.recentScripts.slice(0, state.settings.maxRecentScripts);
  }

  saveStorageState(state);
}

// Update storage settings
export function updateSettings(updates: Partial<StorageState['settings']>): void {
  const state = loadStorageState();
  state.settings = {
    ...state.settings,
    ...updates,
  };
  saveStorageState(state);
}

// Get storage usage information
export function getStorageInfo(): { used: number; total: number; scripts: number } {
  try {
    const state = loadStorageState();
    const serialized = JSON.stringify(state);
    const used = new Blob([serialized]).size;
    
    // Estimate total storage (usually 5-10MB for localStorage)
    const total = 5 * 1024 * 1024; // 5MB estimate
    
    return {
      used,
      total,
      scripts: Object.keys(state.scripts).length,
    };
  } catch (error) {
    return { used: 0, total: 0, scripts: 0 };
  }
}

// Clear all stored data
export function clearAllData(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch (error) {
    console.error('Failed to clear storage:', error);
  }
}

// Export all scripts as JSON
export function exportAllScripts(): string {
  const state = loadStorageState();
  return JSON.stringify({
    version: '1.0',
    exported: new Date().toISOString(),
    scripts: state.scripts,
  }, null, 2);
}

// Import scripts from JSON
export function importScripts(jsonData: string): number {
  try {
    const imported = JSON.parse(jsonData);
    
    if (!imported.scripts || typeof imported.scripts !== 'object') {
      throw new Error('Invalid export format');
    }

    const state = loadStorageState();
    let importedCount = 0;

    Object.values(imported.scripts).forEach((script: any) => {
      if (script && typeof script === 'object' && script.code && script.name) {
        const id = generateScriptId();
        state.scripts[id] = {
          id,
          name: `${script.name} (imported)`,
          code: script.code,
          timestamp: new Date(script.timestamp || new Date()),
          description: script.description,
        };
        importedCount++;
      }
    });

    saveStorageState(state);
    return importedCount;
  } catch (error) {
    console.error('Failed to import scripts:', error);
    throw new Error('Failed to import scripts. Please check the file format.');
  }
}