import { createFileRoute } from "@tanstack/react-router";
import {
  ApiDocsProvider,
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
  ApiDocsSearch,
  ApiDocsTypeFilterButton,
  ApiDocsCount,
  ApiDocsProcedureList,
  ApiDocsProcedureGroup,
  ApiDocsProcedureCard,
  ApiDocsProcedureBadge,
  ApiDocsProcedurePath,
  ApiDocsProcedureDeprecated,
  ApiDocsProcedureDescription,
  ApiDocsProcedureTags,
  ApiDocsTester,
  ApiDocsTesterInput,
  ApiDocsTesterExecute,
  ApiDocsTesterResponse,
  ApiDocsTypeRenderer,
  useApiDocsContext,
  useCurrentProcedure,
  type ProcedureType,
} from "@tauri-nexus/rpc-docs";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Spinner } from "@/components/ui/spinner";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Alert, AlertDescription } from "@/components/ui/alert";

const TYPE_FILTERS: Array<{ value: ProcedureType | "all"; label: string }> = [
  { value: "all", label: "All" },
  { value: "query", label: "Queries" },
  { value: "mutation", label: "Mutations" },
  { value: "subscription", label: "Subscriptions" },
];

function ProcedureTypeBadge() {
  const { schema } = useCurrentProcedure();
  const variants: Record<string, string> = {
    query: "bg-blue-500/20 text-blue-400 border-blue-500/30",
    mutation: "bg-orange-500/20 text-orange-400 border-orange-500/30",
    subscription: "bg-violet-500/20 text-violet-400 border-violet-500/30",
  };

  return (
    <ApiDocsProcedureBadge
      className={`px-2 py-0.5 rounded text-xs font-medium ${variants[schema.procedure_type] ?? ""}`}
    />
  );
}

function ProcedureInputSchema() {
  const { schema } = useCurrentProcedure();
  if (!schema.input) return null;

  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">Input</p>
      <div className="bg-muted rounded-lg p-3 font-mono text-xs overflow-x-auto">
        <ApiDocsTypeRenderer schema={schema.input} />
      </div>
    </div>
  );
}

function ProcedureOutputSchema() {
  const { schema } = useCurrentProcedure();
  if (!schema.output) return null;

  return (
    <div className="space-y-2">
      <p className="text-xs font-medium text-muted-foreground">Output</p>
      <div className="bg-muted rounded-lg p-3 font-mono text-xs overflow-x-auto">
        <ApiDocsTypeRenderer schema={schema.output} />
      </div>
    </div>
  );
}

function ProcedureTesterSection() {
  const { path, schema } = useCurrentProcedure();

  return (
    <ApiDocsTester path={path} inputSchema={schema.input}>
      <div className="space-y-3 pt-4 border-t">
        <p className="text-xs font-medium text-muted-foreground">Try It</p>
        <ApiDocsTesterInput
          className="w-full min-h-[100px] font-mono text-xs bg-muted border-border rounded-lg p-3 resize-y"
          errorClassName="border-destructive"
          renderError={(error) => (
            <p className="text-xs text-destructive mt-1">{error}</p>
          )}
        />
        <ApiDocsTesterExecute
          className="px-4 py-2 bg-primary text-primary-foreground rounded-lg text-sm font-medium hover:bg-primary/90 disabled:opacity-50"
          loadingContent={
            <>
              <Spinner className="size-4 mr-2 inline" /> Executing...
            </>
          }
        >
          Execute
        </ApiDocsTesterExecute>
        <ApiDocsTesterResponse
          render={({ response, error, executionTime, isLoading }) => (
            <div className="space-y-2">
              {executionTime !== null && (
                <p className="text-xs text-muted-foreground">
                  {executionTime}ms
                </p>
              )}
              {isLoading && (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Spinner className="size-4" /> Executing...
                </div>
              )}
              {!isLoading && error && (
                <Alert variant="destructive">
                  <AlertDescription>{error}</AlertDescription>
                </Alert>
              )}
              {!isLoading && response !== null && !error && (
                <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-3">
                  <pre className="text-xs font-mono text-green-400 overflow-x-auto">
                    {JSON.stringify(response, null, 2)}
                  </pre>
                </div>
              )}
            </div>
          )}
        />
      </div>
    </ApiDocsTester>
  );
}

