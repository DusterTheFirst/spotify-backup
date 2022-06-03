import render from "preact-render-to-string";
import { VNode } from "preact";
import { h } from "preact";

export function html_response(body: VNode, head?: VNode) {
    return new Response(
        "<!DOCTYPE html>" +
            render(
                <html lang="en">
                    <head>
                        <meta charSet="UTF-8" />
                        <meta http-equiv="X-UA-Compatible" content="IE=edge" />
                        <meta
                            name="viewport"
                            content="width=device-width, initial-scale=1.0"
                        />
                        <link
                            rel="shortcut icon"
                            href="/assets/spotify_192_transparent.png"
                            type="image/png"
                        />
                        <link
                            rel="manifest"
                            href="/manifest.json"
                        />
                        {head}
                    </head>
                    <body>
                        <main>{body}</main>
                        <footer>
                            <a href="javascript:window.location.reload()">
                                refresh
                            </a>
                        </footer>
                    </body>
                </html>
            ),
        {
            headers: {
                "content-type": "text/html;charset=UTF-8",
            },
        }
    );
}
