// =============================================================================
// API Documentation Module
// =============================================================================
// Exports for the OpenAPI-like documentation component.

// Components
export { ApiDocs, type ApiDocsProps } from './ApiDocs';
export { ProcedureCard, type ProcedureCardProps } from './ProcedureCard';
export { TypeRenderer, type TypeRendererProps } from './TypeRenderer';
export { FilterBar, type FilterBarProps } from './FilterBar';

// Hooks
export { useRouterSchema } from './useRouterSchema';

// Utilities
export { groupProcedures, filterProcedures } from './utils';

// Types
export type {
  RouterSchema,
  ProcedureSchema,
  TypeSchema,
  ProcedureType,
  ProcedureGroup,
  FilterState,
  FilterResult,
} from './types';
