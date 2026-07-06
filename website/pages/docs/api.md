---
title: 'API Reference'
description: 'The complete @napi-rs/simple-git surface — Repository, git object handles, options, enums, functions and typed error handling, all from the package root.'
---

# API Reference

Everything is exported from the package root:

```ts
import { Repository, isGitError, GitErrorCode } from '@napi-rs/simple-git'
```

This page is organized by kind: the `Repository` class first, then the object handles it returns, the option and result shapes those methods use, the enums, the standalone functions, and finally error handling.

[[toc]]

## Repository

The primary entry point — open, clone, inspect and mutate a repository in-process, with an `*Async` twin for each expensive operation.

## Git objects & handles

The handle types a `Repository` hands back: commits, trees, references, remotes, the index, blame results and their kin.

## Options & result types

The plain-object option bags passed into methods and the result shapes they return.

## Enums

The bitflag and discriminant enums used across statuses, resets, revision-walk sorts and error codes.

## Functions

The standalone functions exported alongside `Repository`, including the `isGitError` type guard.

## Error handling

How Git-layer failures surface as typed `Error`s carrying a `GitErrorCode`, and how to narrow them safely inside a `catch`.
