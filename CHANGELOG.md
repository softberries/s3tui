# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Bug Fixes

- Rename dir_path param to file_path, fix README heading([d987b14](https://github.com/softberries/s3tui/commit/d987b14be61205151b253cf6dc185c736b9939c1))


### Features

- Specify a custom credentials file path with the `--creds-file`([f0e89ef](https://github.com/softberries/s3tui/commit/f0e89efd88c5a9601e5bed8c7214e5262d612f89))


### Miscellaneous

- Release v0.4.2([92898d8](https://github.com/softberries/s3tui/commit/92898d8083125e03ee4fc9e34baa2df21aa1487e))


### Bug Fixes

- Rename dir_path param to file_path, fix README heading([d987b14](https://github.com/softberries/s3tui/commit/d987b14be61205151b253cf6dc185c736b9939c1))


### Features

- Specify a custom credentials file path with the `--creds-file`([f0e89ef](https://github.com/softberries/s3tui/commit/f0e89efd88c5a9601e5bed8c7214e5262d612f89))


### Bug Fixes

- Handle empty bucket regions in S3-compatible storage([c325ddd](https://github.com/softberries/s3tui/commit/c325dddd7bdf7c057069e4bd66419302ba274b5e))


### Miscellaneous

- Release v0.4.1([7be6997](https://github.com/softberries/s3tui/commit/7be69976e71c399676834f567f7946690aa07053))


### Refactoring

- Remove unnecessary intermediate variable client_with_location([aaace72](https://github.com/softberries/s3tui/commit/aaace7208695aee49aed63effd962cd10f4c4a27))


### Bug Fixes

- Add missing files([762ab0c](https://github.com/softberries/s3tui/commit/762ab0c762d78f6d5f0532446cc6c53687d3ccf0))


### Miscellaneous

- Release v0.4.1([4ae38b1](https://github.com/softberries/s3tui/commit/4ae38b10384d0cda1d15a37464cf2a690dccaaef))


### Other

- Added some integration and prop tests([f90da2b](https://github.com/softberries/s3tui/commit/f90da2b8fe56c13ffd9b386403621375715240f4))


### Bug Fixes

- Add missing files([762ab0c](https://github.com/softberries/s3tui/commit/762ab0c762d78f6d5f0532446cc6c53687d3ccf0))


### Other

- Added some integration and prop tests([f90da2b](https://github.com/softberries/s3tui/commit/f90da2b8fe56c13ffd9b386403621375715240f4))


## [0.4.0] - 2026-01-21

### Added

- S3-compatible storage support (MinIO, Backblaze B2, Wasabi, Cloudflare R2, etc.)
- `endpoint_url` credential option for custom S3 endpoints
- `force_path_style` credential option for path-style URL formatting
- MinIO integration tests using testcontainers
- Development and Contributing sections in README

### Features

- Add ability to keep unfinished transfers during app restarts([2b71177](https://github.com/softberries/s3tui/commit/2b71177229c96109e312b5b98fd4d1fe3c1aba4c))


### Other

- Upgrade minor version due to new features([879ac11](https://github.com/softberries/s3tui/commit/879ac11a52debf18bdfd481502ede1ac0f53306b))


### Refactoring

- Decouple s3 client creation *(client)*([afec8f2](https://github.com/softberries/s3tui/commit/afec8f29fe4b828d63cd4cfcc95272143f38344a))

- Implement Bounded Channels for backpressure *(tasks)*([bd86182](https://github.com/softberries/s3tui/commit/bd86182520932e7ab83fca97f23a5ebeea63d80b))

- Improve tokio tasks management *(tasks)*([08e12b8](https://github.com/softberries/s3tui/commit/08e12b8527b661681a33a2669fc825b19b103ac3))

- Refactor the whole transfer logic and encapsulate it in transfer manager *(transfers)*([36d1a36](https://github.com/softberries/s3tui/commit/36d1a362d92cddad76e286f517e25d21867df8f2))


### Features

- Improve status line with more context, change help screen to overlay *(ui)*([ec287e3](https://github.com/softberries/s3tui/commit/ec287e3454f38c1c6b092bcc69e3d5d4e65e3305))

- Add sorting on s3 and local components by name (f1), size (f2) and type (f3)([f2730bd](https://github.com/softberries/s3tui/commit/f2730bd8679e06ae2658a9856650ad4282161d00))

### Refactoring

- Decouple s3 client creation *(client)*([afec8f2](https://github.com/softberries/s3tui/commit/afec8f29fe4b828d63cd4cfcc95272143f38344a))

- Implement Bounded Channels for backpressure *(tasks)*([bd86182](https://github.com/softberries/s3tui/commit/bd86182520932e7ab83fca97f23a5ebeea63d80b))

- Improve tokio tasks management *(tasks)*([08e12b8](https://github.com/softberries/s3tui/commit/08e12b8527b661681a33a2669fc825b19b103ac3))

- Refactor the whole transfer logic and encapsulate it in transfer manager *(transfers)*([36d1a36](https://github.com/softberries/s3tui/commit/36d1a362d92cddad76e286f517e25d21867df8f2))

- Refactor transfer properties to transfer state enum([9c0f930](https://github.com/softberries/s3tui/commit/9c0f93016a1f049082f86fe2ce4a5baa514a53a0))

### Bug Fixes

- Fix refresh on s3 after transfer, add F5 as manual refresh action([34438cf](https://github.com/softberries/s3tui/commit/34438cf719fde977fe54a78e5b036d870ae94fb6))

- Fix progress reporting on transfers component([07cf191](https://github.com/softberries/s3tui/commit/07cf191a3d5bc50487ede00e7a0c4964f770796b))

### Other

- Enhanced transfer display with progress bar and stats([5fd9a88](https://github.com/softberries/s3tui/commit/5fd9a881bfc1316a576df2e49492a2f7b480beac))

- Upgraded dependencies([ee64f04](https://github.com/softberries/s3tui/commit/ee64f0466494d2a2c9fd7d9780db085fab0d3f8a))

- Create s3 and local error representations instead of strings([df1d507](https://github.com/softberries/s3tui/commit/df1d5071b4692db705c61ca6a7105a27b0baab14))

- Improved iterator patterns across the code base([fe5e978](https://github.com/softberries/s3tui/commit/fe5e9784cf6206a3fe1caddcfbac27418e07678e))

- Remove excessive cloning([8bae65b](https://github.com/softberries/s3tui/commit/8bae65b271b0aa596d0e1ddfc81127f162a08087))

- Unify flatten methods([91f5cd7](https://github.com/softberries/s3tui/commit/91f5cd747e484958fe701214a91305e720875151))

- Switching to release-plz([636b65c](https://github.com/softberries/s3tui/commit/636b65c1ac17d38936e033e055aab85f80ab1bae))
