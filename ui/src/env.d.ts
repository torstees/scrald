/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Fallbacks a test build forces on (src/lib/support.ts); unset in real builds. */
  readonly VITE_SCRALD_FALLBACKS?: string;
}
