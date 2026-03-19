import { useCallback, useEffect, useRef, useState } from "react";
import type { SetupStepOutputEvent } from "@/lib/ipc";

export type TerminalLine = { text: string; isStderr: boolean; key: number };

export function useStreamingTerminal(invoke: (runId: string) => Promise<void>) {
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [running, setRunning] = useState(false);
  const [termSuccess, setTermSuccess] = useState<boolean | null>(null);
  const termBodyRef = useRef<HTMLDivElement>(null);
  const lineKeyRef = useRef(0);

  const showTerminal = running || termSuccess !== null;

  useEffect(() => {
    if (termBodyRef.current) {
      termBodyRef.current.scrollTop = termBodyRef.current.scrollHeight;
    }
  }, [lines]);

  const run = useCallback(async () => {
    const runId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    setLines([]);
    setTermSuccess(null);
    setRunning(true);

    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<SetupStepOutputEvent>("setup_step_output", (e) => {
      if (e.payload.run_id !== runId) return;
      if (e.payload.done) {
        setRunning(false);
        setTermSuccess(e.payload.success ?? false);
        unlisten();
      } else if (e.payload.line) {
        setLines((prev) => [
          ...prev,
          { text: e.payload.line, isStderr: e.payload.is_stderr, key: lineKeyRef.current++ },
        ]);
      }
    });

    try {
      await invoke(runId);
    } catch (err) {
      unlisten();
      setRunning(false);
      setTermSuccess(false);
      setLines((prev) => [
        ...prev,
        { text: String(err), isStderr: true, key: lineKeyRef.current++ },
      ]);
    }
  }, [invoke]);

  return { lines, running, termSuccess, showTerminal, termBodyRef, run };
}
