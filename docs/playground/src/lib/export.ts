export type ExportFormat = 'csv' | 'json' | 'txt';

export interface ExportData {
  code: string;
  output: string;
  timestamp: Date;
  filename?: string;
}

// Convert HTML output to plain text for export
export function htmlToText(html: string): string {
  // Create a temporary element to parse HTML
  const temp = document.createElement('div');
  temp.innerHTML = html;
  
  // Remove script and style elements
  const scripts = temp.querySelectorAll('script, style');
  scripts.forEach(script => script.remove());
  
  // Get text content and clean it up
  let text = temp.textContent || temp.innerText || '';
  
  // Clean up whitespace
  text = text.replace(/\s+/g, ' ').trim();
  
  return text;
}

// Extract structured data from playground output
export function extractStructuredData(output: string): any[] {
  try {
    // Look for table-like data in the output
    const temp = document.createElement('div');
    temp.innerHTML = output;
    
    // Try to find tables
    const tables = temp.querySelectorAll('table');
    if (tables.length > 0) {
      const tableData: any[] = [];
      tables.forEach(table => {
        const rows = table.querySelectorAll('tr');
        const headers: string[] = [];
        const data: any[] = [];
        
        rows.forEach((row, index) => {
          const cells = row.querySelectorAll('td, th');
          const rowData: any = {};
          
          if (index === 0) {
            // Header row
            cells.forEach(cell => {
              headers.push(cell.textContent?.trim() || '');
            });
          } else {
            // Data row
            cells.forEach((cell, cellIndex) => {
              const header = headers[cellIndex] || `column_${cellIndex}`;
              const value = cell.textContent?.trim() || '';
              
              // Try to parse numbers and booleans
              if (!isNaN(Number(value)) && value !== '') {
                rowData[header] = Number(value);
              } else if (value.toLowerCase() === 'true') {
                rowData[header] = true;
              } else if (value.toLowerCase() === 'false') {
                rowData[header] = false;
              } else {
                rowData[header] = value;
              }
            });
            
            if (Object.keys(rowData).length > 0) {
              data.push(rowData);
            }
          }
        });
        
        if (data.length > 0) {
          tableData.push(...data);
        }
      });
      
      if (tableData.length > 0) {
        return tableData;
      }
    }
    
    // If no tables, try to extract any JSON-like data
    const jsonMatches = output.match(/\{[^}]*\}/g);
    if (jsonMatches) {
      const jsonData: any[] = [];
      jsonMatches.forEach(match => {
        try {
          const parsed = JSON.parse(match);
          jsonData.push(parsed);
        } catch {
          // Ignore invalid JSON
        }
      });
      
      if (jsonData.length > 0) {
        return jsonData;
      }
    }
    
    return [];
  } catch (error) {
    console.error('Failed to extract structured data:', error);
    return [];
  }
}

// Generate CSV content from data
export function generateCSV(data: any[]): string {
  if (data.length === 0) {
    return '';
  }
  
  // Get all unique keys from all objects
  const allKeys = Array.from(new Set(data.flatMap(Object.keys)));
  
  // Create header row
  const csvRows: string[] = [allKeys.map(key => `"${key}"`).join(',')];
  
  // Create data rows
  data.forEach(item => {
    const row = allKeys.map(key => {
      const value = item[key] ?? '';
      const stringValue = String(value);
      
      // Escape quotes and wrap in quotes if necessary
      if (stringValue.includes(',') || stringValue.includes('"') || stringValue.includes('\n')) {
        return `"${stringValue.replace(/"/g, '""')}"`;
      }
      
      return stringValue;
    });
    
    csvRows.push(row.join(','));
  });
  
  return csvRows.join('\n');
}

// Generate JSON content from data
export function generateJSON(data: ExportData): string {
  const exportObject = {
    metadata: {
      timestamp: data.timestamp.toISOString(),
      filename: data.filename || 'playground-export',
      version: '1.0'
    },
    code: data.code,
    output: {
      html: data.output,
      text: htmlToText(data.output)
    },
    structuredData: extractStructuredData(data.output)
  };
  
  return JSON.stringify(exportObject, null, 2);
}

// Generate plain text content
export function generateText(data: ExportData): string {
  const lines: string[] = [];
  
  lines.push('PipTable Playground Export');
  lines.push('='.repeat(30));
  lines.push('');
  lines.push(`Timestamp: ${data.timestamp.toISOString()}`);
  lines.push('');
  lines.push('Code:');
  lines.push('-'.repeat(10));
  lines.push(data.code);
  lines.push('');
  lines.push('Output:');
  lines.push('-'.repeat(10));
  lines.push(htmlToText(data.output));
  
  return lines.join('\n');
}

// Generate filename with timestamp
export function generateFilename(format: ExportFormat, customName?: string): string {
  const timestamp = new Date().toISOString().split('T')[0]; // YYYY-MM-DD
  const baseName = customName || `piptable-playground-${timestamp}`;
  
  return `${baseName}.${format}`;
}

// Download file with given content
export function downloadFile(content: string, filename: string, mimeType: string): void {
  try {
    const blob = new Blob([content], { type: mimeType });
    const url = URL.createObjectURL(blob);
    
    // Create temporary download link
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    link.style.display = 'none';
    
    // Trigger download
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    
    // Clean up
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  } catch (error) {
    console.error('Failed to download file:', error);
    throw new Error('Failed to download file. Please try again.');
  }
}

// Export data in specified format
export function exportData(data: ExportData, format: ExportFormat): void {
  let content: string;
  let mimeType: string;
  let filename: string;
  
  switch (format) {
    case 'csv':
      const structuredData = extractStructuredData(data.output);
      if (structuredData.length === 0) {
        throw new Error('No tabular data found in output. Please run code that generates table results.');
      }
      content = generateCSV(structuredData);
      mimeType = 'text/csv;charset=utf-8;';
      filename = generateFilename('csv', data.filename);
      break;
      
    case 'json':
      content = generateJSON(data);
      mimeType = 'application/json;charset=utf-8;';
      filename = generateFilename('json', data.filename);
      break;
      
    case 'txt':
      content = generateText(data);
      mimeType = 'text/plain;charset=utf-8;';
      filename = generateFilename('txt', data.filename);
      break;
      
    default:
      throw new Error(`Unsupported export format: ${format}`);
  }
  
  if (!content.trim()) {
    throw new Error('No content to export. Please run some code first.');
  }
  
  downloadFile(content, filename, mimeType);
}