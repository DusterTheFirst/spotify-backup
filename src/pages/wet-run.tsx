import { h } from "preact";
import html_response from "../render/response";

export default function wet_run() {
    return html_response(
        <main>
            <section>
                <h1>Are you sure you want to trigger an update?</h1>
                <ul>
                    <li>
                        <a href="javascript:location.replace('/')">no, take me back</a>
                    </li>
                    <hr />
                    <li>
                        <a href="javascript:location.replace('/wet-run/no-really')">yes, update now</a>
                    </li>
                </ul>
            </section>
        </main>
    );
}
