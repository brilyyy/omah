import { createContext, useContext, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type Theme = "dark" | "light" | "system";

type ThemeProviderState = {
  theme: Theme;
  setTheme: (theme: Theme) => void;
};

const ThemeProviderContext = createContext<ThemeProviderState>({
  theme: "system",
  setTheme: () => null,
});

function resolveTheme(t: Theme): "dark" | "light" {
  if (t !== "system") return t;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyTheme(t: Theme) {
  const resolved = resolveTheme(t);
  const root = document.documentElement;
  root.classList.remove("light", "dark");
  root.classList.add(resolved);
  // Sync the native window chrome (title bar, traffic lights) with resolved theme.
  // setTheme(null) tells the OS to follow system; setTheme("dark"|"light") forces it.
  getCurrentWindow()
    .setTheme(t === "system" ? null : resolved)
    .catch(() => {
      // Silently ignore if the permission isn't granted yet or we're in browser dev mode.
    });
}

export function ThemeProvider({
  children,
  defaultTheme = "system",
  storageKey = "omah-theme",
}: {
  children: React.ReactNode;
  defaultTheme?: Theme;
  storageKey?: string;
}) {
  const [theme, setThemeState] = useState<Theme>(
    () => (localStorage.getItem(storageKey) as Theme) || defaultTheme,
  );

  // Apply CSS class + sync Tauri window on every theme change.
  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  // Listen for OS-level theme changes so "system" preference stays in sync.
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const mqHandler = () => {
      if (theme === "system") applyTheme("system");
    };
    mq.addEventListener("change", mqHandler);

    // Also wire up Tauri's finer-grained theme-changed event.
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onThemeChanged(({ payload }) => {
        if (theme === "system") {
          document.documentElement.classList.remove("light", "dark");
          document.documentElement.classList.add(payload === "dark" ? "dark" : "light");
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});

    return () => {
      mq.removeEventListener("change", mqHandler);
      unlisten?.();
    };
  }, [theme]);

  const value: ThemeProviderState = {
    theme,
    setTheme: (t) => {
      localStorage.setItem(storageKey, t);
      setThemeState(t);
    },
  };

  return (
    <ThemeProviderContext.Provider value={value}>{children}</ThemeProviderContext.Provider>
  );
}

export const useTheme = () => {
  const context = useContext(ThemeProviderContext);
  if (context === undefined) throw new Error("useTheme must be used within a ThemeProvider");
  return context;
};
