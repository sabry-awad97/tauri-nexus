// =============================================================================
// ProcedureTester Component
// =============================================================================
// Interactive component for testing RPC procedures with JSON input.

import { useState, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { InputEditor } from "./InputEditor";
import { ResponseViewer } from "./ResponseViewer";
import { generatePlaceholderJson } from "./utils";
import type { TypeSchema } from "./types";

export interface ProcedureTesterProps {
  /** Procedure path for RPC call */
  path: string;
  /** Input schema for placeholder generation */
  inputSchema?: TypeSchema;
}

interface TesterState {
  input: string;
  parseError: string | null;
  isLoading: boolean;
  response: unknown | null;
  error: string | null;
  executionTime: number | null;
}

/**
 * ProcedureTester component for testing RPC procedures.
 * Provides JSON input editor and displays response/error.
 */
export function ProcedureTester({
  path,
  inputSchema,
}: ProcedureTesterProps): JSX.Element {
  const placeholder = useMemo(
    () => generatePlaceholderJson(inputSchema),
    [inputSchema],
  );

  const [state, setState] = useState<TesterState>({
    input: placeholder,
    parseError: null,
    isLoading: false,
    response: null,
    error: null,
    executionTime: null,
  });

  // Validate JSON on input change
  const handleInputChange = useCallback((value: string) => {
    let parseError: string | null = null;

    if (value.trim()) {
      try {
        JSON.parse(value);
      } catch (e) {
        parseError = e instanceof Error ? e.message : "Invalid JSON";
      }
    }

    setState((prev) => ({
      ...prev,
      input: value,
      parseError,
    }));
  }, []);

  // Execute RPC call
  const handleExecute = useCallback(async () => {
    // Parse input
    let parsedInput: unknown;
    try {
      parsedInput = state.input.trim() ? JSON.parse(state.input) : {};
    } catch {
      return; // Should not happen if button is properly disabled
    }

    // Clear previous response and start loading
    setState((prev) => ({
      ...prev,
      isLoading: true,
      response: null,
      error: null,
      executionTime: null,
    }));

    const startTime = performance.now();

    try {
      const result = await invoke<unknown>("plugin:rpc|rpc_call", {
        path,
        input: parsedInput,
      });

      const executionTime = Math.round(performance.now() - startTime);

      setState((prev) => ({
        ...prev,
        isLoading: false,
        response: result,
        error: null,
        executionTime,
      }));
    } catch (e) {
      const executionTime = Math.round(performance.now() - startTime);
      const errorMessage = e instanceof Error ? e.message : String(e);

      setState((prev) => ({
        ...prev,
        isLoading: false,
        response: null,
        error: errorMessage,
        executionTime,
      }));
    }
  }, [path, state.input]);

  const isDisabled = state.isLoading || !!state.parseError;

  return (
    <div className="procedure-tester" data-testid="procedure-tester">
      <div className="procedure-tester-header">
        <span className="procedure-tester-title">Try It</span>
      </div>

      <InputEditor
        value={state.input}
        onChange={handleInputChange}
        placeholder={placeholder}
        error={state.parseError}
        disabled={state.isLoading}
        label="Input"
      />

      <div className="procedure-tester-actions">
        <button
          type="button"
          className="procedure-tester-btn"
          onClick={handleExecute}
          disabled={isDisabled}
          data-testid="procedure-tester-execute"
        >
          {state.isLoading ? (
            <>
              <span className="procedure-tester-spinner" />
              Executing...
            </>
          ) : (
            "Execute"
          )}
        </button>
      </div>

      <ResponseViewer
        response={state.response}
        error={state.error}
        executionTime={state.executionTime}
        isLoading={state.isLoading}
      />
    </div>
  );
}

export default ProcedureTester;
