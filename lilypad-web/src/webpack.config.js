const path = require('path');

module.exports = {
    entry: './run.js',
    output: {
        filename: 'editor.js',
        path: path.resolve(__dirname, 'dist')
    }
};