import { h } from "preact";
import html_response from "../../render/response";
import Error from "./error";

export default function method_not_allowed(allowed: string[]) {
    return html_response(
        <Error status={405}>
            <p>The page is not available under the current http method.</p>
        </Error>,
        { status: 405, headers: { Allow: allowed.join(",") } }
    );
}
