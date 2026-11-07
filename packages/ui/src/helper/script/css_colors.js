// Colors of CSS variables, as scripts take them.

let pixel = null;

/**
 * `color` as `rgb(…)` (or `rgba(…)`), which scripts and xterm.js can read:
 * a color the styles mix (`color-mix`) comes as `oklab(…)` and the like, so
 * it is drawn and read back.
 */
function rgbColor(color) {
  if (/^rgba?\(/.test(color)) {
    return color;
  }
  if (!pixel) {
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 1;
    pixel = canvas.getContext("2d", { willReadFrequently: true });
  }
  pixel.clearRect(0, 0, 1, 1);
  pixel.fillStyle = color;
  pixel.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = pixel.getImageData(0, 0, 1, 1).data;
  return a === 255
    ? `rgb(${r}, ${g}, ${b})`
    : `rgba(${r}, ${g}, ${b}, ${Math.round((a / 255) * 1000) / 1000})`;
}

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
        colors.set(name, rgbColor(getComputedStyle(probe).color));
      }
      return colors.get(name);
    },
    clear() {
      colors.clear();
    },
  };
}
