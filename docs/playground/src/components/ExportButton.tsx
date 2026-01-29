import { useState } from 'preact/hooks';
import { exportData, type ExportFormat, type ExportData } from '../lib/export';
import { cn } from '../lib/utils';

interface ExportButtonProps {
  code: string;
  output: string;
  disabled?: boolean;
  className?: string;
}

export function ExportButton({ code, output, disabled = false, className }: ExportButtonProps) {
  const [isExportModalOpen, setExportModalOpen] = useState(false);
  const [exportStatus, setExportStatus] = useState<'idle' | 'exporting' | 'success' | 'error'>('idle');
  const [lastError, setLastError] = useState<string>('');

  const handleOpenModal = () => {
    if (!disabled) {
      setExportModalOpen(true);
    }
  };

  const handleExport = async (format: ExportFormat, customFilename?: string) => {
    setExportStatus('exporting');
    
    try {
      const data: ExportData = {
        code,
        output,
        timestamp: new Date(),
        filename: customFilename
      };
      
      exportData(data, format);
      
      setExportStatus('success');
      setTimeout(() => {
        setExportStatus('idle');
        setExportModalOpen(false);
      }, 1500);
    } catch (error) {
      console.error('Export failed:', error);
      setLastError(error instanceof Error ? error.message : 'Export failed');
      setExportStatus('error');
      setTimeout(() => setExportStatus('idle'), 3000);
    }
  };

  const hasOutput = output.trim().length > 0;

  return (
    <>
      <button
        onClick={handleOpenModal}
        disabled={disabled || !hasOutput}
        className={cn(
          "inline-flex items-center justify-center rounded-md text-sm font-medium",
          "h-9 px-3 hover:bg-accent hover:text-accent-foreground",
          "ring-offset-background transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
          "disabled:pointer-events-none disabled:opacity-50",
          className
        )}
        aria-label="Export results"
        title={hasOutput ? "Export results" : "Run code to enable export"}
      >
        <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} 
                d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        Export
      </button>

      {/* Export Modal */}
      {isExportModalOpen && (
        <ExportModal 
          code={code}
          output={output}
          exportStatus={exportStatus}
          lastError={lastError}
          onExport={handleExport}
          onClose={() => {
            setExportModalOpen(false);
            setExportStatus('idle');
            setLastError('');
          }}
        />
      )}
    </>
  );
}

interface ExportModalProps {
  code: string;
  output: string;
  exportStatus: 'idle' | 'exporting' | 'success' | 'error';
  lastError: string;
  onExport: (format: ExportFormat, customFilename?: string) => void;
  onClose: () => void;
}

function ExportModal({ code, output, exportStatus, lastError, onExport, onClose }: ExportModalProps) {
  const [selectedFormat, setSelectedFormat] = useState<ExportFormat>('pip');
  const [customFilename, setCustomFilename] = useState('');

  const formats: Array<{ value: ExportFormat; label: string; description: string }> = [
    { 
      value: 'pip', 
      label: 'PIP Script', 
      description: 'PipTable source code file for reimporting' 
    },
    { 
      value: 'json', 
      label: 'JSON', 
      description: 'Complete export with code, output, and structured data' 
    },
    { 
      value: 'csv', 
      label: 'CSV', 
      description: 'Table data only (requires table output from your code)' 
    },
    { 
      value: 'txt', 
      label: 'Text', 
      description: 'Plain text format with code and output' 
    }
  ];

  const handleExport = () => {
    onExport(selectedFormat, customFilename || undefined);
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-3 sm:p-4">
      <div className="bg-card border rounded-lg shadow-lg max-w-md w-full max-h-[95vh] sm:max-h-[90vh] overflow-auto mx-2 sm:mx-0">
        <div className="flex items-center justify-between p-3 sm:p-4 border-b">
          <h2 className="text-base sm:text-lg font-semibold">Export Results</h2>
          <button
            onClick={onClose}
            className="h-8 w-8 rounded-md hover:bg-accent hover:text-accent-foreground flex items-center justify-center touch-manipulation"
            aria-label="Close modal"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="p-3 sm:p-4 space-y-4">
          {/* Format Selection */}
          <div>
            <h3 className="text-sm font-medium mb-3">Export Format</h3>
            <div className="space-y-2">
              {formats.map((format) => (
                <label key={format.value} className="flex items-start gap-3 cursor-pointer">
                  <input
                    type="radio"
                    name="format"
                    value={format.value}
                    checked={selectedFormat === format.value}
                    onChange={(e) => setSelectedFormat(e.currentTarget.value as ExportFormat)}
                    className="mt-1"
                  />
                  <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium">{format.label}</div>
                    <div className="text-xs text-muted-foreground">{format.description}</div>
                  </div>
                </label>
              ))}
            </div>
          </div>

          {/* Custom Filename */}
          <div>
            <h3 className="text-sm font-medium mb-2">Filename (optional)</h3>
            <input
              type="text"
              value={customFilename}
              onChange={(e) => setCustomFilename(e.currentTarget.value)}
              placeholder="my-export"
              className="w-full px-3 py-2 text-sm bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <div className="text-xs text-muted-foreground mt-1">
              Leave empty for auto-generated filename
            </div>
          </div>

          {/* Export Status */}
          {exportStatus === 'success' && (
            <div className="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-md">
              <div className="flex items-center gap-2 text-green-800 dark:text-green-200">
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                </svg>
                Export completed successfully!
              </div>
            </div>
          )}

          {exportStatus === 'error' && (
            <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
              <div className="text-red-800 dark:text-red-200 text-sm">
                <div className="font-medium">Export failed</div>
                <div className="mt-1">{lastError}</div>
              </div>
            </div>
          )}
        </div>

        <div className="flex justify-end gap-2 p-3 sm:p-4 border-t">
          <button
            onClick={onClose}
            disabled={exportStatus === 'exporting'}
            className="px-4 py-2 text-sm font-medium rounded-md hover:bg-accent hover:text-accent-foreground transition-colors disabled:opacity-50 touch-manipulation"
          >
            Cancel
          </button>
          <button
            onClick={handleExport}
            disabled={exportStatus === 'exporting'}
            className={cn(
              "px-4 py-2 text-sm font-medium rounded-md transition-colors touch-manipulation",
              "bg-primary text-primary-foreground hover:bg-primary/90",
              "focus:outline-none focus:ring-2 focus:ring-ring",
              "disabled:pointer-events-none disabled:opacity-50"
            )}
          >
            {exportStatus === 'exporting' ? (
              <div className="flex items-center gap-2">
                <div className="h-3 w-3 animate-spin rounded-full border border-current border-t-transparent" />
                Exporting...
              </div>
            ) : (
              'Export'
            )}
          </button>
        </div>
      </div>
    </div>
  );
}