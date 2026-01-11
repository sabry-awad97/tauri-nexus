// =============================================================================
// InputEditor Component
// =============================================================================
// A controlled textarea for JSON input with validation feedback.

import { JSX, useCallback, useId } from "react";

export interface InputEditorProps {
  /** Current input value */
  value: string;
  /** Change handler */
  onChange: (value: string) => void;
  /** Placeholder text */
  placeholder: string;
  /** Validation error message */
  error: string | null;
  /** Whether input is disabled */
  disabled?: boolean;
  /** Accessible label */
  label?: string;
}

/**
 * InputEditor component for entering JSON input.
 * Displays a monospace textarea with validation error feedback.
 */
export function InputEditor({
  value,
  onChange,
  placeholder,
  error,
  disabled = false,
  label = "Input JSON",
}: InputEditorProps): JSX.Element {
  const id = useId();
  const errorId = `${id}-error`;

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      onChange(e.target.value);
    },
    [onChange],
  );

  return (
    <div className="input-editor" data-testid="input-editor">
      <label htmlFor={id} className="input-editor-label">
        {label}
      </label>
      <textarea
        id={id}
        className={`input-editor-textarea ${error ? "input-editor-textarea-error" : ""}`}
        value={value}
        onChange={handleChange}
        placeholder={placeholder}
        disabled={disabled}
        spellCheck={false}
        aria-invalid={!!error}
        aria-describedby={error ? errorId : undefined}
        data-testid="input-editor-textarea"
      />
      {error && (
        <div
          id={errorId}
          className="input-editor-error"
          role="alert"
          data-testid="input-editor-error"
        >
          {error}
        </div>
      )}
    </div>
  );
}

export default InputEditor;
