#!/usr/bin/env node

// mdBook preprocessor to convert PipTable code blocks into interactive examples
const fs = require('fs');
const path = require('path');

// Check if this is being run as a preprocessor
const args = process.argv.slice(2);

if (args[0] === 'supports') {
  // Check if this preprocessor supports the renderer
  const renderer = args[1];
  process.exit(renderer === 'html' ? 0 : 1);
}

// Read the entire input from stdin
let input = '';
process.stdin.setEncoding('utf8');

process.stdin.on('data', chunk => {
  input += chunk;
});

process.stdin.on('end', () => {
  try {
    if (!input.trim()) {
      console.error('Error: No input received from stdin');
      process.exit(1);
    }
    
    const [context, book] = JSON.parse(input);
    
    if (!context || !book || !Array.isArray(book.items)) {
      console.error('Error: Invalid mdBook preprocessor input format');
      process.exit(1);
    }
    
    const playgroundBaseUrl = context.config?.preprocessor?.['playground-embed']?.base_url || '/playground';
    
    // Process each item in the book
    book.items = processItems(book.items, playgroundBaseUrl);
    
    // Output just the book object (not the array)
    console.log(JSON.stringify(book));
  } catch (error) {
    console.error('Error processing book:', error.message || error);
    console.error('Input was:', input.substring(0, 200) + (input.length > 200 ? '...' : ''));
    process.exit(1);
  }
});

function processItems(items, baseUrl) {
  if (!items || !Array.isArray(items)) {
    return items || [];
  }
  
  return items.map(item => {
    // Process Chapter items
    if (item && item.Chapter && item.Chapter.content) {
      item.Chapter.content = processMarkdown(item.Chapter.content, baseUrl);
    }
    
    // Recursively process sub_items
    if (item && item.Chapter && item.Chapter.sub_items) {
      item.Chapter.sub_items = processItems(item.Chapter.sub_items, baseUrl);
    }
    
    return item;
  });
}

// Helper function to escape HTML attributes
function escapeAttr(value) {
  if (!value) return '';
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

// Helper function to sanitize height values
function sanitizeHeight(value, fallback = '300px') {
  if (!value) return fallback;
  // Allow only valid CSS units for height
  return /^[0-9]+(px|em|rem|%|vh|vw)$/.test(value) ? value : fallback;
}

function processMarkdown(content, baseUrl) {
  // Replace PipTable code blocks with interactive examples
  return content.replace(/```(?:piptable|vba)\s*\n([\s\S]*?)\n```/g, (match, code) => {
    return createInteractiveExample(code.trim(), baseUrl);
  });
}

function createInteractiveExample(code, baseUrl) {
  // Extract configuration comments
  const titleMatch = code.match(/^\s*'\s*@title\s+(.+)$/m);
  const descMatch = code.match(/^\s*'\s*@description\s+(.+)$/m);
  const heightMatch = code.match(/^\s*'\s*@height\s+(.+)$/m);
  const readonlyMatch = code.match(/^\s*'\s*@readonly\s*$/m);
  const noOutputMatch = code.match(/^\s*'\s*@no-output\s*$/m);
  const staticMatch = code.match(/^\s*'\s*@static\s*$/m);
  
  // If marked as static, just return the original code block
  if (staticMatch) {
    const cleanCode = code
      .replace(/^\s*'\s*@\w+.*$/gm, '')
      .replace(/\n\n+/g, '\n\n')
      .trim();
    return `\`\`\`piptable\n${cleanCode}\n\`\`\``;
  }
  
  // Remove configuration comments from displayed code
  const cleanCode = code
    .replace(/^\s*'\s*@\w+.*$/gm, '')
    .replace(/\n\n+/g, '\n\n')
    .trim();
  
  // Build query parameters
  const params = new URLSearchParams();
  params.set('code', encodeURIComponent(cleanCode));
  
  // Sanitize and validate extracted values
  const height = sanitizeHeight(heightMatch?.[1], '300px');
  const title = titleMatch?.[1];
  const description = descMatch?.[1];
  const readonly = !!readonlyMatch;
  const showOutput = !noOutputMatch;
  
  if (height !== '200px') params.set('height', height);
  if (readonly) params.set('readonly', 'true');
  if (!showOutput) params.set('showOutput', 'false');
  if (title) params.set('title', title);
  if (description) params.set('description', description);
  
  const embedUrl = `${baseUrl}/embed.html?${params.toString()}`;
  const safeEmbedUrl = escapeAttr(embedUrl);
  const safeTitle = escapeAttr(title || 'PipTable Code Example');
  const safeHeight = escapeAttr(height);
  
  // Generate the iframe HTML with escaped attributes
  return `<div class="playground-embed" style="margin: 1.5em 0;">
  <iframe 
    src="${safeEmbedUrl}" 
    width="100%" 
    height="${safeHeight}"
    frameborder="0" 
    style="border: 1px solid #e5e7eb; border-radius: 8px; background: white;"
    title="${safeTitle}"
    sandbox="allow-scripts allow-same-origin"
    loading="lazy">
  </iframe>
</div>`;
}