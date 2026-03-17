import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createHashHistory,
  createRouter,
} from "@tanstack/react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { routeTree } from "./routeTree.gen";
import AboutWindow from "./components/about-window";
import "./App.css";

// Detect which Tauri window we're in before React renders.
// getCurrentWindow().label is synchronous — it reads from __TAURI_INTERNALS__.
const windowLabel = getCurrentWindow().label;

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: 1, staleTime: 10_000 },
  },
});

const router = createRouter({
  routeTree,
  history: createHashHistory(),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

const root = document.getElementById("root") as HTMLElement;

if (windowLabel === "about") {
  // About window: standalone UI, no sidebar or router.
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <AboutWindow />
    </React.StrictMode>,
  );
} else {
  // Main window: full app with router + React Query.
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </React.StrictMode>,
  );
}
