# `@napi-rs/simple-git`

![https://github.com/Brooooooklyn/simple-git/actions](https://github.com/Brooooooklyn/simple-git/workflows/CI/badge.svg)
![](https://img.shields.io/npm/dm/@napi-rs/simple-git.svg?sanitize=true)
[![Install size](https://packagephobia.com/badge?p=@napi-rs/simple-git)](https://packagephobia.com/result?p=@napi-rs/simple-git)

## `getFileLatestModifiedDateByGit`

```ts
import { getFileLatestModifiedDateByGit } from '@napi-rs/simple-git`

const timestamp = new Date(getFileLatestModifiedDateByGit('.', 'build.rs'))
console.log(timestamp) // 2022-03-13T12:47:47.920Z
```

## `getFileLatestModifiedDateByGitAsync`

Non blocking API for `getFileLatestModifiedDateByGit`:

```ts
import { getFileLatestModifiedDateByGitAsync } from '@napi-rs/simple-git`

const timestamp = new Date(await getFileLatestModifiedDateByGitAsync('.', 'build.rs'))
console.log(timestamp) // 2022-03-13T12:47:47.920Z
```

## Performance

Compared with the `exec` function, which gets the file's latest modified date by spawning a child process. Getting the latest modified date from the file 1000 times:

```
Child process took 1.9s
@napi-rs/simple-git took 49ms
```
