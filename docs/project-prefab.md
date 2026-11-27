# Project property inheritance

Projects inherit from the slash-separated prefixes of their name; a session
inherits from its project, with the session's own nonempty values winning.
An absent, null, or empty text value inherits. The first version includes
purpose, notes, routing, memory budget, workspace root, and worktree
configuration. Title, project membership, participants, model, reasoning,
allowlists, and runtime state remain session properties.

New sessions start with no routing override; the effective default remains
`everyone`. Existing sessions retain their stored routing choice. The editor's
Inherit choice clears that override. Workspace roots and worktrees are
optional; configured roots must be absolute existing directories.

A project stores only a worktree's repo and patterns. The queue gate resolves
an inherited worktree for each session and stores its provisioned record
separately from authored session settings. A later group change therefore
cannot turn inherited configuration into a session override. Previously
provisioned checkouts remain session-owned until session deletion, preserving
any work in them.

Editing group properties requires `project.create`; signed-in non-admins also
need edit access to every session affected by that group. Group listings and
settings are visible through sessions the viewer may read.

When a sandboxed agent starts, its session's provisioned checkout is mounted
read-write at `/work`, together with the Git common directory needed by the
checkout. If no worktree is configured, an explicit workspace root is mounted
at `/work` instead. Existing live sandboxes are reused only when their
recorded mounts match; otherwise the turn reports that the sandbox must be
removed before the new checkout can be used. An older sandbox with unknown
mounts is not reused for a configured checkout.

The tab bar's new-session button starts the new session in the current
session's project. The New project form defaults its parent to the current
session's project, and lets the user select a different parent.
