// The site's header, on the docs and the playground alike (header.py
// makes it): loaded in each page's <head>, so the chosen theme applies
// before the page shows. The theme is the system's until the toggle
// picks one, which is kept under one key for the whole site, so it
// carries between the docs and the playground (and other tabs).
(() => {
  const KEY = "temper-theme";
  const THEMES = ["system", "light", "dark"];
  const root = document.documentElement;
  root.classList.add("js");

  function stored() {
    try {
      const theme = localStorage.getItem(KEY);
      return THEMES.includes(theme) ? theme : "system";
    } catch {
      return "system"; // Storage may be refused, as in some private windows.
    }
  }

  let theme = stored();

  function apply() {
    if (theme === "system") delete root.dataset.theme;
    else root.dataset.theme = theme;
    const next = THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length];
    for (const button of document.querySelectorAll(".theme-toggle")) {
      button.dataset.theme = theme;
      button.setAttribute("aria-label", `Theme: ${theme}`);
      button.title = `Theme: ${theme} (next: ${next})`;
    }
  }

  function choose(next) {
    theme = next;
    try {
      if (theme === "system") localStorage.removeItem(KEY);
      else localStorage.setItem(KEY, theme);
    } catch {
      // Not kept, but it still applies to this page.
    }
    apply();
  }

  apply();
  addEventListener("storage", (event) => {
    if (event.key === KEY || event.key === null) {
      theme = stored();
      apply();
    }
  });

  function wire() {
    apply();
    for (const button of document.querySelectorAll(".theme-toggle")) {
      button.addEventListener("click", () => choose(THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length]));
    }
    // The phone menu: the sections, below the header; Escape or a click
    // elsewhere closes it.
    const top = document.querySelector(".top");
    const menu = top?.querySelector(".menu-button");
    if (!top || !menu) return;
    const open = (yes) => {
      top.classList.toggle("open", yes);
      menu.setAttribute("aria-expanded", String(yes));
    };
    menu.addEventListener("click", () => open(!top.classList.contains("open")));
    top.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && top.classList.contains("open")) {
        open(false);
        menu.focus();
      }
    });
    document.addEventListener("click", (event) => {
      if (top.classList.contains("open") && !top.contains(event.target)) open(false);
    });
    top.addEventListener("focusout", (event) => {
      if (top.classList.contains("open") && event.relatedTarget && !top.contains(event.relatedTarget)) open(false);
    });
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", wire);
  else wire();
})();
