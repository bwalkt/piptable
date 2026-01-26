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
    const book = JSON.parse(input);
    const playgroundBaseUrl = book.config?.preprocessor?.['playground-embed']?.base_url || '/playground';
    
    // Process each section
    book.sections = processSections(book.sections, playgroundBaseUrl);
    
    // Output the modified book
    console.log(JSON.stringify(book));
  } catch (error) {
    console.error('Error processing book:', error);
    process.exit(1);
  }
});

function processSections(sections, baseUrl) {
  return sections.map(section => {
    if (section.Chapter && section.Chapter.content) {
      section.Chapter.content = processMarkdown(section.Chapter.content, baseUrl);
    }
    
    if (section.Chapter && section.Chapter.sub_items) {
      section.Chapter.sub_items = processSections(section.Chapter.sub_items, baseUrl);
    }
    
    return section;
  });
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
  
  const height = heightMatch?.[1] || '300px';
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
  
  // Generate the iframe HTML
  return `<div class="playground-embed" style="margin: 1.5em 0;">
  <iframe 
    src="${embedUrl}" 
    width="100%" 
    height="${height}"
    frameborder="0" 
    style="border: 1px solid #e5e7eb; border-radius: 8px; background: white;"
    title="${title || 'PipTable Code Example'}"
    sandbox="allow-scripts allow-same-origin"
    loading="lazy">
  </iframe>
</div>`;
}