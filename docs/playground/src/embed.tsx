import { render } from 'preact';
import { EmbedPlayground } from './components/EmbedPlayground';
import './styles.css';

interface EmbedConfig {
  code: string;
  height?: string;
  readonly?: boolean;
  showOutput?: boolean;
  title?: string;
  description?: string;
}

// Function to initialize embedded playground from URL parameters or postMessage
function initializeEmbedPlayground() {
  const container = document.getElementById('embed-root');
  if (!container) {
    console.error('Embed root element not found');
    return;
  }
  const root = container;

  // Get configuration from URL parameters
  const urlParams = new URLSearchParams(window.location.search);
  let currentConfig: EmbedConfig = {
    code: urlParams.get('code') || '',
    height: urlParams.get('height') || '200px',
    readonly: urlParams.get('readonly') === 'true',
    showOutput: urlParams.get('showOutput') !== 'false', // default true
    title: urlParams.get('title') || undefined,
    description: urlParams.get('description') || undefined,
  };

  // Get allowed origin for postMessage security
  const allowedOrigin =
    urlParams.get('origin') || root.getAttribute('data-origin') || undefined;

  // If no code in URL, try to get from data attributes
  if (!currentConfig.code) {
    currentConfig.code = root.getAttribute('data-code') || 'PRINT "Hello, World!"';
    currentConfig.height = container.getAttribute('data-height') || currentConfig.height;
    currentConfig.readonly = container.getAttribute('data-readonly') === 'true';
    currentConfig.showOutput = container.getAttribute('data-show-output') !== 'false';
    currentConfig.title = container.getAttribute('data-title') || undefined;
    currentConfig.description = container.getAttribute('data-description') || undefined;
  }

  // Listen for configuration updates via postMessage (for dynamic examples)
  window.addEventListener('message', (event) => {
    // Validate message source
    if (event.source !== window.parent) return;
    
    // Validate origin if specified
    if (allowedOrigin && event.origin !== allowedOrigin) return;
    
    // Check message structure
    if (event.data?.type === 'UPDATE_PLAYGROUND_CONFIG') {
      currentConfig = { ...currentConfig, ...event.data.config };
      renderPlayground(currentConfig);
    }
  });

  // Initial render
  renderPlayground(currentConfig);

  function renderPlayground(playgroundConfig: EmbedConfig) {
    render(
      <EmbedPlayground
        initialCode={playgroundConfig.code}
        height={playgroundConfig.height}
        readonly={playgroundConfig.readonly}
        showOutput={playgroundConfig.showOutput}
        title={playgroundConfig.title}
        description={playgroundConfig.description}
      />,
      root
    );
  }

  // Notify parent window that playground is ready
  if (window.parent !== window) {
    window.parent.postMessage(
      { type: 'PLAYGROUND_READY' }, 
      allowedOrigin ?? '*'
    );
  }
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeEmbedPlayground);
} else {
  initializeEmbedPlayground();
}
