/** @type {import('tailwindcss').Config} */
export default {
    content: ["./src/**/*.{html,js,svelte,ts}"],
    theme: {
        extend: {
            colors: {
                white: "#E6EAED",
                black: "#151515",
                primary: "#80B7DC",
                secondary: "#98BCD4",
                accent: "#274457",
            },
        },
    },
    plugins: [],
};
