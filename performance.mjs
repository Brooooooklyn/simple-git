import { exec } from 'child_process'

import prettyMs from 'pretty-ms'

import { getFileLatestModifiedDateByGitAsync } from './index.js'

const startChildProcessTime = process.hrtime.bigint()

const GIT_DIR = '.'
const FILE = 'src/lib.rs'

await Promise.all(
  Array.from({ length: 1000 }).map(
    () =>
      new Promise((resolve, reject) => {
        let output = ''
        const cp = exec(
          `git log -1 --format=%cd --date=iso ${FILE}`,
          {
            encoding: 'utf8',
            cwd: GIT_DIR,
          },
          (err, stdout) => {
            if (err) {
              return reject(err)
            }
            output += stdout
          }
        )
        cp.on('exit', () => {
          resolve(new Date(output))
        })
      })
  )
)

const childProcessNs = process.hrtime.bigint() - startChildProcessTime

console.info(
  'Child process took %s',
  prettyMs(Number(childProcessNs) / 1000_000)
)

const startLibGit2 = process.hrtime.bigint()

await Promise.all(
  Array.from({ length: 1000 }).map(() =>
    getFileLatestModifiedDateByGitAsync(GIT_DIR, FILE).then(
      (timestamp) => new Date(timestamp)
    )
  )
)

const libGit2Ns = process.hrtime.bigint() - startLibGit2

console.info(
  '@napi-rs/simple-git took %s',
  prettyMs(Number(libGit2Ns) / 1000_000)
)
