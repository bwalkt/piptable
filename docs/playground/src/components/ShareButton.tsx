import { useState } from 'preact/hooks';
import { signal } from '@preact/signals';
import { generateShareURL, copyToClipboard, generateCodeDescription, type ShareableState } from '../lib/share';
import { cn } from '../lib/utils';

interface ShareButtonProps {
  getState: () => ShareableState;
  className?: string;
}

const isShareModalOpen = signal(false);

export function ShareButton({ getState, className }: ShareButtonProps) {
  const [copyStatus, setCopyStatus] = useState<'idle' | 'copying' | 'success' | 'error'>('idle');

  const handleShare = async () => {
    try {
      const currentState = getState();
      const shareURL = generateShareURL(currentState);
      const success = await copyToClipboard(shareURL);
      
      if (success) {
        setCopyStatus('success');
        setTimeout(() => setCopyStatus('idle'), 2000);
      } else {
        setCopyStatus('error');
        setTimeout(() => setCopyStatus('idle'), 3000);
      }
    } catch (error) {
      console.error('Failed to generate share URL:', error);
      setCopyStatus('error');
      setTimeout(() => setCopyStatus('idle'), 3000);
    }
  };

  const handleOpenModal = () => {
    isShareModalOpen.value = true;
  };

  return (
    <>
      <button
        onClick={handleOpenModal}
        className={cn(
          "inline-flex items-center justify-center rounded-md text-sm font-medium",
          "h-9 px-3 hover:bg-accent hover:text-accent-foreground",
          "ring-offset-background transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
          className
        )}
        aria-label="Share code"
        title="Share code"
      >
        <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} 
                d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.367 2.684 3 3 0 00-5.367-2.684z" />
        </svg>
        Share
      </button>

      {/* Share Modal */}
      {isShareModalOpen.value && (
        <ShareModal 
          getState={getState} 
          copyStatus={copyStatus} 
          onShare={handleShare}
          onClose={() => isShareModalOpen.value = false}
        />
      )}
    </>
  );
}

interface ShareModalProps {
  getState: () => ShareableState;
  copyStatus: 'idle' | 'copying' | 'success' | 'error';
  onShare: () => void;
  onClose: () => void;
}

function ShareModal({ getState, copyStatus, onShare, onClose }: ShareModalProps) {
  const state = getState();
  let shareURL = '';
  try {
    shareURL = generateShareURL(state);
  } catch (error) {
    console.error('Failed to generate share URL for modal:', error);
  }

  const codeDescription = generateCodeDescription(state.code);

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-3 sm:p-4">
      <div className="bg-card border rounded-lg shadow-lg max-w-lg w-full max-h-[95vh] sm:max-h-[90vh] overflow-auto mx-2 sm:mx-0">
        <div className="flex items-center justify-between p-3 sm:p-4 border-b">
          <h2 className="text-base sm:text-lg font-semibold">Share Code</h2>
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
          <div>
            <h3 className="text-sm font-medium mb-2">Code Preview</h3>
            <div className="p-3 bg-muted rounded-md text-sm">
              <div className="font-medium text-foreground">{codeDescription}</div>
              <div className="text-muted-foreground mt-1">
                {state.code.split('\n').length} lines • {state.theme || 'dark'} theme
              </div>
            </div>
          </div>

          <div>
            <h3 className="text-sm font-medium mb-2">Shareable URL</h3>
            <div className="flex gap-2">
              <input
                type="text"
                value={shareURL}
                readOnly
                className="flex-1 px-3 py-2 text-sm bg-background border rounded-md focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="Generating URL..."
              />
              <button
                onClick={onShare}
                disabled={copyStatus === 'copying' || !shareURL}
                className={cn(
                  "px-3 py-2 text-sm font-medium rounded-md transition-colors",
                  "focus:outline-none focus:ring-2 focus:ring-ring",
                  copyStatus === 'success' 
                    ? "bg-green-600 text-white" 
                    : copyStatus === 'error'
                    ? "bg-red-600 text-white"
                    : "bg-primary text-primary-foreground hover:bg-primary/90",
                  "disabled:pointer-events-none disabled:opacity-50"
                )}
              >
                {copyStatus === 'copying' ? (
                  <div className="flex items-center gap-1">
                    <div className="h-3 w-3 animate-spin rounded-full border border-current border-t-transparent" />
                    Copying...
                  </div>
                ) : copyStatus === 'success' ? (
                  <div className="flex items-center gap-1">
                    <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fillRule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clipRule="evenodd" />
                    </svg>
                    Copied!
                  </div>
                ) : copyStatus === 'error' ? (
                  'Error'
                ) : (
                  'Copy'
                )}
              </button>
            </div>
            
            {copyStatus === 'success' && (
              <div className="mt-2 text-sm text-green-600 dark:text-green-400">
                URL copied to clipboard! You can now share it with others.
              </div>
            )}
            
            {copyStatus === 'error' && (
              <div className="mt-2 text-sm text-red-600 dark:text-red-400">
                Failed to copy URL. Please select and copy manually.
              </div>
            )}
          </div>

          <div className="text-sm text-muted-foreground">
            <p>
              This URL contains your code and settings. Anyone with this link can view and edit the code.
            </p>
          </div>
        </div>

        <div className="flex justify-end gap-2 p-3 sm:p-4 border-t">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium rounded-md hover:bg-accent hover:text-accent-foreground transition-colors touch-manipulation"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}