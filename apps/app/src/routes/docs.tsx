import { createFileRoute } from "@tanstack/react-router";
import { ApiDocs } from "@tauri-nexus/rpc-docs";

export const Route = createFileRoute("/docs")({
  component: DocsPage,
});

function DocsPage() {
  return (
    <div className="p-8 max-w-6xl mx-auto">
      <ApiDocs
        title="API Documentation"
        description="Browse and explore available RPC procedures in this application"
      />
    </div>
  );
}
