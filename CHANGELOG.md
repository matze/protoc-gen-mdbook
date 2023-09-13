# Changelog

## Unreleased

### Fixed

- Render nested message types.

### Added

- Optimization for Doxygen Markdown.

### Changed

- **Breaking**: `mdbook_opt` now takes a comma-separated list of key-value
  pairs.


## 1.2.1

**2023-01-10**

### Fixed

- Do not render duplicate items for the same message (e.g. using the same enum
  multiple times for different purposes).


## 1.2.0

**2023-01-09**

### Changed

- Render enum types.


## 1.1.0

**2023-01-06**

### Changed

- Emit all related message input/output types, i.e. if a message embeds another
  one, emit that as well.
- Refactor internal structure for type safety and clearer layering.
- Group deprecated methods.

### Fixed

- Emit `repeated` labels.


## 1.0.0

**2022-12-19**

- Initial release.
