import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { useQuery } from "@tanstack/react-query";
import { RpcProvider, orpc } from "../rpc/contract";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
} from "@/components/ui/sidebar";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import "../styles/global.css";

function HealthStatus() {
  const { data, isLoading, error } = useQuery({
    ...orpc.health.queryOptions(),
    refetchInterval: 30000,
  });

  if (isLoading) {
    return (
      <Tooltip>
        <TooltipTrigger>
          <span className="size-2.5 rounded-full bg-yellow-500 animate-pulse" />
        </TooltipTrigger>
        <TooltipContent>Checking...</TooltipContent>
      </Tooltip>
    );
  }

  if (error) {
    return (
      <Tooltip>
        <TooltipTrigger>
          <span className="size-2.5 rounded-full bg-destructive" />
        </TooltipTrigger>
        <TooltipContent>Disconnected</TooltipContent>
      </Tooltip>
    );
  }

  return (
    <Tooltip>
      <TooltipTrigger>
        <span className="size-2.5 rounded-full bg-green-500" />
      </TooltipTrigger>
      <TooltipContent>Connected v{data?.version}</TooltipContent>
    </Tooltip>
  );
}

const navItems = {
  overview: [{ to: "/", icon: "🏠", label: "Dashboard" }],
  queries: [
    { to: "/greet", icon: "👋", label: "Greet" },
    { to: "/users", icon: "👥", label: "Users" },
  ],
  advanced: [
    { to: "/batch", icon: "📦", label: "Batch" },
    { to: "/advanced", icon: "🔧", label: "Advanced" },
    { to: "/docs", icon: "📚", label: "API Docs" },
  ],
  subscriptions: [
    { to: "/streams/counter", icon: "�", label: "Counter" },
    { to: "/streams/stocks", icon: "📈", label: "Stocks" },
    { to: "/streams/chat", icon: "💬", label: "Chat" },
    { to: "/streams/time", icon: "⏰", label: "Time" },
  ],
};

function AppSidebar() {
  return (
    <Sidebar>
      <SidebarHeader className="border-b border-sidebar-border px-4 py-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="text-xl">⚡</span>
            <span className="font-semibold text-lg">Tauri RPC</span>
          </div>
          <HealthStatus />
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Overview</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.overview.map((item) => (
                <SidebarMenuItem key={item.to}>
                  <SidebarMenuButton asChild>
                    <Link
                      to={item.to}
                      activeProps={{
                        className:
                          "bg-sidebar-accent text-sidebar-accent-foreground",
                      }}
                    >
                      <span>{item.icon}</span>
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>Queries</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.queries.map((item) => (
                <SidebarMenuItem key={item.to}>
                  <SidebarMenuButton asChild>
                    <Link
                      to={item.to}
                      activeProps={{
                        className:
                          "bg-sidebar-accent text-sidebar-accent-foreground",
                      }}
                    >
                      <span>{item.icon}</span>
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>Advanced</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.advanced.map((item) => (
                <SidebarMenuItem key={item.to}>
                  <SidebarMenuButton asChild>
                    <Link
                      to={item.to}
                      activeProps={{
                        className:
                          "bg-sidebar-accent text-sidebar-accent-foreground",
                      }}
                    >
                      <span>{item.icon}</span>
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>Subscriptions</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.subscriptions.map((item) => (
                <SidebarMenuItem key={item.to}>
                  <SidebarMenuButton asChild>
                    <Link
                      to={item.to}
                      activeProps={{
                        className:
                          "bg-sidebar-accent text-sidebar-accent-foreground",
                      }}
                    >
                      <span>{item.icon}</span>
                      <span>{item.label}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className="border-t border-sidebar-border px-4 py-3">
        <span className="text-xs text-muted-foreground">v0.1.0</span>
      </SidebarFooter>
    </Sidebar>
  );
}

function RootLayout() {
  return (
    <RpcProvider>
      <SidebarProvider>
        <AppSidebar />
        <SidebarInset className="bg-background">
          <main className="flex-1 overflow-auto">
            <Outlet />
          </main>
        </SidebarInset>
      </SidebarProvider>
      <TanStackRouterDevtools position="bottom-right" />
    </RpcProvider>
  );
}

export const Route = createRootRoute({
  component: RootLayout,
});
