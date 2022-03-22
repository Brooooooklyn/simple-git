import { Repository } from './index.js'

const repo = new Repository('.')

repo.remote('origin').fetch(['main'], (p) => {
  console.log(p)
})
