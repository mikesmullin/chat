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
