# Session Operation Window Refactor

## Goal

Refactor `src/gui/title_bar/session_window` so file responsibilities match the
project convention while preserving the existing create/edit session behavior.

## Constraints

- Only files under `src/gui/title_bar/session_window` may change.
- The window component is named `SessionOperationWindow`.
- `mod.rs` contains module declarations, structs, enums, and
  `impl Render for SessionOperationWindow`.
- `core.rs` contains non-UI behavior.
- `ui.rs` contains UI construction code.
- Do not import or store `gui::session::SessionComponent`.
- All session persistence uses `cx.global::<Storage>().session`.
- The existing caller of `open_edit_session_window` must remain source-compatible
  because it is outside the permitted edit scope.

## Architecture

### `mod.rs`

Defines `ConnectionProtocol`, `FormSection`, `SessionFormMode`, and
`SessionOperationWindow`. It owns only the `Render` implementation, which
composes UI helpers defined in `ui.rs`.

### `core.rs`

Implements construction, input initialization, draft creation and validation,
submit/cancel handlers, window opening, and port parsing.

Create uses `Storage.session.insert`. Edit uses `Storage.session.update`
directly. Successful persistence emits the existing
`GlobalEvent::CreateSession`, allowing the existing session-list subscription to
reload without coupling this window to `SessionComponent`.

`open_edit_session_window` retains a generic, unused second parameter so its
existing external call remains valid without naming or importing
`SessionComponent`.

### `ui.rs`

Implements field rows, protocol selection, connection/proxy panels, and section
selection. These helpers may update component presentation state but do not
perform persistence.

## Error Handling

Validation and storage errors are converted to strings and displayed in the
window's existing error area. The window closes only after successful
persistence.

## Verification

- Formatting succeeds with `cargo fmt -- --check`.
- The project compiles with `cargo check`.
- Available relevant tests pass.
- A final diff confirms that no path outside
  `src/gui/title_bar/session_window` changed.
