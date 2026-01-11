# @tauri-nexus/rpc-docs

> Auto-generated API documentation components for Tauri RPC. Interactive explorer with live testing.

[![npm version](https://img.shields.io/npm/v/@tauri-nexus/rpc-docs.svg)](https://www.npmjs.com/package/@tauri-nexus/rpc-docs)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## Features

- 📚 **Auto-generated docs** — Introspect your RPC backend and display all procedures
- 🧪 **Live testing** — Execute procedures directly from the documentation
- 🎨 **Beautiful UI** — Clean, modern design with syntax highlighting
- 🔍 **Search & filter** — Find procedures by name, type, or namespace
- 📝 **Type visualization** — Display input/output schemas with examples
- ⚡ **Zero config** — Works out of the box with your Tauri RPC backend

## Installation

```bash
npm install @tauri-nexus/rpc-docs
# or
pnpm add @tauri-nexus/rpc-docs
# or
bun add @tauri-nexus/rpc-docs
```

### Peer Dependencies

- `react` ^18.0.0 || ^19.0.0
- `@tanstack/react-query` ^5.0.0

## Quick Start

### Basic Usage

```tsx
import { ApiDocs } from "@tauri-nexus/rpc-docs";
import "@tauri-nexus/rpc-docs/styles.css";

function DocsPage() {
  return (
    <ApiDocs
      title="My API Documentation"
      description="Browse and test available RPC procedures"
    />
  );
}
```

That's it! The component will automatically fetch the schema from your Tauri backend and render interactive documentation.

## Components

### ApiDocs

The main documentation component that renders the full API explorer:

```tsx
import { ApiDocs } from "@tauri-nexus/rpc-docs";

<ApiDocs
  title="API Documentation" // Page title
  description="API description" // Subtitle text
  defaultExpanded={false} // Expand all procedures by default
  showTester={true} // Show the procedure tester panel
  groupBy="namespace" // Group by: "namespace" | "type" | "none"
/>;
```

### ProcedureCard

Display a single procedure with its details:

```tsx
import { ProcedureCard } from "@tauri-nexus/rpc-docs";

<ProcedureCard
  procedure={procedureSchema}
  expanded={false}
  onToggle={() => {}}
  onTest={(procedure) => {}}
/>;
```

### TypeRenderer

Render type schemas with syntax highlighting:

```tsx
import { TypeRenderer } from "@tauri-nexus/rpc-docs";

<TypeRenderer schema={typeSchema} depth={0} maxDepth={3} />;
```

### FilterBar

Search and filter procedures:

```tsx
import { FilterBar } from "@tauri-nexus/rpc-docs";

<FilterBar value={filterState} onChange={setFilterState} procedureCount={42} />;
```

### ProcedureTester

Interactive procedure testing panel:

```tsx
import { ProcedureTester } from "@tauri-nexus/rpc-docs";

<ProcedureTester procedure={selectedProcedure} onClose={() => {}} />;
```

### InputEditor

JSON input editor with validation:

```tsx
import { InputEditor } from "@tauri-nexus/rpc-docs";

<InputEditor
  value={inputJson}
  onChange={setInputJson}
  schema={inputSchema}
  placeholder="Enter JSON input..."
/>;
```

### ResponseViewer

Display RPC responses with formatting:

```tsx
import { ResponseViewer } from "@tauri-nexus/rpc-docs";

<ResponseViewer
  response={response}
  error={error}
  duration={123}
  isLoading={false}
/>;
```

## Hooks

### useRouterSchema

Fetch and parse the router schema from your backend:

```tsx
import { useRouterSchema } from "@tauri-nexus/rpc-docs";

function MyDocs() {
  const { schema, isLoading, error } = useRouterSchema();

  if (isLoading) return <div>Loading schema...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      <h1>Procedures: {schema.procedures.length}</h1>
      {schema.procedures.map((proc) => (
        <div key={proc.path}>{proc.path}</div>
      ))}
    </div>
  );
}
```

## Utilities

### groupProcedures

Group procedures by namespace:

```typescript
import { groupProcedures } from "@tauri-nexus/rpc-docs";

const groups = groupProcedures(procedures);
// => { user: [...], stream: [...], ... }
```

### filterProcedures

Filter procedures by search query and type:

```typescript
import { filterProcedures } from "@tauri-nexus/rpc-docs";

const filtered = filterProcedures(procedures, {
  search: "user",
  type: "query", // "query" | "mutation" | "subscription" | "all"
});
```

### generatePlaceholder

Generate placeholder input for a procedure:

```typescript
import { generatePlaceholder } from "@tauri-nexus/rpc-docs";

const placeholder = generatePlaceholder(procedure.inputSchema);
// => { id: 0, name: "" }
```

### generatePlaceholderJson

Generate placeholder as formatted JSON string:

```typescript
import { generatePlaceholderJson } from "@tauri-nexus/rpc-docs";

const json = generatePlaceholderJson(procedure.inputSchema);
// => '{\n  "id": 0,\n  "name": ""\n}'
```

## Types

```typescript
import type {
  RouterSchema, // Full router schema
  ProcedureSchema, // Single procedure schema
  TypeSchema, // Type definition schema
  ProcedureType, // "query" | "mutation" | "subscription"
  ProcedureGroup, // Grouped procedures
  FilterState, // Filter state
  FilterResult, // Filter result with counts
} from "@tauri-nexus/rpc-docs";
```

### RouterSchema

```typescript
interface RouterSchema {
  procedures: ProcedureSchema[];
  version?: string;
  description?: string;
}
```

### ProcedureSchema

```typescript
interface ProcedureSchema {
  path: string; // e.g., "user.get"
  type: ProcedureType; // "query" | "mutation" | "subscription"
  description?: string;
  inputSchema?: TypeSchema;
  outputSchema?: TypeSchema;
  deprecated?: boolean;
  tags?: string[];
}
```

### TypeSchema

```typescript
interface TypeSchema {
  type: string; // "object" | "array" | "string" | "number" | etc.
  properties?: Record<string, TypeSchema>;
  items?: TypeSchema;
  required?: string[];
  description?: string;
  example?: unknown;
  enum?: unknown[];
  nullable?: boolean;
}
```

## Styling

### Using the Default Styles

Import the included stylesheet:

```tsx
import "@tauri-nexus/rpc-docs/styles.css";
```

### Custom Styling

Override CSS variables for theming:

```css
:root {
  --rpc-docs-primary: #6366f1;
  --rpc-docs-background: #ffffff;
  --rpc-docs-surface: #f8fafc;
  --rpc-docs-border: #e2e8f0;
  --rpc-docs-text: #1e293b;
  --rpc-docs-text-muted: #64748b;
  --rpc-docs-success: #22c55e;
  --rpc-docs-error: #ef4444;
  --rpc-docs-warning: #f59e0b;
  --rpc-docs-query: #3b82f6;
  --rpc-docs-mutation: #8b5cf6;
  --rpc-docs-subscription: #22c55e;
}
```

### Component Classes

All components use BEM-style class names for easy customization:

```css
.rpc-docs {
}
.rpc-docs__header {
}
.rpc-docs__content {
}

.procedure-card {
}
.procedure-card--expanded {
}
.procedure-card__header {
}
.procedure-card__body {
}

.type-renderer {
}
.type-renderer__property {
}

.filter-bar {
}
.filter-bar__search {
}
.filter-bar__filters {
}

.procedure-tester {
}
.procedure-tester__input {
}
.procedure-tester__output {
}
```

## Backend Requirements

Your Tauri backend should expose a `rpc_schema` command that returns the router schema:

```rust
#[tauri::command]
fn rpc_schema() -> RouterSchema {
    RouterSchema {
        procedures: vec![
            ProcedureSchema {
                path: "user.get".to_string(),
                procedure_type: "query".to_string(),
                input_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "number" }
                    },
                    "required": ["id"]
                })),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "number" },
                        "name": { "type": "string" },
                        "email": { "type": "string" }
                    }
                })),
                ..Default::default()
            },
            // ... more procedures
        ],
        ..Default::default()
    }
}
```

## Example

Here's a complete example with custom configuration:

```tsx
import { useState } from "react";
import {
  ApiDocs,
  useRouterSchema,
  filterProcedures,
  type FilterState,
} from "@tauri-nexus/rpc-docs";
import "@tauri-nexus/rpc-docs/styles.css";

function CustomDocs() {
  const { schema, isLoading } = useRouterSchema();
  const [filter, setFilter] = useState<FilterState>({
    search: "",
    type: "all",
  });

  if (isLoading) {
    return <div className="loading">Loading API documentation...</div>;
  }

  const filtered = filterProcedures(schema?.procedures ?? [], filter);

  return (
    <div className="custom-docs">
      <header>
        <h1>🚀 My API</h1>
        <p>{filtered.length} procedures available</p>
      </header>

      <ApiDocs
        title="" // Hide default title
        showTester={true}
        groupBy="namespace"
      />
    </div>
  );
}
```

## Related Packages

- [`@tauri-nexus/rpc-core`](../rpc-core) — Core RPC client (framework-agnostic)
- [`@tauri-nexus/rpc-react`](../rpc-react) — React hooks and TanStack Query integration

## License

MIT © Tauri Nexus
