/**
 * @type {import('semantic-release').GlobalConfig}
 */
module.exports = {
    plugins: [
        [
            "@semantic-release/commit-analyzer",
            {
                preset: "conventionalcommits",
            },
        ],
        [
            "@semantic-release/release-notes-generator",
            {
                preset: "conventionalcommits",
            },
        ],
        [
            "@semantic-release/changelog",
            {
                changelogTitle: "Changelog",
                changelogFile: "CHANGELOG.md",
            },
        ],
        "@semantic-release/github",
    ],
    branches: [
        {
            name: "main",
        },
    ],
};
