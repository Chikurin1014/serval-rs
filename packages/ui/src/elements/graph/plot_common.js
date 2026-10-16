// What the plots' scripts share: the theme's colors, and Rust's messages.
// Prepended to each plot's script.

/**
 * Colors of CSS variables as the theme has them, taken in `container`: each
 * kept until `clear`ed (as the theme changes), as taking one makes the page
 * work its styles out.
 */
function themeColors(container) {
  const probe = document.createElement("span");
  probe.style.display = "none";
  container.appendChild(probe);
  const colors = new Map();
  return {
    color(name) {
      if (!colors.has(name)) {
        probe.style.color = `var(${name})`;
        colors.set(name, getComputedStyle(probe).color);
      }
      return colors.get(name);
    },
    clear() {
      colors.clear();
    },
  };
}

/** A computed `rgb(...)` color, translucent. */
function withAlpha(color, alpha) {
  return color.replace(/^rgb\((.*)\)$/, `rgba($1, ${alpha})`);
}

/**
 * Calls `onChange` as the theme changes, the page's or the system's; gives
 * what stops that.
 */
function watchTheme(onChange) {
  const observer = new MutationObserver(onChange);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme"],
  });
  const system = matchMedia("(prefers-color-scheme: dark)");
  system.addEventListener("change", onChange);
  return () => {
    observer.disconnect();
    system.removeEventListener("change", onChange);
  };
}

/** What Rust sends, until it sends `null` as the plot is dropped. */
async function* messages() {
  while (true) {
    const message = await dioxus.recv();
    if (message === null) {
      return;
    }
    yield message;
  }
}
