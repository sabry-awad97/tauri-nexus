import { createFileRoute } from '@tanstack/react-router';
import { ApiDocs } from '../lib/rpc/docs';

export const Route = createFileRoute('/docs')({
  component: DocsPage,
});

function DocsPage() {
  return (
    <div className="page">
      <ApiDocs
        title="API Documentation"
        description="Browse and explore available RPC procedures in this application"
      />
    </div>
  );
}
