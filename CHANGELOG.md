# Changelog

## [1.2.1](https://github.com/futharkd/heimdall/compare/v1.2.0...v1.2.1) (2026-05-05)


### Bug Fixes

* **service komodo:** load env for every action ([225a504](https://github.com/futharkd/heimdall/commit/225a50482298e53592a6628644b58128c56a0fc2))

## [1.2.0](https://github.com/futharkd/heimdall/compare/v1.1.0...v1.2.0) (2026-05-05)


### Features

* move formatted output to core feature ([3befc45](https://github.com/futharkd/heimdall/commit/3befc457478c2d18eceee0c238386a8c0e51c18b))

## [1.1.0](https://github.com/futharkd/heimdall/compare/v1.0.2...v1.1.0) (2026-05-05)


### Features

* **doctor:** check infisical agent service ([826a5ee](https://github.com/futharkd/heimdall/commit/826a5ee8e706fa9981da7d63be7ccba05146b2bc))
* **service:** add lifecycle commands for services ([bc4250c](https://github.com/futharkd/heimdall/commit/bc4250c300c69712ecf86a490ea2aa00e5090d01))


### Bug Fixes

* **bootstrap komodo:** add proper container names ([f72add9](https://github.com/futharkd/heimdall/commit/f72add9618c4682ecbb8d0c9ae83cc958b3e4801))

## [1.0.2](https://github.com/futharkd/heimdall/compare/v1.0.1...v1.0.2) (2026-05-05)


### Bug Fixes

* **bootstrap infisical:** create secret subdirs before agent start ([da5c00c](https://github.com/futharkd/heimdall/commit/da5c00ca65e01307bbb95a16c2fdded1103694ee))
* **io:** route config and key reads through elevation ([6ff3c94](https://github.com/futharkd/heimdall/commit/6ff3c94d0090bb5c430dfca13b2d318affca90c8))

## [1.0.1](https://github.com/futharkd/heimdall/compare/v1.0.0...v1.0.1) (2026-05-05)


### Bug Fixes

* **core:** centralize privilege elevation policy ([6d4e1b4](https://github.com/futharkd/heimdall/commit/6d4e1b42102e2b4c96c924bec7d9ca7bf0977427))

## [1.0.0](https://github.com/futharkd/heimdall/compare/v0.8.8...v1.0.0) (2026-05-05)


### ⚠ BREAKING CHANGES

* **core:** `heimdall verify doctor` is removed; use `heimdall doctor`.

### Bug Fixes

* **bootstrap infisical:** restart service instead of only starting (in case already running) ([94af442](https://github.com/futharkd/heimdall/commit/94af4420d60a68ecd5b513403b393f7671b95b30))


### Code Refactoring

* **core:** centralize doctor and execution contracts ([911bfc2](https://github.com/futharkd/heimdall/commit/911bfc25f01de732b79055ee57b33f64d4d71d1a))

## [0.8.8](https://github.com/futharkd/heimdall/compare/v0.8.7...v0.8.8) (2026-05-05)


### Bug Fixes

* **bootstrap infisical:** use better command for access checks ([a6e53ca](https://github.com/futharkd/heimdall/commit/a6e53ca26dd0c03323abdc7f902683e9be7898fd))

## [0.8.7](https://github.com/futharkd/heimdall/compare/v0.8.6...v0.8.7) (2026-05-05)


### Bug Fixes

* **bootstrap infisical:** match properly json parser and remove redundant login ([ebe9570](https://github.com/futharkd/heimdall/commit/ebe9570bf8358cec9055df44b09a0c194b976b45))

## [0.8.6](https://github.com/futharkd/heimdall/compare/v0.8.5...v0.8.6) (2026-05-05)


### Bug Fixes

* **bootstrap infisical:** add project id and add to heimdall static config ([bc5e579](https://github.com/futharkd/heimdall/commit/bc5e579abd16377fdafecbc8bff1b0ce753b0414))

## [0.8.5](https://github.com/futharkd/heimdall/compare/v0.8.4...v0.8.5) (2026-05-05)


### Bug Fixes

* **bootstrap infisical:** remove unnecessary flag from folder discovery ([c3d1655](https://github.com/futharkd/heimdall/commit/c3d1655564588b90852147f97ff4f58ee71274e4))

## [0.8.4](https://github.com/futharkd/heimdall/compare/v0.8.3...v0.8.4) (2026-05-05)


### Bug Fixes

* **bootstrap infisical:** allow selecting infisical domain ([6edb05b](https://github.com/futharkd/heimdall/commit/6edb05bd4a06e53c77cc990b4ba267c5015e51fc))

## [0.8.3](https://github.com/futharkd/heimdall/compare/v0.8.2...v0.8.3) (2026-05-05)


### Bug Fixes

* **bootstrap infisical:** login before discovery ([e8eba37](https://github.com/futharkd/heimdall/commit/e8eba378f11883b1af2cb15e5436aefe259ce0ff))

## [0.8.2](https://github.com/futharkd/heimdall/compare/v0.8.1...v0.8.2) (2026-05-05)


### Bug Fixes

* **infisical:** create parent directories with sudo before writing privileged files ([279415b](https://github.com/futharkd/heimdall/commit/279415bf938e0b59d24e5d37393db3bf6e35cb78))

## [0.8.1](https://github.com/futharkd/heimdall/compare/v0.8.0...v0.8.1) (2026-05-05)


### Bug Fixes

* **infisical:** use sudo for writing files to privileged directories ([5a5ffe9](https://github.com/futharkd/heimdall/commit/5a5ffe9af03a02b7946fda1280b0c83cdcb1f722))

## [0.8.0](https://github.com/futharkd/heimdall/compare/v0.7.0...v0.8.0) (2026-05-05)


### Features

* fully complete rework for global modular operations ([5d753b9](https://github.com/futharkd/heimdall/commit/5d753b9b104bf53bd80ef70ed70a2be560e44d92))

## [0.7.0](https://github.com/futharkd/heimdall/compare/v0.6.2...v0.7.0) (2026-05-05)


### Features

* **bootstrap:** add infisical command for secrets management ([8093dbd](https://github.com/futharkd/heimdall/commit/8093dbdca913e005049bbc6f899e3b313110e11e))

## [0.6.2](https://github.com/futharkd/heimdall/compare/v0.6.1...v0.6.2) (2026-05-04)


### Bug Fixes

* **harden firewall:** use proper parameters for the established connections ([4a4bd11](https://github.com/futharkd/heimdall/commit/4a4bd118936cd97cda00d884fec2e52b68675580))

## [0.6.1](https://github.com/futharkd/heimdall/compare/v0.6.0...v0.6.1) (2026-05-04)


### Bug Fixes

* **harden firewall:** set-default-zone doesn't use permanent ([7706dda](https://github.com/futharkd/heimdall/commit/7706ddac2260c6f07b4dd6c194dd65eaabd19619))

## [0.6.0](https://github.com/futharkd/heimdall/compare/v0.5.2...v0.6.0) (2026-05-04)


### Features

* add operation kinds and ensure packages are installed ([9bc0a40](https://github.com/futharkd/heimdall/commit/9bc0a407b451f38a0b3d7b2b5769209a63c8d2a9))
* **harden firewall:** ensure firewalld is installed ([3682a31](https://github.com/futharkd/heimdall/commit/3682a3171bed78a38305933672a3c6179c28cbf6))

## [0.5.2](https://github.com/futharkd/heimdall/compare/v0.5.1...v0.5.2) (2026-05-04)


### Bug Fixes

* **harden ssh:** allow new ssh port with selinux ([197c68a](https://github.com/futharkd/heimdall/commit/197c68a864e0527dc6327e8ef6f881854f4d26b1))

## [0.5.1](https://github.com/futharkd/heimdall/compare/v0.5.0...v0.5.1) (2026-05-04)


### Bug Fixes

* **harden ssh:** reload or restart service for edge cases ([e44996c](https://github.com/futharkd/heimdall/commit/e44996cac1248caa00e71358ddf46167fb34fcb0))

## [0.5.0](https://github.com/futharkd/heimdall/compare/v0.4.1...v0.5.0) (2026-05-04)


### Features

* add global sudo fallback and always sudo for specific commands ([a31a17b](https://github.com/futharkd/heimdall/commit/a31a17b293bbfa6a9403734693c5e4beb40698be))

## [0.4.1](https://github.com/futharkd/heimdall/compare/v0.4.0...v0.4.1) (2026-05-04)


### Bug Fixes

* **harden ssh:** add more validation steps and fix pattern matching ([8655ce4](https://github.com/futharkd/heimdall/commit/8655ce4f2400e7903fdc2692eedb0ed427276825))

## [0.4.0](https://github.com/futharkd/heimdall/compare/v0.3.0...v0.4.0) (2026-05-04)


### Features

* **harden ssh:** remember sudo approval across operations in same execution ([843a011](https://github.com/futharkd/heimdall/commit/843a0119f58ca4fd596ebac4f451b17e6e5ea27c))


### Bug Fixes

* **harden ssh:** add 'Access denied' to permission error detection for systemd services ([c49adfd](https://github.com/futharkd/heimdall/commit/c49adfd6dfc19090bd305bf3bac1d0850887910d))

## [0.3.0](https://github.com/futharkd/heimdall/compare/v0.2.3...v0.3.0) (2026-05-04)


### Features

* add docker bootstrap command ([61e9123](https://github.com/futharkd/heimdall/commit/61e9123a0b7fa9e0b254dae03b4d08c2d3d3d1d5))
* **harden ssh:** add sudo fallback on permission denied + verify port listening ([6b78db0](https://github.com/futharkd/heimdall/commit/6b78db05cbc63a6470446178ed9cb22502a67ed6))

## [0.2.3](https://github.com/futharkd/heimdall/compare/v0.2.2...v0.2.3) (2026-05-03)


### Bug Fixes

* use proper ids for release workflows ([000b1e7](https://github.com/futharkd/heimdall/commit/000b1e79db3090dfae9900edf55c2cd3419189db))

## [0.2.2](https://github.com/futharkd/heimdall/compare/v0.2.1...v0.2.2) (2026-05-03)


### Bug Fixes

* directly move release builds to release-please ([368a41f](https://github.com/futharkd/heimdall/commit/368a41f34befd6eb29748effe44906e74325f61b))

## [0.2.1](https://github.com/futharkd/heimdall/compare/v0.2.0...v0.2.1) (2026-05-03)


### Bug Fixes

* add release-assets workflow, remove dead release job ([74bb9f2](https://github.com/futharkd/heimdall/commit/74bb9f29578eff643368f8e9d4f9d269f1d7ef09))

## [0.2.0](https://github.com/futharkd/heimdall/compare/v0.1.0...v0.2.0) (2026-05-03)


### Features

* **bootstrap:** add sudo/wheel group membership ([9f84d38](https://github.com/futharkd/heimdall/commit/9f84d3845a994cf6926316f175d7decef88bf276))
