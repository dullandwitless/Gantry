import { useCallback, useEffect, useRef, useState } from 'react';
import { ListedRunnable } from '../../types';

/** What a studio has open: a deployed item, or a new, unsaved one. */
export type RunnableSelection = { kind: 'existing'; path: string } | { kind: 'new' };

export interface RunnablePicker {
  items: ListedRunnable[];
  /** `null` while the first listing for the workspace is in flight. */
  selection: RunnableSelection | null;
  select: (selection: RunnableSelection) => void;
  /** Lists again, then opens `path` if given (e.g. after a save). */
  refresh: (path?: string) => Promise<void>;
}

/**
 * Lists a workspace's scripts or flows and tracks which one a studio has open.
 * On a workspace change it opens the first item, or a new one when there are
 * none (specs/scripts-studio.md and specs/flow-editor.md, section 2.0).
 *
 * `list` must be a stable reference. Responses that arrive after a newer
 * listing has started are discarded.
 */
export function useRunnablePicker(
  workspace: string,
  list: (workspace: string) => Promise<ListedRunnable[]>,
  onError: (error: Error) => void,
): RunnablePicker {
  const [items, setItems] = useState<ListedRunnable[]>([]);
  const [selection, setSelection] = useState<RunnableSelection | null>(null);

  const onErrorRef = useRef(onError);
  onErrorRef.current = onError;
  const latestRequest = useRef(0);

  const load = useCallback(
    async (open: (found: ListedRunnable[]) => RunnableSelection) => {
      const request = ++latestRequest.current;
      try {
        const found = await list(workspace);
        if (request !== latestRequest.current) return;
        setItems(found);
        setSelection(open(found));
      } catch (e) {
        if (request !== latestRequest.current) return;
        onErrorRef.current(e instanceof Error ? e : new Error(String(e)));
        setItems([]);
        setSelection({ kind: 'new' });
      }
    },
    [workspace, list],
  );

  useEffect(() => {
    setItems([]);
    setSelection(null);
    if (!workspace) return;
    void load((found) => (found.length > 0 ? { kind: 'existing', path: found[0].path } : { kind: 'new' }));
    return () => {
      // Invalidate the in-flight listing so a late response cannot overwrite a newer workspace.
      latestRequest.current++;
    };
  }, [workspace, load]);

  const refresh = useCallback(
    (path?: string) =>
      load((found) => {
        if (path !== undefined && found.some((item) => item.path === path)) {
          return { kind: 'existing', path };
        }
        return found.length > 0 ? { kind: 'existing', path: found[0].path } : { kind: 'new' };
      }),
    [load],
  );

  return { items, selection, select: setSelection, refresh };
}
