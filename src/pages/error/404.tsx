import { h } from "preact";
import html_response from "../../render/response";
import Error from "./error";

export default function not_found() {
    return html_response(
        <Error status={404}>
            <p>The page you are looking for does not exist.</p>
        </Error>,
        { status: 404 }
    );
}
