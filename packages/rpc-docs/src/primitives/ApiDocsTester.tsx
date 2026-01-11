// =============================================================================
// ApiDocsTester - Procedure Testing Components
// =============================================================================

import {
  type HTMLAttributes,
  type TextareaHTMLAttributes,
  type ButtonHTMLAttributes,
  forwardRef,
  createContext,
  useContext,
  useState,
  useCallback,
  useMemo,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { generatePlaceholderJson } from "../utils";
import type { TypeSchema } from "../types";

export interface ApiDocsTesterState {
  input: string;
  parseError: string | null;
  isLoading: boolean;
  response: unknown | null;
  error: string | null;
  executionTime: number | null;
}

interface TesterContextValue extends ApiDocsTesterState {
  setInput: (value: string) => void;
  execute: () => Promise<void>;
  reset: () => void;
  placeholder: string;
  isValid: boolean;
  path: string;
}

const TesterContext = createContext<TesterContextValue | null>(null);

function useTesterContext() {
  const context = useContext(TesterContext);
  if (!context) {
    throw new Error("Tester components must be used within ApiDocsTester");
  }
  return context;
}

/** Hook for using tester state outside of context */
export function useApiDocsTester(path: string, inputSchema?: TypeSchema) {
  const placeholder = useMemo(
    () => generatePlaceholderJson(inputSchema),
    [inputSchema],
  );

  const [state, setState] = useState<ApiDocsTesterState>({
    input: placeholder,
    parseError: null,
    isLoading: false,
    response: null,
    error: null,
    executionTime: null,
  });

  const setInput = useCallback((value: string) => {
    let parseError: string | null = null;
    if (value.trim()) {
      try {
        JSON.parse(value);
      } catch (e) {
        parseError = e instanceof Error ? e.message : "Invalid JSON";
      }
    }
    setState((prev) => ({ ...prev, input: value, parseError }));
  }, []);

  const execute = useCallback(async () => {
    let parsedInput: unknown;
    try {
      parsedInput = state.input.trim() ? JSON.parse(state.input) : {};
    } catch {
      return;
    }

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

  const reset = useCallback(() => {
    setState({
      input: placeholder,
      parseError: null,
      isLoading: false,
      response: null,
      error: null,
      executionTime: null,
    });
  }, [placeholder]);

  return {
    ...state,
    setInput,
    execute,
    reset,
    placeholder,
    isValid: !state.parseError,
  };
}

export interface ApiDocsTesterProps extends HTMLAttributes<HTMLDivElement> {
  /** Procedure path */
  path: string;
  /** Input schema for placeholder generation */
  inputSchema?: TypeSchema;
}

/** Tester container with context provider */
export const ApiDocsTester = forwardRef<HTMLDivElement, ApiDocsTesterProps>(
  ({ path, inputSchema, className, children, ...props }, ref) => {
    const tester = useApiDocsTester(path, inputSchema);

    const contextValue: TesterContextValue = {
      ...tester,
      path,
    };

    return (
      <TesterContext.Provider value={contextValue}>
        <div
          ref={ref}
          className={className}
          data-testid="api-docs-tester"
          {...props}
        >
          {children}
        </div>
      </TesterContext.Provider>
    );
  },
);
ApiDocsTester.displayName = "ApiDocsTester";

export interface ApiDocsTesterInputProps extends Omit<
  TextareaHTMLAttributes<HTMLTextAreaElement>,
  "value" | "onChange"
> {
  /** Error class name */
  errorClassName?: string;
  /** Show error message below input */
  showError?: boolean;
  /** Custom error renderer */
  renderError?: (error: string) => ReactNode;
}

/** JSON input textarea */
export const ApiDocsTesterInput = forwardRef<
  HTMLTextAreaElement,
  ApiDocsTesterInputProps
>(
  (
    { errorClassName, showError = true, renderError, className, ...props },
    ref,
  ) => {
    const { input, setInput, parseError, isLoading, placeholder } =
      useTesterContext();

    return (
      <div data-testid="api-docs-tester-input-wrapper">
        <textarea
          ref={ref}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={placeholder}
          disabled={isLoading}
          spellCheck={false}
          aria-invalid={!!parseError}
          className={`${className ?? ""} ${parseError && errorClassName ? errorClassName : ""}`.trim()}
          data-testid="api-docs-tester-input"
          {...props}
        />
        {showError && parseError && (
          <div data-testid="api-docs-tester-input-error" role="alert">
            {renderError ? renderError(parseError) : parseError}
          </div>
        )}
      </div>
    );
  },
);
ApiDocsTesterInput.displayName = "ApiDocsTesterInput";

export interface ApiDocsTesterExecuteProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Loading content */
  loadingContent?: ReactNode;
}

/** Execute button */
export const ApiDocsTesterExecute = forwardRef<
  HTMLButtonElement,
  ApiDocsTesterExecuteProps
>(({ loadingContent, className, children, ...props }, ref) => {
  const { execute, isLoading, isValid } = useTesterContext();

  return (
    <button
      ref={ref}
      type="button"
      onClick={execute}
      disabled={isLoading || !isValid}
      className={className}
      data-testid="api-docs-tester-execute"
      {...props}
    >
      {isLoading ? (loadingContent ?? "Executing...") : (children ?? "Execute")}
    </button>
  );
});
ApiDocsTesterExecute.displayName = "ApiDocsTesterExecute";

export interface ApiDocsTesterResponseProps extends HTMLAttributes<HTMLDivElement> {
  /** Render prop for custom response display */
  render?: (props: {
    response: unknown | null;
    error: string | null;
    executionTime: number | null;
    isLoading: boolean;
  }) => ReactNode;
}

/** Response display */
export const ApiDocsTesterResponse = forwardRef<
  HTMLDivElement,
  ApiDocsTesterResponseProps
>(({ render, className, children, ...props }, ref) => {
  const { response, error, executionTime, isLoading } = useTesterContext();

  // Don't render if nothing to show
  if (!isLoading && response === null && error === null) {
    return null;
  }

  if (render) {
    return (
      <div
        ref={ref}
        className={className}
        data-testid="api-docs-tester-response"
        {...props}
      >
        {render({ response, error, executionTime, isLoading })}
      </div>
    );
  }

  return (
    <div
      ref={ref}
      className={className}
      data-testid="api-docs-tester-response"
      {...props}
    >
      {executionTime !== null && <span>{executionTime}ms</span>}
      {isLoading && <span>Executing...</span>}
      {!isLoading && error && <span data-error="true">{error}</span>}
      {!isLoading && response !== null && !error && (
        <pre>
          <code>{JSON.stringify(response, null, 2)}</code>
        </pre>
      )}
      {children}
    </div>
  );
});
ApiDocsTesterResponse.displayName = "ApiDocsTesterResponse";

/** Hook to access tester context */
export function useCurrentTester() {
  return useTesterContext();
}
