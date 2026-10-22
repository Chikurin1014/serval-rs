"""Serves the directory given on 127.0.0.1, on the port given.

As `python -m http.server`, but the Nix store's files are all of 1970: told
"not modified" since any page a browser kept (as one of `dx serve` on the same
port), it would show that. So never "not modified", and asked again each time.
"""

import functools
import sys
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer


class Handler(SimpleHTTPRequestHandler):
    def send_head(self):
        del self.headers["If-Modified-Since"]
        return super().send_head()

    def end_headers(self):
        self.send_header("Cache-Control", "no-cache")
        super().end_headers()


port, directory = int(sys.argv[1]), sys.argv[2]
server = ThreadingHTTPServer(
    ("127.0.0.1", port), functools.partial(Handler, directory=directory)
)
server.serve_forever()
