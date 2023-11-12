/// <reference path="./types.d.ts" />

const sb = supabase.createClient(
    "https://jbovpzewembwwyozyqmr.supabase.co",
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Impib3ZwemV3ZW1id3d5b3p5cW1yIiwicm9sZSI6ImFub24iLCJpYXQiOjE2OTk1NjAxMTgsImV4cCI6MjAxNTEzNjExOH0.NsWMZoV9OjC3VGdQ60lnqi0ajkdPj0ngAA7XJ4dKTPc"
);

sb.auth.onAuthStateChange((event, session) => {
    console.log(event);
    console.log(session);

    const user = session?.user;

    document.getElementById("debug").innerText = JSON.stringify(
        user?.user_metadata,
        undefined,
        4
    );
});

window.addEventListener("DOMContentLoaded", () => {
    const spotify_button = /** @type {HTMLButtonElement} */ (
        document.querySelector("button.login[data-spotify]")
    );
    spotify_button.addEventListener("click", async () => {
        const { data, error } = await sb.auth.signInWithOAuth({
            provider: "spotify",
            options: { redirectTo: "http://localhost:3000/" },
        });

        console.log(data, error);
    });

    const github_button = /** @type {HTMLButtonElement} */ (
        document.querySelector("button.login[data-github]")
    );
    github_button.addEventListener("click", async () => {
        const { data, error } = await sb.auth.signInWithOAuth({
            provider: "github",
            options: { redirectTo: "http://localhost:3000/" },
        });

        console.log(data, error);
    });

    const sign_out_button = /** @type {HTMLButtonElement} */ (
        document.querySelector("button.logout")
    );
    sign_out_button.addEventListener("click", async () => {
        const { error } = await sb.auth.signOut();

        console.log(error);
    });
});
