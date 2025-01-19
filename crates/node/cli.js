const { run } = require('./index.js');

console.log(process.argv.slice(2), typeof run);

run(process.argv.slice(2));
