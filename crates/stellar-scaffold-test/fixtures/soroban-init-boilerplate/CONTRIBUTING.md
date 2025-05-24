# Contributing

This is a guide to contributing to `loam-build/soroban-frontend-template`
itself. Feel free to delete or modify it for your own project.

soroban-cli requires that the main branch obtained with `git clone` be the branch to use as a template. So we are keeping `main` free of artifacts that do not make sense in the context of a `soroban contract init` template, such as the `contracts` folder.

However, when actually maintaining and improving this template, we need these artifacts.

Therefore, to contribute to this project, please check out the `dev` branch. All pushes/merges to the `dev` branch will be automatically pushed to `main` [on every push](.github/workflows/publish.yml).
