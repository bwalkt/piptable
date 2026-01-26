// Utility functions for generating embeddable playground URLs

export interface EmbedOptions {
  code: string;
  height?: string;
  readonly?: boolean;
  showOutput?: boolean;
  title?: string;
  description?: string;
  baseUrl?: string;
}

/**
 * Generate an embed URL for a PipTable code example
 */
export function generateEmbedUrl(options: EmbedOptions): string {
  const baseUrl = options.baseUrl || '/playground/embed.html';
  const params = new URLSearchParams();
  
  params.set('code', encodeURIComponent(options.code));
  
  if (options.height && options.height !== '200px') {
    params.set('height', options.height);
  }
  
  if (options.readonly === true) {
    params.set('readonly', 'true');
  }
  
  if (options.showOutput === false) {
    params.set('showOutput', 'false');
  }
  
  if (options.title) {
    params.set('title', options.title);
  }
  
  if (options.description) {
    params.set('description', options.description);
  }
  
  return `${baseUrl}?${params.toString()}`;
}

/**
 * Generate an iframe HTML element for embedding
 */
export function generateEmbedIframe(options: EmbedOptions): string {
  const url = generateEmbedUrl(options);
  const height = options.height || '200px';
  
  return `<iframe 
    src="${url}" 
    width="100%" 
    height="${height}"
    frameborder="0" 
    style="border: 1px solid #e5e7eb; border-radius: 8px; background: white;"
    title="${options.title || 'PipTable Code Example'}"
    sandbox="allow-scripts allow-same-origin"
  ></iframe>`;
}

/**
 * Create a code block replacement for mdBook
 */
export function createCodeBlockReplacement(codeBlock: string, language: string = 'piptable'): string {
  // Extract code from markdown code block
  const codeMatch = codeBlock.match(/```(?:piptable|vba)?\n([\s\S]*?)\n```/);
  const code = codeMatch ? codeMatch[1].trim() : codeBlock;
  
  // Check for special comments that configure the playground
  const titleMatch = code.match(/^\s*'\s*@title\s+(.+)$/m);
  const descMatch = code.match(/^\s*'\s*@description\s+(.+)$/m);
  const heightMatch = code.match(/^\s*'\s*@height\s+(.+)$/m);
  const readonlyMatch = code.match(/^\s*'\s*@readonly\s*$/m);
  const noOutputMatch = code.match(/^\s*'\s*@no-output\s*$/m);
  
  // Remove configuration comments from displayed code
  const cleanCode = code
    .replace(/^\s*'\s*@\w+.*$/gm, '')
    .replace(/\n\n+/g, '\n\n')
    .trim();
  
  const options: EmbedOptions = {
    code: cleanCode,
    title: titleMatch?.[1],
    description: descMatch?.[1],
    height: heightMatch?.[1] || '250px',
    readonly: !!readonlyMatch,
    showOutput: !noOutputMatch,
  };
  
  return generateEmbedIframe(options);
}

// Browser-compatible version for client-side usage
if (typeof window !== 'undefined') {
  // Make functions available globally for use in documentation
  (window as any).PipTableEmbed = {
    generateEmbedUrl,
    generateEmbedIframe,
    createCodeBlockReplacement
  };
}