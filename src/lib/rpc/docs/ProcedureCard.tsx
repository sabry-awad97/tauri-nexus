// =============================================================================
// ProcedureCard Component
// =============================================================================
// Displays a single RPC procedure with expandable details.

import type { ProcedureSchema } from './types';
import { TypeRenderer } from './TypeRenderer';
import { getTypeLabel, getTypeBadgeClass } from './utils';

export interface ProcedureCardProps {
  /** Procedure path (e.g., "user.get") */
  path: string;
  /** Procedure schema data */
  schema: ProcedureSchema;
  /** Whether the card is expanded */
  expanded: boolean;
  /** Callback when expand state changes */
  onToggle: () => void;
}

/**
 * ProcedureCard component for displaying a single procedure.
 */
export function ProcedureCard({
  path,
  schema,
  expanded,
  onToggle,
}: ProcedureCardProps): JSX.Element {
  const typeLabel = getTypeLabel(schema.procedure_type);
  const badgeClass = getTypeBadgeClass(schema.procedure_type);

  return (
    <div
      className={`procedure-card ${expanded ? 'expanded' : ''}`}
      data-testid={`procedure-${path}`}
    >
      <button
        className="procedure-header"
        onClick={onToggle}
        aria-expanded={expanded}
        type="button"
      >
        <div className="procedure-header-left">
          <span className={`procedure-badge ${badgeClass}`} data-testid="type-badge">
            {typeLabel}
          </span>
          <span className="procedure-path" data-testid="procedure-path">
            {path}
          </span>
          {schema.deprecated && (
            <span className="procedure-deprecated" data-testid="deprecated-indicator" title="Deprecated">
              ⚠️ Deprecated
            </span>
          )}
        </div>
        <div className="procedure-header-right">
          {schema.description && !expanded && (
            <span className="procedure-description-preview">
              {schema.description.length > 60
                ? `${schema.description.substring(0, 60)}...`
                : schema.description}
            </span>
          )}
          <span className="procedure-expand-icon">
            {expanded ? '▼' : '▶'}
          </span>
        </div>
      </button>

      {expanded && (
        <div className="procedure-details" data-testid="procedure-details">
          {schema.description && (
            <div className="procedure-description" data-testid="description">
              <p>{schema.description}</p>
            </div>
          )}

          {schema.tags.length > 0 && (
            <div className="procedure-tags" data-testid="tags">
              <span className="procedure-section-label">Tags:</span>
              {schema.tags.map((tag) => (
                <span key={tag} className="procedure-tag">
                  {tag}
                </span>
              ))}
            </div>
          )}

          {schema.input && (
            <div className="procedure-schema-section" data-testid="input-schema">
              <span className="procedure-section-label">Input:</span>
              <div className="procedure-schema-content">
                <TypeRenderer schema={schema.input} showExamples={true} />
              </div>
            </div>
          )}

          {schema.output && (
            <div className="procedure-schema-section" data-testid="output-schema">
              <span className="procedure-section-label">Output:</span>
              <div className="procedure-schema-content">
                <TypeRenderer schema={schema.output} showExamples={true} />
              </div>
            </div>
          )}

          {schema.metadata && (
            <div className="procedure-metadata" data-testid="metadata">
              <span className="procedure-section-label">Metadata:</span>
              <pre className="procedure-metadata-content">
                {JSON.stringify(schema.metadata, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default ProcedureCard;
