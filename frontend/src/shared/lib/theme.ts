import { useEffect } from "react";
import type { Theme } from "@/shared/schemas/core";

function useTheme(theme: Theme) {
  useEffect(() => {
    const root = document.documentElement;
    const systemPreference = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved = theme === "system" ? (systemPreference.matches ? "dark" : "light") : theme;
      root.dataset.theme = theme;
      root.classList.toggle("dark", resolved === "dark");
      root.style.colorScheme = resolved;
      document
        .querySelector('meta[name="theme-color"]')
        ?.setAttribute("content", resolved === "dark" ? "#0a0a0a" : "#fafafa");
    };

    apply();
    systemPreference.addEventListener("change", apply);
    return () => systemPreference.removeEventListener("change", apply);
  }, [theme]);
}

export { useTheme };
