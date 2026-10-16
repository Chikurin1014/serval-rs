// Keeps a log table (newest first) at its newest rows as they come on top,
// while it is scrolled to them; else keeps the rows in view where they are, as
// those come over them.
//
// From Rust: the table's id as it is mounted, the number of rows that came on
// top as they do, then `null` on unmount.

// As `ROW_HEIGHT`
const ROW_HEIGHT = 24;

let list = null;
// Whether it is at the newest, as last scrolled by the user
let following = true;
// Where it was last scrolled to by this: its scrolling from there is not the
// user's (its event may come after the table changes again)
let scrolledTo = null;
// Rows that came on top, to be scrolled past once laid out
let added = 0;
let frame = 0;

function onScroll() {
  if (scrolledTo !== null && Math.abs(list.scrollTop - scrolledTo) < 1) {
    return;
  }
  scrolledTo = null;
  following = list.scrollTop <= 4;
}

function keepInView() {
  frame = 0;
  if (list) {
    // At the newest; or the rows in view where they were, below those on top
    list.scrollTop = following ? 0 : list.scrollTop + added * ROW_HEIGHT;
    scrolledTo = list.scrollTop;
  }
  added = 0;
}

while (true) {
  const message = await dioxus.recv();
  if (message === null) {
    break;
  }
  if (typeof message === "string") {
    list = document.getElementById(message)?.querySelector("[role=list]");
    list?.addEventListener("scroll", onScroll, { passive: true });
    continue;
  }
  added += message;
  // Once they are laid out, the next frame
  if (!frame) {
    frame = requestAnimationFrame(keepInView);
  }
}

cancelAnimationFrame(frame);
list?.removeEventListener("scroll", onScroll);
