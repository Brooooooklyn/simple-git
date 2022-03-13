import { execSync } from 'child_process'
import { join } from 'path'
import { fileURLToPath } from 'url'

import test from 'ava'

const __dirname = join(fileURLToPath(import.meta.url), '..')

import { getFileLatestModifiedDateByGit } from '../index.js'

test('Date should be equal with cli', (t) => {
  let date
  try {
    date = execSync('git log -1 --format=%cd --date=iso src/lib.rs', {
      cwd: join(__dirname, '..'),
    }).toString('utf8')
  } catch (e) {
    t.pass('Git not installed')
    return
  }
  t.is(
    new Date(date).valueOf(),
    getFileLatestModifiedDateByGit(join(__dirname, '..'), join('src', 'lib.rs'))
  )
})
