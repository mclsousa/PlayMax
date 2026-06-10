/** Serialize all native mpv calls — concurrent FFI causes STATUS_ACCESS_VIOLATION on Windows. */
let mpvChain = Promise.resolve<void>(undefined);

export function withMpvLock<T>(fn: () => Promise<T>): Promise<T> {
  const run = mpvChain.then(fn);
  mpvChain = run.then(
    () => undefined,
    () => undefined,
  );
  return run;
}
