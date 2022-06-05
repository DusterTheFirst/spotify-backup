import { h } from "preact";
import html_response from "../../render/response";
import { FetchFailure } from "../../spotify";
import Error from "./error";

export default function server_error(
    message: string,
    error: FetchFailure["error"] | string
) {
    console.log("SERVER ERROR:", message, error);

    return html_response(
        <Error status={500}>
            <p>The server encountered an unknown error.</p>
            <pre>{message}</pre>
            {error !== undefined ?? (
                <pre>{JSON.stringify(error, undefined, 4)}</pre>
            )}
        </Error>,
        { status: 500 }
    );
}
