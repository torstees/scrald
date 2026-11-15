// The title Scrald draws in the macOS title bar: "• Title — ~/notes/a.md".
// When the path doesn't fit, its first folders give way to "…", so the
// file name stays (the system's title bar would cut the end instead).

/** The separator a path uses: `\` for a Windows path, else `/`. */
export function separatorOf(path: string): string {
  return path.includes("\\") && !path.includes("/") ? "\\" : "/";
}

/**
 * The longest form of `path` that `fits`: the whole path, or "…" and its
 * last folders (`…/shopping/tasks.md`), down to "…/tasks.md", which is
 * returned even if it doesn't fit (CSS then cuts it).
 */
export function fitPath(path: string, fits: (text: string) => boolean): string {
  if (fits(path)) return path;
  const sep = separatorOf(path);
  const parts = path.split(sep);
  for (let drop = 1; drop < parts.length - 1; drop++) {
    const shorter = `…${sep}${parts.slice(drop).join(sep)}`;
    if (fits(shorter)) return shorter;
  }
  return parts.length > 1 ? `…${sep}${parts[parts.length - 1]}` : path;
}
