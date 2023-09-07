module.exports = {
    "env": {
        "browser": true,
        "es2020": true
    },
    "extends": [
        "eslint:recommended",
        "preact"
    ],
    "parser": "@typescript-eslint/parser",
    "parserOptions": {
        "ecmaFeatures": {
            "jsx": true,
        },
        "ecmaVersion": 11,
        "sourceType": "module",
        "project": "./tsconfig.json",
    },
    "settings": {
        // we don't have jest installed but the preact config requires it
        // this is a workaround:
        "jest": { "version": "latest" },
    },
    "rules": {
        "no-unused-vars": ["error", {"argsIgnorePattern": "^_"}],
        "no-else-return": "off",
        "prefer-template": "off",
    }
};