function ProcedureCardContent() {
  const { expanded, toggle } = useCurrentProcedure();

  return (
    <Card>
      <Collapsible open={expanded} onOpenChange={toggle}>
        <CollapsibleTrigger asChild>
          <button className="w-full text-left">
            <CardHeader className="py-3 px-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <ProcedureTypeBadge />
                  <ApiDocsProcedurePath className="font-mono text-sm" />
                  <ApiDocsProcedureDeprecated className="text-xs text-yellow-500">
                    ⚠️ Deprecated
                  </ApiDocsProcedureDeprecated>
                </div>
                <div className="flex items-center gap-2">
                  <ApiDocsProcedureDescription className="text-xs text-muted-foreground max-w-md truncate hidden md:block" />
                  <span className="text-muted-foreground">
                    {expanded ? "▼" : "▶"}
                  </span>
                </div>
              </div>
            </CardHeader>
          </button>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <CardContent className="pt-0 space-y-4">
            <ApiDocsProcedureDescription className="text-sm text-muted-foreground" />
            <ApiDocsProcedureTags
              className="flex flex-wrap gap-1"
              renderTag={(tag) => (
                <Badge key={tag} variant="secondary" className="text-xs">
                  {tag}
                </Badge>
              )}
            />
            <ProcedureInputSchema />
            <ProcedureOutputSchema />
            <ProcedureTesterSection />
          </CardContent>
        </CollapsibleContent>
      </Collapsible>
    </Card>
  );
}

function DocsContent() {
  const { groups, isLoading, error } = useApiDocsContext();

  if (isLoading) {
    return (
      <ApiDocsLoading className="flex flex-col items-center justify-center py-16 text-muted-foreground">
        <Spinner className="size-8 mb-4" />
        <p>Loading API documentation...</p>
      </ApiDocsLoading>
    );
  }

  if (error) {
    return (
      <ApiDocsError
        className="flex flex-col items-center justify-center py-16"
        render={({ error, refetch }) => (
          <>
            <span className="text-4xl mb-4">⚠️</span>
            <p className="text-muted-foreground mb-4">{error.message}</p>
            <Button onClick={refetch}>Retry</Button>
          </>
        )}
      />
    );
  }

  return (
    <ApiDocsContent>
      <ApiDocsEmpty
        className="text-center py-16 text-muted-foreground"
        render={({ clearFilters, hasSearch }) => (
          <>
            <p className="mb-4">No procedures match your filters.</p>
            {hasSearch && (
              <Button variant="outline" onClick={clearFilters}>
                Clear search
              </Button>
            )}
          </>
        )}
      />

      <ApiDocsProcedureList className="space-y-6">
        {groups.map((group) => (
          <ApiDocsProcedureGroup
            key={group.namespace || "__root__"}
            group={group}
            className="space-y-3"
          >
            {group.namespace && (
              <h3 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
                {group.namespace}
              </h3>
            )}
            <div className="space-y-2">
              {group.procedures.map((proc) => (
                <ApiDocsProcedureCard
                  key={proc.path}
                  path={proc.path}
                  schema={proc.schema}
                >
                  <ProcedureCardContent />
                </ApiDocsProcedureCard>
              ))}
            </div>
          </ApiDocsProcedureGroup>
        ))}
      </ApiDocsProcedureList>
    </ApiDocsContent>
  );
}

function DocsPage() {
  return (
    <div className="p-8 max-w-6xl mx-auto">
      <ApiDocsProvider>
        <ApiDocsRoot className="space-y-6">
          <ApiDocsHeader className="flex items-start justify-between">
            <div>
              <ApiDocsTitle
                className="text-3xl font-bold mb-2"
                title="API Documentation"
              />
              <ApiDocsDescription
                className="text-muted-foreground"
                description="Browse and explore available RPC procedures"
              />
            </div>
            <ApiDocsVersion className="text-xs text-muted-foreground bg-muted px-2 py-1 rounded" />
          </ApiDocsHeader>

          <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
            <div className="flex items-center gap-2 flex-wrap">
              <ApiDocsSearch
                className="w-64 px-3 py-2 bg-muted border border-border rounded-lg text-sm"
                placeholder="Search procedures..."
              />
              <div className="flex gap-1">
                {TYPE_FILTERS.map(({ value, label }) => (
                  <ApiDocsTypeFilterButton
                    key={value}
                    value={value}
                    className="px-3 py-1.5 text-xs rounded-lg border border-border hover:bg-muted transition-colors data-[active=true]:bg-primary data-[active=true]:text-primary-foreground data-[active=true]:border-primary"
                  >
                    {label}
                  </ApiDocsTypeFilterButton>
                ))}
              </div>
            </div>

            <div className="flex items-center gap-3">
              <ApiDocsCount
                className="text-sm text-muted-foreground"
                render={({ filtered, total }) => (
                  <>
                    <span className="font-semibold text-foreground">
                      {filtered}
                    </span>{" "}
                    / {total} procedures
                  </>
                )}
              />
              <ApiDocsActions
                render={({
                  expandAll,
                  collapseAll,
                  canExpand,
                  canCollapse,
                }) => (
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={expandAll}
                      disabled={!canExpand}
                    >
                      Expand All
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={collapseAll}
                      disabled={!canCollapse}
                    >
                      Collapse All
                    </Button>
                  </div>
                )}
              />
            </div>
          </div>

          <DocsContent />
        </ApiDocsRoot>
      </ApiDocsProvider>
    </div>
  );
}

export const Route = createFileRoute("/docs")({
  component: DocsPage,
});
