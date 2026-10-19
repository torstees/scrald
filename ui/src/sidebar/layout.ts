// Which sidebars a document window shows (DESIGN.md §8.2, §10): Contents
// at the left, and the properties panel at the top of the document, as a
// tab beside Contents, or in a sidebar of its own at the right.
import type { PropertiesPlacement } from "../lib/types";

export type Panel = "contents" | "properties";

/** What the user asked for, before the window's width has its say. */
export interface SidebarState {
  /** The left sidebar is shown (Contents, or its Properties tab). */
  leftOpen: boolean;
  /**
   * Properties are shown: the panel at the top is expanded, the left
   * sidebar is on its Properties tab, or the right sidebar is shown. One
   * setting (`properties.open`), so each placement remembers it.
   */
  propertiesOpen: boolean;
  /** The panel opened most recently, which wins when both don't fit. */
  last: Panel;
}

/** What the window shows. */
export interface Layout {
  /** The left sidebar's tab, or null when it's hidden. */
  left: Panel | null;
  /** The right properties sidebar is shown. */
  right: boolean;
}

export function layout(placement: PropertiesPlacement, state: SidebarState, wide: boolean): Layout {
  if (placement === "left") {
    return { left: state.leftOpen ? (state.propertiesOpen ? "properties" : "contents") : null, right: false };
  }
  const left = state.leftOpen ? "contents" : null;
  const right = placement === "right" && state.propertiesOpen;
  // Two sidebars would cramp the reading column of a narrow window: show
  // the one opened last, without forgetting the other.
  if (left && right && !wide) {
    return state.last === "properties" ? { left: null, right: true } : { left, right: false };
  }
  return { left, right };
}

/** Whether `panel` is on screen. At the top, properties count while expanded. */
export function isShown(panel: Panel, placement: PropertiesPlacement, state: SidebarState, wide: boolean): boolean {
  const shown = layout(placement, state, wide);
  if (panel === "contents") return shown.left === "contents";
  if (placement === "top") return state.propertiesOpen;
  return shown.left === "properties" || shown.right;
}

/**
 * Shows `panel`, or hides it if it's on screen: `Ctrl+\` for Contents,
 * `Ctrl+Shift+\` for properties, and the left sidebar's tabs.
 */
export function toggle(panel: Panel, placement: PropertiesPlacement, state: SidebarState, wide: boolean): SidebarState {
  const hide = isShown(panel, placement, state, wide);
  if (panel === "contents") {
    if (hide) return { ...state, leftOpen: false };
    // In the left sidebar, Contents is the other tab.
    const propertiesOpen = placement === "left" ? false : state.propertiesOpen;
    return { leftOpen: true, propertiesOpen, last: "contents" };
  }
  if (placement === "left") {
    // Hiding keeps the tab, so the sidebar comes back as it was.
    return hide ? { ...state, leftOpen: false } : { leftOpen: true, propertiesOpen: true, last: "properties" };
  }
  return hide ? { ...state, propertiesOpen: false } : { ...state, propertiesOpen: true, last: "properties" };
}

/** Picks a tab of the left sidebar (properties placed at the left). */
export function chooseTab(panel: Panel): SidebarState {
  return { leftOpen: true, propertiesOpen: panel === "properties", last: panel };
}
