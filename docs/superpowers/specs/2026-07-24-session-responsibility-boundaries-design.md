# Session Responsibility Boundaries Design

## Goal

Separate the saved-session sidebar from the active Workspace session system.
Each feature owns its own state and business operations. Cross-feature
communication uses a minimal global event bus.

## Global event boundary

`GlobalState` is an empty GPUI event bus. It contains no repositories, session
catalog, snapshots, CRUD methods, or application state.

It exposes exactly two events:

```rust
pub enum GlobalEvent {
    CreateActiveSession(SessionProfile),
    SessionProfileDeleted(String),
}
```

- `CreateActiveSession` carries a complete immutable connection profile so the
  Workspace never queries the sidebar database.
- `SessionProfileDeleted` carries the saved profile ID. It is emitted only
  after the database deletion succeeds.

## Saved-session sidebar

The sidebar feature is split into three responsibilities:

```text
LeftSession (GUI)
    -> LeftSessionService (use cases and observable state)
        -> LeftSessionStore (saved-session data and repository access)
            -> SessionRepository
                -> SQLite
```

### LeftSession

`LeftSession` owns only view-specific state and components:

- draggable list entity;
- action handlers and context menu;
- subscription to `LeftSessionService` changes.

It does not hold or call `SessionCatalog`, `WorkspaceService`,
`SessionRepository`, or SQLite.

### LeftSessionService

`LeftSessionService` is a GPUI entity used by the sidebar and session form.
It:

- exposes the current saved profiles and storage error for rendering;
- handles create, update, delete, and connect use cases;
- emits a local `LeftSessionEvent::Changed` after successful mutations;
- emits `GlobalEvent::CreateActiveSession` when the user connects;
- emits `GlobalEvent::SessionProfileDeleted` after successful deletion.

The title-bar new/edit form receives the same `Entity<LeftSessionService>` and
submits through it. It does not use `GlobalState` for database operations.

### LeftSessionStore

`LeftSessionStore` owns the `Box<dyn SessionRepository>`, cached
`Vec<SessionProfile>`, and storage initialization/read error. It:

- loads profiles during construction;
- validates and persists create/update/delete operations;
- updates its cache only after repository operations succeed;
- contains no GPUI entities, subscriptions, or events.

The existing `SessionCatalog` behavior moves into this store and the obsolete
catalog type is removed.

## Active Workspace sessions

Workspace session behavior is split into:

```text
Workspace GUI
    -> WorkspaceSessionService
        -> WorkspaceSessionStore
            -> InMemoryWorkspaceSessionStore
        -> TerminalService
```

### WorkspaceSessionService

`WorkspaceSessionService` is the active-session use-case layer. It:

- subscribes to the two global events;
- creates, activates, and closes active Workspace sessions;
- starts and stops the corresponding terminal runtime;
- emits `WorkspaceSessionEvent::Changed` for Workspace views;
- exposes read-only active-session data to the GUI.

It owns the global-event subscription so the Workspace view does not translate
global events into business calls.

### WorkspaceSessionStore

`WorkspaceSessionStore` is an application-layer interface for active-session
state. It stores complete opened-session records containing both:

- `WorkspaceSession`, which supplies the unique active-tab ID;
- the `SessionProfile` snapshot used to open and label that tab.

The interface supports listing opened sessions, reading the active ID,
inserting, activating, removing one session, and removing all sessions for a
saved profile.

### InMemoryWorkspaceSessionStore

The current implementation stores active sessions only in memory. Application
restart does not restore tabs.

The interface is intentionally replaceable. A future
`SqliteWorkspaceSessionStore` may persist and restore tabs without changing
the Workspace GUI or terminal orchestration API. Persistent restoration is
outside this implementation.

## Composition

The application composition root creates:

1. `GlobalState` event bus;
2. `LeftSessionStore` with the SQLite repository;
3. `LeftSessionService` with its store and global event bus;
4. `InMemoryWorkspaceSessionStore`;
5. `WorkspaceSessionService` with the Workspace store, terminal service, and
   global event bus.

`HomeView` receives the service entities and passes:

- `LeftSessionService` to the sidebar and title bar;
- `WorkspaceSessionService` and `TerminalService` to the Workspace view.

The old `AppServices` global container is unnecessary once these dependencies
are passed explicitly and should be removed.

## Data flows

### Connect

```text
LeftSession action
-> LeftSessionService finds its local saved profile
-> GlobalEvent::CreateActiveSession(profile)
-> WorkspaceSessionService
-> WorkspaceSessionStore inserts opened session
-> TerminalService starts SSH
-> WorkspaceSessionEvent::Changed
-> Workspace GUI redraws
```

### Delete

```text
LeftSession action
-> LeftSessionService
-> LeftSessionStore deletes from repository
-> cache removes profile
-> LeftSessionEvent::Changed
-> GlobalEvent::SessionProfileDeleted(profile_id)
-> WorkspaceSessionService closes matching tabs and terminals
-> WorkspaceSessionEvent::Changed
```

### Create and edit

```text
Session form
-> LeftSessionService
-> LeftSessionStore
-> SessionRepository / SQLite
-> LeftSessionEvent::Changed
-> LeftSession redraws
```

## Error handling

- Repository errors return to `LeftSessionService` and then to the initiating
  form or action.
- Failed mutations do not change the cached saved-session list.
- Failed deletion does not emit `SessionProfileDeleted`.
- A connect request for a missing saved profile is ignored and does not emit a
  global event.
- Workspace in-memory operations are deterministic and do not produce storage
  errors in this implementation.

## Testing

- `LeftSessionStore` tests cover loading and successful/failed CRUD cache
  consistency with a real in-memory SQLite repository.
- `WorkspaceSessionStore` tests cover insertion, activation, neighbor
  selection after close, and removal by profile ID.
- Service tests cover the decision logic separately where it can be tested
  without rendering GPUI elements.
- The full existing `cargo test` suite must pass after the refactor.

## Out of scope

- Persisting or restoring Workspace tabs.
- Adding global events beyond `CreateActiveSession` and
  `SessionProfileDeleted`.
- Changing SSH, terminal buffering, or terminal rendering behavior.
