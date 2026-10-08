import { githubUrl, xProfileUrl } from '@/lib/urls';
import { cn } from '@/lib/utils';

import { IslandErrorBoundary } from './island-error-boundary';
import { RandomPoemButton } from './random-poem-button';
import { SettingsDialog } from './settings-dialog';

function Separator({ className }: { readonly className?: string }) {
  return (
    <span className={cn('text-text-subtle', className)} aria-hidden="true">
      •
    </span>
  );
}

export function Footer({ className }: { className?: string }) {
  return (
    <IslandErrorBoundary feature="footer" fallback={null}>
      <footer
        className={cn(
          'relative flex w-full items-center justify-between gap-4 py-4 text-xs text-text-muted xxs:text-sm md:text-base xl:text-lg',
          className
        )}
      >
        <nav className="flex items-center gap-1 md:gap-2">
          <a
            href={xProfileUrl()}
            target="_blank"
            rel="noopener noreferrer"
            className="-mr-1 inline-flex min-h-11 items-center rounded-md px-1 transition-colors hover:text-text focus-visible:ring-1 focus-visible:ring-text focus-visible:outline-none md:-mr-2 md:px-2"
          >
            تويتر
          </a>
          <Separator />
          <a
            href={githubUrl()}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex min-h-11 items-center rounded-md px-1 transition-colors hover:text-text focus-visible:ring-1 focus-visible:ring-text focus-visible:outline-none md:px-2"
          >
            قتهب
          </a>
          <Separator />
          <a
            href="/about"
            className="inline-flex min-h-11 items-center rounded-md px-1 transition-colors hover:text-text focus-visible:ring-1 focus-visible:ring-text focus-visible:outline-none md:px-2"
          >
            من نحن
          </a>
          <Separator className="hidden lg:inline" />
          <a
            href="/developers"
            className="hidden min-h-11 items-center rounded-md px-1 transition-colors hover:text-text focus-visible:ring-1 focus-visible:ring-text focus-visible:outline-none md:px-2 lg:inline-flex"
          >
            للمطورين
          </a>
          <Separator />
          <SettingsDialog className="inline-flex min-h-11 items-center rounded-md px-1 focus-visible:ring-1 focus-visible:ring-text focus-visible:outline-none md:px-2" />
        </nav>
        <RandomPoemButton />
      </footer>
    </IslandErrorBoundary>
  );
}
