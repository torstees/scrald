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

/** The title before the path: "• Title" with unsaved changes. */
export function titleName(title: { title: string; dirty: boolean }): string {
  return `${title.dirty ? "\u2022 " : ""}${title.title}`;
}

/**
 * The text for a system title bar that can't shorten the path itself
 * (Windows, Linux): "• Title — path", with the path shortened by `fitPath`
 * so the whole text `fits`. Without a path, just the title.
 */
export function fittedTitle(
  title: { title: string; dirty: boolean; path: string | null },
  fits: (text: string) => boolean,
): string {
  const name = titleName(title);
  if (!title.path) return name;
  const prefix = `${name} \u2014 `;
  return prefix + fitPath(title.path, (path) => fits(prefix + path));
}
