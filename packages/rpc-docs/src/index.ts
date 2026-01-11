// =============================================================================
// API Documentation Module
// =============================================================================
// Headless primitives for building API documentation UIs.

// =============================================================================
// Headless Primitives
// =============================================================================
export {
  // Provider & Context
  ApiDocsProvider,
  useApiDocsContext,
  type ApiDocsContextValue,
  type ApiDocsProviderProps,

  // Root Layout Components
  ApiDocsRoot,
  ApiDocsHeader,
  ApiDocsTitle,
  ApiDocsDescription,
  ApiDocsVersion,
  ApiDocsActions,
  ApiDocsContent,
  ApiDocsEmpty,
  ApiDocsLoading,
  ApiDocsError,
  type ApiDocsRootProps,

  // Filter Components
  ApiDocsSearch,
  ApiDocsTypeFilter,
  ApiDocsTypeFilterButton,
  ApiDocsCount,
  type ApiDocsSearchProps,
  type ApiDocsTypeFilterProps,
  type ApiDocsCountProps,

  // Procedure Components
  ApiDocsProcedureList,
  ApiDocsProcedureGroup,
  ApiDocsProcedureCard,
  ApiDocsProcedureHeader,
  ApiDocsProcedureBadge,
  ApiDocsProcedurePath,
  ApiDocsProcedureDeprecated,
  ApiDocsProcedureDescription,
  ApiDocsProcedureDetails,
  ApiDocsProcedureTags,
  ApiDocsProcedureSchema,
  useCurrentProcedure,
  type ApiDocsProcedureListProps,
  type ApiDocsProcedureGroupProps,
  type ApiDocsProcedureCardProps,

  // Tester Components
  ApiDocsTester,
  ApiDocsTesterInput,
  ApiDocsTesterExecute,
  ApiDocsTesterResponse,
  useApiDocsTester,
  useCurrentTester,
  type ApiDocsTesterProps,
  type ApiDocsTesterState,

  // Type Renderer
  ApiDocsTypeRenderer,
  type ApiDocsTypeRendererProps,
} from "./primitives";

// =============================================================================
// Hooks
// =============================================================================
export { useRouterSchema } from "./useRouterSchema";

// =============================================================================
// Utilities
// =============================================================================
export {
  groupProcedures,
  filterProcedures,
  generatePlaceholder,
  generatePlaceholderJson,
} from "./utils";

// =============================================================================
// Types
// =============================================================================
export type {
  RouterSchema,
  ProcedureSchema,
  TypeSchema,
  ProcedureType,
  ProcedureGroup,
  ProcedureEntry,
  FilterState,
  FilterResult,
} from "./types";
