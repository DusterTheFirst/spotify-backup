import { h, JSX } from "preact";

export default function html_wrapper(body: JSX.Element) {
    return (
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
                <link rel="manifest" href="/manifest.json" />
                <title>Spotify Backup</title>
            </head>
            <body>{body}</body>
        </html>
    );
}
