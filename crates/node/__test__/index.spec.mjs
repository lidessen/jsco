import test from 'ava'

import { jsco } from '../index.js'

test('jsco', async (t) => {
  t.truthy(await jsco('https://cdn.jsdelivr.net/npm/es-toolkit@1.31.0/dist/browser.global.min.js'))
})
