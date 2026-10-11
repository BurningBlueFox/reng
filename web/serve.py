import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import sys
from threading import Thread
import webbrowser


def open_browser(url):
    try:
        if webbrowser.open_new_tab(url):
            return
    except webbrowser.Error as error:
        print(f"Could not open the browser: {error}", file=sys.stderr)
    print(f"Open {url} in your browser.", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description="Serve the reng browser demo.")
    parser.add_argument("--open", action="store_true", help="Open the default browser")
    args = parser.parse_args()
    handler = partial(SimpleHTTPRequestHandler, directory=str(Path(__file__).resolve().parent))

    with ThreadingHTTPServer(("127.0.0.1", 18080), handler) as server:
        url = f"http://127.0.0.1:{server.server_port}/"
        print(f"Serving reng at {url} (Ctrl+C to stop)", flush=True)
        if args.open:
            # Browser controllers may block; keep HTTP requests on the main thread.
            Thread(target=open_browser, args=(url,), daemon=True).start()
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
