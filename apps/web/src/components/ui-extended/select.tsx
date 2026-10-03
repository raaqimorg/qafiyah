'use client';

import { Check, ChevronDown, X } from 'lucide-react';
import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { type ArabicNounForms, formatArabicCount, formatArabicNumber } from '@/lib/arabic';
import { cn } from '@/lib/utils';

import type { SelectOption } from '@/lib/constants/taxonomy-data';
import type React from 'react';

const ALL_OPTION_VALUE = '__all__';

type Props = {
  readonly options: readonly SelectOption[];
  readonly value: string | readonly string[];
  readonly onChange: (value: string | string[]) => void;
  readonly placeholder: string;
  readonly placeholderNounForms?: ArabicNounForms;
  readonly disabled?: boolean;
  readonly className?: string;
  readonly multiple?: boolean;
  readonly sortOptions?: boolean;
  readonly clearValue?: string;
  readonly allOptionLabel?: string;
  readonly showCounts?: boolean;
  readonly onOpenChange?: ((isOpen: boolean) => void) | undefined;
  readonly labelledBy?: string;
  readonly onEscape?: () => void;
};

export function Select({
  options,
  value,
  onChange,
  placeholder,
  placeholderNounForms,
  disabled = false,
  className,
  multiple = false,
  sortOptions = true,
  clearValue,
  allOptionLabel,
  showCounts = false,
  onOpenChange,
  labelledBy,
  onEscape,
}: Props) {
  const [isOpen, setIsOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const comboboxRef = useRef<HTMLDivElement>(null);
  const listboxId = useId();
  const defaultLabel = multiple ? 'اختيار متعدد' : 'اختيار';

  const selectedValues = useMemo(() => (typeof value === 'string' ? [value] : value), [value]);

  const visibleOptions = useMemo(
    () =>
      sortOptions
        ? [...options].sort((a, b) => a.label.localeCompare(b.label, 'ar'))
        : [...options],
    [options, sortOptions]
  );

  const displayedOptions = useMemo(
    () =>
      multiple && allOptionLabel !== undefined
        ? [{ value: ALL_OPTION_VALUE, label: allOptionLabel }, ...visibleOptions]
        : visibleOptions,
    [visibleOptions, multiple, allOptionLabel]
  );

  const isAllSelected = useMemo(() => selectedValues.length === 0, [selectedValues.length]);

  const isOptionSelected = useCallback(
    (option: SelectOption) =>
      option.value === ALL_OPTION_VALUE ? isAllSelected : selectedValues.includes(option.value),
    [isAllSelected, selectedValues]
  );

  const setOpen = useCallback(
    (next: boolean) => {
      setIsOpen(next);
      onOpenChange?.(next);
    },
    [onOpenChange]
  );

  const toggleOpen = (e?: React.SyntheticEvent) => {
    e?.stopPropagation();
    if (!disabled) {
      setOpen(!isOpen);
    }
  };

  const toggleOption = useCallback(
    (option: SelectOption) => {
      if (multiple) {
        if (option.value === ALL_OPTION_VALUE) {
          onChange([]);
        } else if (selectedValues.includes(option.value)) {
          onChange(selectedValues.filter((v) => v !== option.value));
        } else {
          onChange([...selectedValues, option.value]);
        }
      } else {
        onChange(option.value);
        setOpen(false);
      }
    },
    [multiple, onChange, selectedValues, setOpen]
  );

  const canClear = multiple ? selectedValues.length > 0 : value !== (clearValue ?? '');

  const clearSelection = (e?: React.SyntheticEvent) => {
    e?.stopPropagation();
    onChange(multiple ? [] : (clearValue ?? ''));
    comboboxRef.current?.focus();
  };

  const getDisplayValue = () => {
    if (selectedValues.length === 0) return placeholder;
    if (selectedValues.length === 1) {
      const option = options.find((o) => o.value === selectedValues[0]);
      return option?.label ?? placeholder;
    }
    return placeholderNounForms
      ? formatArabicCount({ count: selectedValues.length, nounForms: placeholderNounForms })
      : String(selectedValues.length);
  };

  useEffect(() => {
    if (!isOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !(e.target instanceof Node && containerRef.current.contains(e.target))
      ) {
        setOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen, setOpen]);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.target !== e.currentTarget) return;
    if (!isOpen) {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggleOpen();
      }
      return;
    }
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setHighlightedIndex((prev) => (prev + 1) % displayedOptions.length);
        break;
      case 'ArrowUp':
        e.preventDefault();
        setHighlightedIndex(
          (prev) => (prev - 1 + displayedOptions.length) % displayedOptions.length
        );
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (displayedOptions[highlightedIndex] !== undefined)
          toggleOption(displayedOptions[highlightedIndex]);
        break;
      case 'Escape':
        e.preventDefault();
        onEscape?.();
        setOpen(false);
        break;
      case 'Tab':
        setOpen(false);
        break;
    }
  };

  return (
    <div ref={containerRef} className={cn('relative w-full min-w-0', className)}>
      <div
        ref={comboboxRef}
        role="combobox"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={listboxId}
        className={cn(
          'flex h-12 w-full items-center justify-between rounded-md border border-border px-3 py-2 text-base text-text-muted shadow-none focus-ring transition-colors hover:bg-surface-hover',
          disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer',
          { 'border-text': isOpen }
        )}
        onClick={toggleOpen}
        onKeyDown={handleKeyDown}
        tabIndex={disabled ? -1 : 0}
        aria-labelledby={labelledBy}
        aria-label={labelledBy === undefined ? defaultLabel : undefined}
        aria-activedescendant={isOpen ? `option-${highlightedIndex}` : undefined}
      >
        <div className="flex w-full min-w-0 items-center justify-between">
          <span className={cn('truncate', selectedValues.length === 0 && 'text-text-subtle')}>
            {getDisplayValue()}
          </span>
          <div className="flex items-center gap-1.5">
            {canClear && (
              <button
                type="button"
                onClick={clearSelection}
                className="rounded-sm text-text-subtle focus-ring transition-colors hover:text-text"
                aria-label="مسح الاختيار"
              >
                <X className="h-4 w-4" />
              </button>
            )}
            <ChevronDown
              className={cn('h-4 w-4 transition-transform duration-200', isOpen && 'rotate-180')}
            />
          </div>
        </div>
      </div>

      {isOpen && (
        <ul
          id={listboxId}
          className={cn(
            'absolute z-50 w-full overflow-auto rounded-md border border-border bg-surface-raised p-2',
            'max-h-60 focus:outline-none'
          )}
          role="listbox"
          aria-multiselectable={multiple}
          style={{
            position: 'absolute',
            left: 0,
            right: 0,
            marginTop: '0.25rem',
            overflowY: 'auto',
          }}
        >
          {displayedOptions.map((option, index) => {
            const isSelected = isOptionSelected(option);
            return (
              <li
                key={option.value}
                id={`option-${index}`}
                role="option"
                aria-selected={isSelected}
                className={cn(
                  'cursor-pointer rounded-md px-2 py-2 text-base',
                  index === highlightedIndex && 'bg-surface-hover',
                  isSelected && 'text-text'
                )}
                onClick={() => toggleOption(option)}
                onMouseDown={(e) => e.preventDefault()}
                onMouseEnter={() => setHighlightedIndex(index)}
              >
                <div className="flex items-center gap-2">
                  <div
                    className={cn(
                      'flex h-4 w-4 shrink-0 items-center justify-center rounded border bg-surface-raised',
                      multiple ? 'rounded' : 'rounded-full',
                      isSelected ? 'border-text bg-text' : 'border-border'
                    )}
                  >
                    {isSelected &&
                      (multiple ? (
                        <Check className="h-3 w-3 text-surface" />
                      ) : (
                        <div className="h-2 w-2 rounded-full bg-surface" />
                      ))}
                  </div>
                  <span>{option.label}</span>
                  {showCounts && option.poemsCount !== undefined && (
                    <Badge variant="outline" className="shrink-0 text-xs text-text-subtle">
                      {formatArabicNumber(option.poemsCount)}
                    </Badge>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
