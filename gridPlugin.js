const plugin = require("tailwindcss/plugin");

const gridPlugin = plugin(function ({ addUtilities, theme, e }) {
  const max = 12;
  const baseWidth = 8;

  const utilities = {};

  for (let i = 1; i <= max; i++) {
    // data-columns
    utilities[`[data-columns="${i}"]`] = {
      "grid-template-columns": `repeat(${i}, minmax(0, 1fr))`,
      ...(i <= 4
        ? { "@media (min-width: 768px)": { width: `${i * baseWidth}rem` } }
        : i === 5
          ? { "@media (min-width: 640px)": { width: `${i * baseWidth}rem` } }
          : { width: `${i * baseWidth}rem` }),
    };

    // data-width
    utilities[`[data-width="${i}"]`] = {
      "@media (min-width: 768px)": { "min-width": `${i * baseWidth - 1}rem` },
      "grid-column": `span ${i} / span ${i}`,
    };

    // data-height
    utilities[`[data-height="${i}"]`] = {
      "min-height": `${i * baseWidth - 1}rem`,
      "max-height": `${i * baseWidth - 1}rem`,
      "grid-row": `span ${i} / span ${i}`,
    };
  }

  addUtilities(utilities);
});

module.exports = gridPlugin;
