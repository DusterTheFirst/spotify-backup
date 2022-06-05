import render from "preact-render-to-string";
import { JSX } from "preact";
import html_wrapper from "./html";

export default async function html_response(
    body: JSX.Element,
    init?: ResponseInit
) {
    return new Response("<!DOCTYPE html>" + render(html_wrapper(body)), {
        headers: {
            "content-type": "text/html;charset=UTF-8",
        },
        ...init,
    });
}
