import os
import http.server
import socketserver
import sys
import html
import urllib


class SortedHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def list_directory(self, path):
        """Helper to produce a directory listing."""
        try:
            entries = sorted(
                os.scandir(path), key=lambda entry: entry.stat().st_mtime, reverse=True
            )
            display_path = html.escape(urllib.parse.unquote(self.path))
            enc = sys.getfilesystemencoding()
            title = "Directory listing for %s" % display_path.encode(enc)
            r = []
            r.append(
                '<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01//EN" '
                '"http://www.w3.org/TR/html4/strict.dtd">'
            )
            r.append("<html>\n<head>")
            r.append(
                '<meta http-equiv="Content-Type" '
                'content="text/html; charset=%s">' % enc
            )
            r.append("<title>%s</title>\n</head>" % title)
            r.append("<body>\n<h1>%s</h1>" % title)
            r.append("<hr>\n<ul>")
            for entry in entries:
                basename = os.path.basename(entry.path)
                basename_e = html.escape(basename, quote=False)
                if entry.is_dir():
                    basename_e += "/"
                fileurl = urllib.parse.quote(basename)
                r.append('<li><a href="%s">%s</a></li>' % (fileurl, basename_e))
            r.append("</ul>\n<hr>\n</body>\n</html>\n")
            encoded = "\n".join(r).encode(enc)
            f = self.send_head()
            if f:
                self.wfile.write(encoded)
        except Exception as e:
            print(e)
            # self.send_error(e)
            # self.handle_error(e)


# Create an HTTP server and serve the current directory
handler = SortedHTTPRequestHandler
server = socketserver.TCPServer(("", 8000), handler)
print("Serving at http://localhost:8000/")
server.serve_forever()
