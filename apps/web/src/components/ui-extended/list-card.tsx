import { Badge } from '@/components/ui/badge';
import { TYPE } from '@/lib/constants/design-tokens';
import { cn } from '@/lib/utils';

type ListCardProps = {
  readonly title: string;
  readonly subtitle: string;
  readonly href: string;
  readonly era?: string | undefined;
  readonly headingTag?: 'h2' | 'h4';
  readonly className?: string;
};

export function ListCard({
  title,
  subtitle,
  href,
  era,
  headingTag: Heading = 'h2',
  className = '',
}: ListCardProps) {
  return (
    <a
      href={href}
      className={cn(
        'group flex w-full break-inside-avoid flex-col gap-1 border-b border-border px-2 py-4 transition-colors hover:cursor-pointer hover:bg-surface-hover focus-visible:ring-1 focus-visible:ring-text focus-visible:outline-none sm:py-5',
        className
      )}
    >
      <Heading className="w-full min-w-0">
        <span
          className={cn(
            TYPE.heading,
            'truncate-ink block text-text duration-300 group-hover:text-text-muted group-hover:underline group-hover:underline-offset-4'
          )}
        >
          {title.replaceAll('"', '')}
        </span>
      </Heading>
      <div className="flex w-full min-w-0 items-baseline justify-between gap-2">
        <p className={cn(TYPE.body, 'truncate-ink min-w-0 shrink text-text-subtle duration-300')}>
          {subtitle}
        </p>
        {era !== undefined && (
          <Badge variant="outline" className="shrink-0 text-xs text-text-subtle">
            {era}
          </Badge>
        )}
      </div>
    </a>
  );
}
