import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { Repository } from './index.js'

const ROOT_DIR = join(fileURLToPath(import.meta.url), '..')

// Open the sub directory
const repo = Repository.discover(join(ROOT_DIR, 'src'))

console.info('Repo root path:', join(repo.path(), '..'))

const head = repo.head()

console.info('HEAD:', head.name())
console.info('HEAD shorthand:', head.shorthand())

repo.tagForeach((oid, nameBuffer) => {
  console.info(`Tag: ${nameBuffer.toString()} (${oid.toString()})`)
})
