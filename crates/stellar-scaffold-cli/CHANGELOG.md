# Changelog

All notable changes to this project will be documented in this file. Do not edit manually.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.16](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.15...stellar-scaffold-cli-v0.0.16) - 2025-11-01

### Added

- add install and build steps to init ([#268](https://github.com/theahaco/scaffold-stellar/pull/268))

### Other

- Fix name of second OZ contract ([#262](https://github.com/theahaco/scaffold-stellar/pull/262))

## [0.0.15](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.14...stellar-scaffold-cli-v0.0.15) - 2025-10-29

### Added

- [**breaking**] no longer force users to use local config and add Builder type to remove need for using ENV internall ([#238](https://github.com/theahaco/scaffold-stellar/pull/238))

### Fixed

- *(stellar-scaffold-cli)* use npm.cmd on windows ([#252](https://github.com/theahaco/scaffold-stellar/pull/252))
- update stellar-cli; remove unneeded features for smaller binaries ([#237](https://github.com/theahaco/scaffold-stellar/pull/237))

### Other

- Fix/missing oz contracts ([#261](https://github.com/theahaco/scaffold-stellar/pull/261))
- Try to fix just test-integration ([#260](https://github.com/theahaco/scaffold-stellar/pull/260))

## [0.0.14](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.13...stellar-scaffold-cli-v0.0.14) - 2025-10-20

### Added

- add network in target wasm path ([#213](https://github.com/theahaco/scaffold-stellar/pull/213))

### Other

- recommend to use --locked when installing binary crates ([#235](https://github.com/theahaco/scaffold-stellar/pull/235))
- Copy .env and Run git init  ([#216](https://github.com/theahaco/scaffold-stellar/pull/216))
- *(stellar-registry-cli)* release v0.0.12 ([#208](https://github.com/theahaco/scaffold-stellar/pull/208))

## [0.0.13](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.12...stellar-scaffold-cli-v0.0.13) - 2025-10-03

### Fixed

- default to development environment ([#206](https://github.com/theahaco/scaffold-stellar/pull/206))

## [0.0.12](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.11...stellar-scaffold-cli-v0.0.12) - 2025-09-26

### Fixed

- update all links ([#198](https://github.com/theahaco/scaffold-stellar/pull/198))
- If the packages do not exist, build the clients ([#201](https://github.com/theahaco/scaffold-stellar/pull/201))

## [0.0.11](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.10...stellar-scaffold-cli-v0.0.11) - 2025-09-11

### Fixed

- split dry run into separate action; bump versions to test release ([#185](https://github.com/theahaco/scaffold-stellar/pull/185))

## [0.0.10](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.9...stellar-scaffold-cli-v0.0.10) - 2025-09-11

### Added

- update CD github actions to do a dry run on a release PR ([#183](https://github.com/theahaco/scaffold-stellar/pull/183))
- `watch` command calls upgrade instead of redeploying when possible ([#149](https://github.com/theahaco/scaffold-stellar/pull/149))

## [0.0.9](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.8...stellar-scaffold-cli-v0.0.9) - 2025-09-03

### Fixed

- *(build-clients)* only `allowHttp` in dev & test ([#180](https://github.com/theahaco/scaffold-stellar/pull/180))

## [0.0.8](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.7...stellar-scaffold-cli-v0.0.8) - 2025-08-19

### Other

- Add metadata in upgrade command ([#171](https://github.com/theahaco/scaffold-stellar/pull/171))
- add vite change file test ([#148](https://github.com/theahaco/scaffold-stellar/pull/148))

## [0.0.7](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.6...stellar-scaffold-cli-v0.0.7) - 2025-07-17

### Fixed

- update to newest CLI ([#144](https://github.com/theahaco/scaffold-stellar/pull/144))

### Other

- only watch rust and toml files ([#146](https://github.com/theahaco/scaffold-stellar/pull/146))
- Add version command ([#140](https://github.com/theahaco/scaffold-stellar/pull/140))

## [0.0.6](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.5...stellar-scaffold-cli-v0.0.6) - 2025-07-11

### Fixed

- npm i to output bindings to ensure linkage ([#141](https://github.com/theahaco/scaffold-stellar/pull/141))

## [0.0.5](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.4...stellar-scaffold-cli-v0.0.5) - 2025-07-08

### Added

- use global args and printer in build / watch ([#124](https://github.com/theahaco/scaffold-stellar/pull/124))
- add --no-prompt to upgrade ([#122](https://github.com/theahaco/scaffold-stellar/pull/122))

### Fixed

- init not copying fungible token to proper directory when relative path used ([#127](https://github.com/theahaco/scaffold-stellar/pull/127))
- init flaky test failure ([#130](https://github.com/theahaco/scaffold-stellar/pull/130))
- continue building clients on error ([#88](https://github.com/theahaco/scaffold-stellar/pull/88))

### Other

- Build clients race condition ([#126](https://github.com/theahaco/scaffold-stellar/pull/126))
- add/update NFT token example on init ([#136](https://github.com/theahaco/scaffold-stellar/pull/136))

## [0.0.4](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.3...stellar-scaffold-cli-v0.0.4) - 2025-06-26

### Added

- init retrieves fresh FT interface ([#95](https://github.com/theahaco/scaffold-stellar/pull/95))

### Fixed

- *(registry-cli)* update mainnet instructions to inform about security practices and docs ([#112](https://github.com/theahaco/scaffold-stellar/pull/112))

### Other

- upgrade a project to a scaffold project ([#114](https://github.com/theahaco/scaffold-stellar/pull/114))

## [0.0.3](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.2...stellar-scaffold-cli-v0.0.3) - 2025-06-18

### Added

- add Cargo metadata ([#108](https://github.com/theahaco/scaffold-stellar/pull/108))

### Fixed

- update readmes to point to current directory instead of project ([#109](https://github.com/theahaco/scaffold-stellar/pull/109))
- only copy contents to output location ([#107](https://github.com/theahaco/scaffold-stellar/pull/107))
- ensure account gets funded even if identity already exists ([#106](https://github.com/theahaco/scaffold-stellar/pull/106))

## [0.0.2](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.1...stellar-scaffold-cli-v0.0.2) - 2025-06-11

### Added

- generate contract command ([#84](https://github.com/theahaco/scaffold-stellar/pull/84))

### Fixed

- CD binary builds  ([#97](https://github.com/theahaco/scaffold-stellar/pull/97))
- use correct repo in cargo.toml's
- out_dir fails when building clients ([#91](https://github.com/theahaco/scaffold-stellar/pull/91))

### Other

- Update README.md ([#96](https://github.com/theahaco/scaffold-stellar/pull/96))

## [0.0.1-alpha.2](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.1-alpha.1...stellar-scaffold-cli-v0.0.1-alpha.2) - 2025-05-19

### Added

- Add metadata to wasm and use in registry publish ([#46](https://github.com/theahaco/scaffold-stellar/pull/46))

## [0.0.1-alpha.1](https://github.com/theahaco/scaffold-stellar/compare/stellar-scaffold-cli-v0.0.1-alpha...stellar-scaffold-cli-v0.0.1-alpha.1) - 2025-05-13

### Other

- docs ([#39](https://github.com/theahaco/scaffold-stellar/pull/39))
