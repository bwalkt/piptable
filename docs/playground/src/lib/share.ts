export interface ShareableState {
  code: string;
  theme?: 'light' | 'dark';
  selectedExample?: string;
}

// Compress code using base64 encoding for URL sharing
export function encodeForURL(state: ShareableState): string {
  try {
    const json = JSON.stringify(state);
    const base64 = btoa(unescape(encodeURIComponent(json)));
    
    // Use URL-safe base64 encoding
    return base64
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=+$/, '');
  } catch (error) {
    console.error('Failed to encode state for URL:', error);
    return '';
  }
}

// Decompress code from URL parameter
export function decodeFromURL(encoded: string): ShareableState | null {
  try {
    // Convert from URL-safe base64
    let base64 = encoded
      .replace(/-/g, '+')
      .replace(/_/g, '/');
    
    // Add padding if needed
    const padding = base64.length % 4;
    if (padding > 0) {
      base64 += '='.repeat(4 - padding);
    }
    
    const json = decodeURIComponent(escape(atob(base64)));
    const state = JSON.parse(json) as ShareableState;
    
    // Validate the decoded state
    if (typeof state.code !== 'string') {
      throw new Error('Invalid state: code must be a string');
    }
    
    return state;
  } catch (error) {
    console.error('Failed to decode state from URL:', error);
    return null;
  }
}

// Generate shareable URL for the current playground state
export function generateShareURL(state: ShareableState): string {
  const encoded = encodeForURL(state);
  if (!encoded) {
    throw new Error('Failed to encode state for sharing');
  }
  
  const url = new URL(window.location.href);
  url.searchParams.set('share', encoded);
  
  // Remove other query parameters to keep URL clean
  for (const [key] of url.searchParams.entries()) {
    if (key !== 'share') {
      url.searchParams.delete(key);
    }
  }
  
  return url.toString();
}

// Load shared state from current URL
export function loadSharedState(): ShareableState | null {
  try {
    const url = new URL(window.location.href);
    const shared = url.searchParams.get('share');
    
    if (!shared) {
      return null;
    }
    
    return decodeFromURL(shared);
  } catch (error) {
    console.error('Failed to load shared state from URL:', error);
    return null;
  }
}

// Copy text to clipboard with fallback
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return true;
    } else {
      // Fallback for older browsers or non-secure contexts
      const textArea = document.createElement('textarea');
      textArea.value = text;
      textArea.style.position = 'fixed';
      textArea.style.left = '-999999px';
      textArea.style.top = '-999999px';
      document.body.appendChild(textArea);
      textArea.focus();
      textArea.select();
      
      const successful = document.execCommand('copy');
      document.body.removeChild(textArea);
      
      return successful;
    }
  } catch (error) {
    console.error('Failed to copy to clipboard:', error);
    return false;
  }
}

// Generate a short description for the shared code (for display purposes)
export function generateCodeDescription(code: string): string {
  const lines = code.trim().split('\n').filter(line => line.trim());
  
  if (lines.length === 0) {
    return 'Empty code';
  }
  
  // Look for first meaningful line (not just comments)
  const firstLine = lines.find(line => !line.trim().startsWith('//'));
  
  if (!firstLine) {
    return 'Code snippet';
  }
  
  // Truncate to reasonable length
  const description = firstLine.trim();
  if (description.length > 50) {
    return description.substring(0, 47) + '...';
  }
  
  return description;
}