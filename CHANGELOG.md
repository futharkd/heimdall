# Changelog

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
