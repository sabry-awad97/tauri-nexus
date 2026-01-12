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
```

## Quick Start

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

---

## Components

### ApiDocs

The main documentation component:

```tsx
<ApiDocs
  title="API Documentation"
  description="API description"
  defaultExpanded={false}
  showTester={true}
  groupBy="namespace" // "namespace" | "type" | "none"
/>
```

### ProcedureCard

Display a single procedure:

```tsx
<ProcedureCard
  procedure={procedureSchema}
  expanded={false}
  onToggle={() => {}}
  onTest={(procedure) => {}}
/>
```

### TypeRenderer

Render type schemas:

```tsx
<TypeRenderer schema={typeSchema} depth={0} maxDepth={3} />
```

### FilterBar

Search and filter:

```tsx
<FilterBar value={filterState} onChange={setFilterState} procedureCount={42} />
```

### ProcedureTester

Interactive testing panel:

```tsx
<ProcedureTester procedure={selectedProcedure} onClose={() => {}} />
```

### InputEditor

JSON input editor:

```tsx
<InputEditor
  value={inputJson}
  onChange={setInputJson}
  schema={inputSchema}
  placeholder="Enter JSON input..."
/>
```

### ResponseViewer

Display RPC responses:

```tsx
<ResponseViewer
  response={response}
  error={error}
  duration={123}
  isLoading={false}
/>
```

---

## Hooks

### useRouterSchema

Fetch the router schema from your backend:

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

---

## Utilities

```typescript
import {
  groupProcedures,
  filterProcedures,
  generatePlaceholder,
  generatePlaceholderJson,
} from "@tauri-nexus/rpc-docs";

// Group by namespace
const groups = groupProcedures(procedures);
// => { user: [...], stream: [...] }

// Filter procedures
const filtered = filterProcedures(procedures, {
  search: "user",
  type: "query",
});

// Generate placeholder input
const placeholder = generatePlaceholder(procedure.inputSchema);
// => { id: 0, name: "" }
```

---

## Types

```typescript
interface RouterSchema {
  procedures: ProcedureSchema[];
  version?: string;
  description?: string;
}

interface ProcedureSchema {
  path: string;
  type: "query" | "mutation" | "subscription";
  description?: string;
  inputSchema?: TypeSchema;
  outputSchema?: TypeSchema;
  deprecated?: boolean;
  tags?: string[];
}

interface TypeSchema {
  type: string;
  properties?: Record<string, TypeSchema>;
  items?: TypeSchema;
  required?: string[];
  description?: string;
  example?: unknown;
  enum?: unknown[];
  nullable?: boolean;
}
```

---

## Styling

### Default Styles

```tsx
import "@tauri-nexus/rpc-docs/styles.css";
```

### Custom Theming

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
  --rpc-docs-query: #3b82f6;
  --rpc-docs-mutation: #8b5cf6;
  --rpc-docs-subscription: #22c55e;
}
```

---

## Backend Requirements

Your Tauri backend should expose a `rpc_schema` command:

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
                    "properties": { "id": { "type": "number" } },
                    "required": ["id"]
                })),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "number" },
                        "name": { "type": "string" }
                    }
                })),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}
```

---

## Related Packages

- [`@tauri-nexus/rpc-core`](../rpc-core) — Core RPC client
- [`@tauri-nexus/rpc-react`](../rpc-react) — React hooks and TanStack Query

## License

MIT © Tauri Nexus
