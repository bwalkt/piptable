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

  // Get configuration from URL parameters
  const urlParams = new URLSearchParams(window.location.search);
  const config: EmbedConfig = {
    code: decodeURIComponent(urlParams.get('code') || ''),
    height: urlParams.get('height') || '200px',
    readonly: urlParams.get('readonly') === 'true',
    showOutput: urlParams.get('showOutput') !== 'false', // default true
    title: urlParams.get('title') || undefined,
    description: urlParams.get('description') || undefined,
  };

  // If no code in URL, try to get from data attributes
  if (!config.code) {
    config.code = container.getAttribute('data-code') || 'PRINT "Hello, World!"';
    config.height = container.getAttribute('data-height') || config.height;
    config.readonly = container.getAttribute('data-readonly') === 'true';
    config.showOutput = container.getAttribute('data-show-output') !== 'false';
    config.title = container.getAttribute('data-title') || undefined;
    config.description = container.getAttribute('data-description') || undefined;
  }

  // Listen for configuration updates via postMessage (for dynamic examples)
  window.addEventListener('message', (event) => {
    if (event.data.type === 'UPDATE_PLAYGROUND_CONFIG') {
      const newConfig = { ...config, ...event.data.config };
      renderPlayground(newConfig);
    }
  });

  // Initial render
  renderPlayground(config);

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
      container
    );
  }

  // Notify parent window that playground is ready
  if (window.parent !== window) {
    window.parent.postMessage({ type: 'PLAYGROUND_READY' }, '*');
  }
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initializeEmbedPlayground);
} else {
  initializeEmbedPlayground();
}