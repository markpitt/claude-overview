// Paths and links every page uses. Paths go through the base so the site serves anywhere.
const base = import.meta.env.BASE_URL.replace(/\/?$/, '/');

export const path = (page = '') => `${base}${page}`;

export const REPO = 'markpitt/claude-overview';
export const REPO_URL = `https://github.com/${REPO}`;
export const RELEASES_URL = `${REPO_URL}/releases`;
export const LATEST_URL = `${REPO_URL}/releases/latest`;

/** The site's own pages, in the order the docs list them. */
export const DOCS = [
  { page: 'install/', label: 'Install' },
  { page: 'using-it/', label: 'Using it' },
  { page: 'settings/', label: 'Settings' },
  { page: 'updates/', label: 'Updates and releases' },
  { page: 'privacy/', label: 'Privacy' },
] as const;

export type Page = '' | (typeof DOCS)[number]['page'];
