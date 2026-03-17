import React from "react";
import { createRootRoute, Link, Outlet, useRouterState } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  FileDiff,
  HardDrive,
  Info,
  LayoutList,
  SlidersHorizontal,
} from "lucide-react";
import { Toaster } from "sonner";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ipc } from "@/lib/ipc";
import { ThemeProvider } from "@/components/theme-provider";
import { ModeToggle } from "@/components/mode-toggle";
import { TooltipProvider } from "@/components/ui/tooltip";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";

export const Route = createRootRoute({
  component: RootLayout,
});

const NAV: {
  to: string;
  label: string;
  icon: React.FC<{ className?: string }>;
  exact?: boolean;
}[] = [
  { to: "/", label: "Dotfiles", icon: LayoutList, exact: true },
  { to: "/diff", label: "Diff", icon: FileDiff },
  { to: "/settings", label: "Settings", icon: SlidersHorizontal },
];

function RootLayout() {
  return (
    <ThemeProvider storageKey="omah-theme">
      <TooltipProvider>
        <SidebarProvider>
          <AppSidebar />
          <SidebarInset className="overflow-hidden">
            <Outlet />
          </SidebarInset>
          <Toaster
            position="bottom-right"
            toastOptions={{
              classNames: {
                toast: "border-border bg-card text-foreground text-sm",
                description: "text-muted-foreground",
              },
            }}
          />
        </SidebarProvider>
      </TooltipProvider>
    </ThemeProvider>
  );
}

function AppSidebar() {
  const { data: version } = useQuery({
    queryKey: ["version"],
    queryFn: () => ipc.getVersion(),
    staleTime: Number.POSITIVE_INFINITY,
  });
  const { data: config } = useQuery({
    queryKey: ["config"],
    queryFn: () => ipc.getConfig(),
  });

  const location = useRouterState({ select: (s) => s.location.pathname });

  function openAbout() {
    const win = new WebviewWindow("about", {
      url: "/",
      title: "About omah",
      width: 420,
      height: 480,
      resizable: false,
      center: true,
    });
    win.once("tauri://error", (e) => console.error("about window error", e));
  }

  return (
    <Sidebar collapsible="icon">
      {/* Header: logo + version */}
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <div className="flex items-center gap-2 px-1 py-1">
              <div className="flex size-7 shrink-0 items-center justify-center rounded-md bg-sidebar-primary text-sidebar-primary-foreground">
                <span className="text-[11px] font-bold">om</span>
              </div>
              <div className="flex min-w-0 flex-1 items-center justify-between group-data-[collapsible=icon]:hidden">
                <span className="text-sm font-semibold tracking-tight">omah</span>
                {version && (
                  <span className="rounded-full bg-sidebar-accent px-1.5 py-0.5 font-mono text-[10px] text-sidebar-accent-foreground">
                    v{version}
                  </span>
                )}
              </div>
            </div>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      {/* Nav items */}
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {NAV.map(({ to, label, icon: Icon, exact }) => {
                const isActive = exact ? location === to : location.startsWith(to);
                return (
                  <SidebarMenuItem key={to}>
                    <SidebarMenuButton asChild isActive={isActive} tooltip={label}>
                      <Link to={to}>
                        <Icon />
                        <span>{label}</span>
                      </Link>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      {/* Footer: vault path + controls */}
      <SidebarFooter>
        <SidebarMenu>
          {/* Vault info (hidden when collapsed) */}
          {config && (
            <SidebarMenuItem className="group-data-[collapsible=icon]:hidden">
              <div className="flex items-start gap-2 px-2 py-1">
                <HardDrive className="mt-0.5 size-3.5 shrink-0 text-sidebar-foreground/40" />
                <div className="min-w-0">
                  <p className="text-[10px] font-medium uppercase tracking-wider text-sidebar-foreground/30">
                    Vault
                  </p>
                  <p
                    className="mt-0.5 truncate font-mono text-[11px] text-sidebar-foreground/60"
                    title={config.vault_path}
                  >
                    {config.vault_path.replace(/^\/Users\/[^/]+/, "~")}
                  </p>
                </div>
              </div>
            </SidebarMenuItem>
          )}

          <Separator className="my-1 group-data-[collapsible=icon]:hidden" />

          {/* Controls row */}
          <SidebarMenuItem>
            <div className="flex items-center justify-between px-1 group-data-[collapsible=icon]:justify-center">
              <SidebarMenuButton
                onClick={openAbout}
                tooltip="About omah"
                className="h-7 w-auto gap-1.5 px-2 text-xs text-sidebar-foreground/50 hover:text-sidebar-foreground group-data-[collapsible=icon]:w-8 group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:px-0"
              >
                <Info className="size-3.5 shrink-0" />
                <span className="group-data-[collapsible=icon]:hidden">About</span>
              </SidebarMenuButton>
              <div className="group-data-[collapsible=icon]:hidden">
                <ModeToggle />
              </div>
            </div>
          </SidebarMenuItem>

          {/* Collapse toggle */}
          <SidebarMenuItem>
            <SidebarTrigger className="w-full" />
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
