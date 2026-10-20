import { Idiomorph } from 'idiomorph';

import { feelingUrl } from '@/lib/urls';

const LEAVE_MS = 300;

const isFeelingPath = (url: URL): boolean =>
  url.pathname === feelingUrl() || url.pathname.startsWith(`${feelingUrl()}/`);

export const isFeelingNavigation = (from: URL, to: URL): boolean =>
  isFeelingPath(from) && isFeelingPath(to);

function fadeOutBeforeRemoving(node: Node): boolean {
  if (!(node instanceof SVGElement) || !Object.hasOwn(node.dataset, 'feelingNode')) return true;
  node.removeAttribute('id');
  node.setAttribute('aria-hidden', 'true');
  node.style.setProperty('--leave-ms', `${LEAVE_MS}ms`);
  node.dataset['leaving'] = '';
  setTimeout(() => node.remove(), LEAVE_MS);
  return false;
}

function markEntering(node: Node): void {
  if (node instanceof SVGElement) node.dataset['entering'] = '';
}

function mainOf(page: ParentNode): Element {
  const main = page.querySelector('#main');
  if (main === null) throw new Error('a feelings page has no #main to morph');
  return main;
}

export function morphMain(current: ParentNode, next: ParentNode): void {
  void Idiomorph.morph(mainOf(current), mainOf(next), {
    callbacks: { beforeNodeRemoved: fadeOutBeforeRemoving, afterNodeAdded: markEntering },
  });
}
