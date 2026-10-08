// Renders a formula with KaTeX. From Rust: `[id, latex]`.

const [id, latex] = await dioxus.recv();

// Its <script> tag may not have run yet
while (!window.katex) {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

const formula = document.getElementById(id);
const math = formula?.querySelector(".formula-math");
if (!math) {
  return;
}
window.katex.render(latex, math, { throwOnError: false });
formula.dataset.rendered = "true";
