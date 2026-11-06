import io
import unittest
from unittest.mock import patch

from web import serve


class LauncherTests(unittest.TestCase):
    def test_browser_opens_only_after_server_is_listening(self):
        events = []
        with patch.object(serve, "ThreadingHTTPServer") as server_class, patch.object(
            serve, "Thread"
        ) as thread_class, patch.object(serve.sys, "argv", ["serve.py", "--open"]), patch(
            "sys.stdout", new_callable=io.StringIO
        ):
            server = server_class.return_value.__enter__.return_value
            server.server_port = 18080
            server_class.side_effect = lambda *args: (
                events.append("listening") or server_class.return_value
            )
            thread_class.return_value.start.side_effect = lambda: events.append("open")
            server.serve_forever.side_effect = KeyboardInterrupt
            serve.main()

            self.assertEqual(events, ["listening", "open"])
            server_class.assert_called_once()
            self.assertEqual(server_class.call_args.args[0], ("127.0.0.1", 18080))
            self.assertEqual(
                server_class.call_args.args[1].keywords["directory"],
                str(serve.Path(serve.__file__).resolve().parent),
            )
            thread_class.assert_called_once_with(
                target=serve.open_browser,
                args=("http://127.0.0.1:18080/",),
                daemon=True,
            )
            server.serve_forever.assert_called_once()
            server_class.return_value.__exit__.assert_called_once()

    def test_serve_only_does_not_open_browser(self):
        with patch.object(serve, "ThreadingHTTPServer") as server_class, patch.object(
            serve, "Thread"
        ) as thread_class, patch.object(serve.sys, "argv", ["serve.py"]), patch(
            "sys.stdout", new_callable=io.StringIO
        ):
            server_class.return_value.__enter__.return_value.serve_forever.side_effect = (
                KeyboardInterrupt
            )
            serve.main()
            thread_class.assert_not_called()

    def test_browser_failure_prints_manual_url(self):
        for result in (False, serve.webbrowser.Error("No browser available")):
            with self.subTest(result=result), patch.object(
                serve.webbrowser, "open_new_tab", return_value=False
            ) as open_tab, patch("sys.stderr", new_callable=io.StringIO) as stderr:
                if isinstance(result, Exception):
                    open_tab.side_effect = result
                serve.open_browser("http://127.0.0.1:18080/")
                self.assertIn("Open http://127.0.0.1:18080/", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
