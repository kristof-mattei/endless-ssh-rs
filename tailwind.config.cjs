/** @type { import("tailwindcss").Config } */
const tailwindCssConfig = {
    content: ["./front-end/index.html", "./front-end/src/**/*.{js,ts,jsx,tsx}"],
    theme: {
        extend: {},
    },
    plugins: [],
};

module.exports = tailwindCssConfig;
