// Colors of CSS variables, as scripts take them.

/**
 * Colors of CSS variables as the page's styles have them, taken in
 * `container`: each kept until `clear`ed (as the theme changes), as taking
 * one makes the page work its styles out.
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
