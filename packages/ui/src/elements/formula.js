// Renders a formula with KaTeX for `Formula` (formula.rs).
//
// Message from Rust: `[id, latex]`, the formula's element id and its LaTeX.

const [id, latex] = await dioxus.recv();

// `katex.min.js` is loaded by a <script> tag that may not have run yet
while (!window.katex) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

const formula = document.getElementById(id);
const math = formula?.querySelector(".formula-math");
if (!math) {
  return;
}
window.katex.render(latex, math, { throwOnError: false });
// Shows the rendered formula in place of the fallback (see formula.css)
formula.dataset.rendered = "true";
