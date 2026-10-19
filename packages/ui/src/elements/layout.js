// Layout files, picked through a file input.

/** Opens the file picker of the input of `id`. */
function pickFile(id) {
  document.getElementById(id)?.click();
}

/** Empties the input of `id`, so the same file picked again is a change. */
function clearFile(id) {
  const input = document.getElementById(id);
  if (input) {
    input.value = "";
  }
}
