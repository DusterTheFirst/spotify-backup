import { ComponentChild, h } from "preact";

const statusTexts = {
    404: "Not Found",
    405: "Method Not Allowed",
    500: "Internal Server Error",
};

export default function Error({
    status,
    children,
}: {
    status: keyof typeof statusTexts;
    children: ComponentChild;
}) {
    return (
        <main>
            <section>
                <h1>
                    {status} {statusTexts[status]}
                </h1>
                {children}
            </section>
            <footer>
                <a href="/">return home</a>
            </footer>
        </main>
    );
}
