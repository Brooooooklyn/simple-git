import { execSync } from 'child_process'
import { join } from 'path'
import { fileURLToPath } from 'url'

import test from 'ava'

const __dirname = join(fileURLToPath(import.meta.url), '..')

import { Repository } from '../index.js'

const workDir = join(__dirname, '..')

test.beforeEach((t) => {
  t.context.repo = new Repository(workDir)
})

test('Date should be equal with cli', (t) => {
  const { repo } = t.context
  if (process.env.CI) {
    t.notThrows(() => repo.getFileLatestModifiedDate(join('src', 'lib.rs')))
  } else {
    t.deepEqual(
      new Date(
        execSync('git log -1 --format=%cd --date=iso src/lib.rs', {
          cwd: workDir,
        })
          .toString('utf8')
          .trim()
      ).valueOf(),
      repo.getFileLatestModifiedDate(join('src', 'lib.rs'))
    )
  }
})

test('Should be able to resolve head', (t) => {
  const { repo } = t.context
  t.is(
    repo.head().target(),
    process.env.CI
      ? process.env.GITHUB_SHA
      : execSync('git rev-parse HEAD', {
          cwd: workDir,
        })
          .toString('utf8')
          .trim()
  )
})
