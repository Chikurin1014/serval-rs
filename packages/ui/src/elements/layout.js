// Layout files: picked through a file input, saved as a download.

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

/** Saves `text` as a JSON file named `name`. */
function download(name, text) {
  const url = URL.createObjectURL(
    new Blob([text], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  // Once the download has taken it
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
