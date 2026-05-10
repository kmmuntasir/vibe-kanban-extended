# Further Investigation — Local Mode UI Issues

**Date:** 2026-05-11
**Branch:** `analysis/enable-local-backend-and-db`

---

## Issue 1 & 2: Stale UI After Mutations (Column Toggle + Status Change)

### Symptoms
- Column visibility toggle reverts after save, corrects only after polling or page refresh
- Issue status change from "To Do" to "In Progress" only reflects after polling

### Root Cause: Fire-and-Forget Fallback Refresh + Bypassed Mutation System

**`maybeRefreshFallbackAfterMutation` does not await the refresh**
[collections.ts:616-619](packages/web-core/src/shared/lib/electric/collections.ts#L616-L619):
```ts
function maybeRefreshFallbackAfterMutation(sourceKey: string): void {
  if (!isSourceFallbackLocked(sourceKey)) return;
  invalidateFallbackCache(sourceKey);
  refreshFallbackSource(sourceKey);  // fire-and-forget — no await
}
```

`refreshFallbackSource` ([line 244](packages/web-core/src/shared/lib/electric/collections.ts#L244)) calls `void refresher()` — discards the promise. The mutation handler returns immediately, the UI shows the optimistic state, and the fallback GET completes asynchronously later.

**`refreshPromise` dedup causes lost updates**
[collections.ts:452-456](packages/web-core/src/shared/lib/electric/collections.ts#L452-L456):
If two mutations fire in quick succession (e.g., column toggle save triggers multiple status updates):
1. Mutation A's refresh starts (GET in flight)
2. Mutation B calls `refreshNow()` → receives the same promise as A (dedup)
3. Mutation A's GET completes → `applySnapshot` replaces all collection data
4. Mutation B's change is NOT in the snapshot → **lost until next 30s poll**

**Bulk operations bypass the collection mutation system entirely**

| Operation | File | Line | Bug |
|---|---|---|---|
| Drag-and-drop status/sort change | [KanbanContainer.tsx](packages/web-core/src/features/kanban/ui/KanbanContainer.tsx) | 738 | Calls `bulkUpdateIssues()` directly — no mutation handler, no fallback refresh triggered |
| Bulk status toggle save | [RemoteProjectsSettingsSection.tsx](packages/web-core/src/shared/dialogs/settings/settings/RemoteProjectsSettingsSection.tsx) | 813 | Calls `bulkUpdateProjectStatuses()` directly — same bypass |

These use `makeRequest()` directly instead of going through the Electric collection's `update()` method. No `onUpdate` handler runs, no `maybeRefreshFallbackAfterMutation` is called. The collection keeps stale data.

**Kanban drag-and-drop effect race**
[KanbanContainer.tsx:484-488](packages/web-core/src/features/kanban/ui/KanbanContainer.tsx#L484-L488):
```ts
useEffect(() => {
  if (isSyncingRef.current) { return; }  // suppresses sync for 500ms
  // ... rebuilds items from filteredIssues ...
}, [filteredIssues, statuses, kanbanFilters]);
```
After `handleDragEnd` sets `isSyncingRef.current = true` for 500ms, the next collection change overwrites the drag's local state with stale collection data.

### Fix Plan

1. **Await the fallback refresh in mutation handlers**
   In `buildMutationHandlers` ([collections.ts:616-755](packages/web-core/src/shared/lib/electric/collections.ts#L616-L755)), change `maybeRefreshFallbackAfterMutation(sourceKey)` to `await refreshFallbackSource(sourceKey)`.

2. **Make `refreshNow` always start a new fetch after dedup resolves**
   After the deduped promise resolves, if more mutations happened during the fetch, trigger another refresh.

3. **Route bulk operations through the collection mutation system**
   In `KanbanContainer.tsx` and `RemoteProjectsSettingsSection.tsx`, replace direct `bulkUpdateIssues`/`bulkUpdateProjectStatuses` calls with collection-based mutations that properly trigger fallback refresh.

---

## Issue 3: Workspace Footer Missing on Kanban Cards

### Root Cause: `local_workspace_id` is `null` in Shape Response

The kanban card renders workspace footers in [KanbanContainer.tsx:1096-1117](packages/web-core/src/features/kanban/ui/KanbanContainer.tsx#L1096-L1117). Workspaces are filtered by `workspacesByIssueId` ([lines 590-645](packages/web-core/src/features/kanban/ui/KanbanContainer.tsx#L590-L645)) which requires:
```ts
!!workspace.local_workspace_id  // line 601: must be truthy
```

Our shape fallback ([shape_fallbacks.rs](crates/server/src/routes/kanban_v1/shape_fallbacks.rs)) sets:
```rust
"local_workspace_id": serde_json::Value::Null,
```

Since `local_workspace_id` is `null`, ALL workspaces are filtered out — no footer rendered.

### Fix

Change `"local_workspace_id"` from `null` to the workspace's actual `id`:
```rust
"local_workspace_id": r.id,  // was: serde_json::Value::Null
```

This makes the workspace card match the local workspace lookup and pass the filter.

---

## Issue 4: Sidebar Workspace Footer Not Clickable

### Root Cause: Same `local_workspace_id: null` Issue

In [IssueWorkspacesSection.tsx:61-67](packages/ui/src/components/IssueWorkspacesSection.tsx#L61-L67):
```tsx
onClick={
  onWorkspaceClick &&
  localWorkspaceId &&           // ← null → no onClick
  workspace.isOwnedByCurrentUser
    ? () => onWorkspaceClick(localWorkspaceId)
    : undefined
}
```

`localWorkspaceId` comes from `workspace.local_workspace_id` which is `null` in our shape response. Same fix as Issue 3.

---

## Issue 5: Merge/PR Doesn't Move Issue to "Done"

### Root Cause: "Done" Auto-Advance Logic Exists Only on Remote Server

The status auto-advance chain for merge/PR:

| Event | Local Handler | Remote Call | Status Change |
|---|---|---|---|
| Workspace linked | [links.rs](crates/server/src/routes/workspaces/links.rs) `auto_move_issue_to_in_progress` | `create_workspace` | Local: In Progress / Remote: In Progress |
| PR created | [pr.rs:330](crates/server/src/routes/workspaces/pr.rs#L330) | `sync_pr_to_remote` → `upsert_pull_request` (Open) | **Remote only**: In Review |
| Direct merge | [git.rs:246](crates/server/src/routes/workspaces/git.rs#L246) | `sync_local_workspace_merge_to_remote` | **Remote only**: Done (if all PRs merged) |

The "Done" logic lives in [crates/remote/src/db/issues.rs:520-598](crates/remote/src/db/issues.rs#L520-L598):
- `sync_status_from_workflow_signal` with `WorkMerged` checks all linked PRs — only moves to "Done" if every PR is merged
- This is triggered by remote API calls that fail in local mode (no remote client)

The local server has NO equivalent. The merge handler in `git.rs` performs the git merge but doesn't update the issue status.

### Fix Plan

1. **Add `auto_move_issue_to_done`** in [links.rs](crates/server/src/routes/workspaces/links.rs) (next to existing `auto_move_issue_to_in_progress`)

2. **Add `auto_move_issue_to_in_review`** in [pr.rs](crates/server/src/routes/workspaces/pr.rs) — called after local PR creation

3. **Call `auto_move_issue_to_done` from [git.rs](crates/server/src/routes/workspaces/git.rs)** — after successful merge, if workspace has linked issue

The pattern for all three is the same as the existing `auto_move_issue_to_in_progress`:
- Look up the target status by name ("In review" or "Done")
- Check current status allows transition
- Update `issues.status_id`

### Files to Change

| File | Change |
|------|--------|
| [links.rs](crates/server/src/routes/workspaces/links.rs) | Add `auto_move_issue_to_done` function |
| [pr.rs](crates/server/src/routes/workspaces/pr.rs) | Add `auto_move_issue_to_in_review` after PR creation |
| [git.rs](crates/server/src/routes/workspaces/git.rs) | Call `auto_move_issue_to_done` after merge |
| [shape_fallbacks.rs](crates/server/src/routes/kanban_v1/shape_fallbacks.rs) | Fix `local_workspace_id` from `null` to workspace `id` |
| [collections.ts](packages/web-core/src/shared/lib/electric/collections.ts) | Await fallback refresh in mutation handlers |

---

## Issue 1 & 2 Appendix: Full Mutation Sequence in Fallback Mode

| Step | What Happens | Timing |
|------|-------------|--------|
| 1 | `collection.update(id, updater)` applies change optimistically | Synchronous |
| 2 | `wrappedOnUpdate` handler fires → HTTP PATCH sent | Async |
| 3 | Handler awaits PATCH response | ~50-200ms |
| 4 | `maybeRefreshFallbackAfterMutation()` fires `refreshNow()` WITHOUT await | Fire-and-forget |
| 5 | Handler returns `undefined` (fallback-locked) | Same tick |
| 6 | Collection marks mutation "persisted" — optimistic change becomes "real" | Same tick |
| 7 | UI shows the change | React re-render |
| 8 | (async) `refreshNow` GET completes → `applySnapshot` truncates + re-inserts all rows | ~100-500ms later |
| 9 | Collection replaced by server snapshot — if mutation not in snapshot, change lost | On GET response |
| 10 | 30-second interval fires next refresh | 30s later |

The gap between step 6 (mutation "persisted") and step 9 (snapshot applied) is where the race lives. If another mutation's snapshot arrives first (via the dedup mechanism), this mutation's change is overwritten.
