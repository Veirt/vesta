/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        white: "#E6EAED",
        black: "#151515",
        "black-2": "#202020",
        primary: "#80B7DC",
        secondary: "#98BCD4",
        accent: "#274457",
      },
    },
  },
  plugins: [],
};
