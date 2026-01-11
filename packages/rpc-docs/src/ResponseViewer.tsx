// =============================================================================
// ResponseViewer Component
// =============================================================================
// Displays procedure response or error with execution time.

import { JSX } from "react";

export interface ResponseViewerProps {
  /** Response data (success case) */
  response: unknown | null;
  /** Error message (failure case) */
  error: string | null;
  /** Execution time in milliseconds */
  executionTime: number | null;
  /** Whether currently loading */
  isLoading: boolean;
}

/**
 * ResponseViewer component for displaying procedure results.
 * Shows JSON response on success, error message on failure.
 */
export function ResponseViewer({
  response,
  error,
  executionTime,
  isLoading,
}: ResponseViewerProps): JSX.Element | null {
  // Don't render if no response, error, or loading state
  if (!isLoading && response === null && error === null) {
    return null;
  }

  return (
    <div className="response-viewer" data-testid="response-viewer">
      <div className="response-viewer-header">
        <span className="response-viewer-title">Response</span>
        {executionTime !== null && (
          <span
            className="response-viewer-time"
            data-testid="response-viewer-time"
          >
            {executionTime}ms
          </span>
        )}
      </div>

      {isLoading && (
        <div
          className="response-viewer-loading"
          data-testid="response-viewer-loading"
        >
          <div className="response-viewer-spinner" />
          <span>Executing...</span>
        </div>
      )}

      {!isLoading && error && (
        <div
          className="response-viewer-error"
          data-testid="response-viewer-error"
        >
          <span className="response-viewer-error-icon">✕</span>
          <span className="response-viewer-error-text">{error}</span>
        </div>
      )}

      {!isLoading && response !== null && !error && (
        <pre
          className="response-viewer-content"
          data-testid="response-viewer-content"
        >
          <code>{JSON.stringify(response, null, 2)}</code>
        </pre>
      )}
    </div>
  );
}

export default ResponseViewer;
