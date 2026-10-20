import type { Host, SurfaceName } from './surfaces';
import type { Result } from 'neverthrow';

export type Expectation = 'ok' | 'client-error' | 'not-found' | 'healthy';

export type BodyCheck = (body: string, res: Response) => Result<void, string>;

export type Check = {
  readonly name: string;
  readonly run: (body: string, res: Response) => Result<void, string>;
};

export type Probe = {
  readonly url: string;
  readonly note: string;
  readonly expect?: Expectation;
  readonly method?: string;
  readonly body?: string;
  readonly headers?: Readonly<Record<string, string>>;
  readonly redirect?: 'manual' | 'follow';
  readonly check?: BodyCheck;
  readonly checks?: readonly Check[];
  readonly host?: Host;
  readonly originOnly?: boolean;
  readonly prodOnly?: boolean;
  readonly surfaces?: readonly SurfaceName[];
  readonly unkeyed?: boolean;
  readonly apiKey?: string | undefined;
};

export type Suite = {
  readonly name: string;
  readonly probes: readonly Probe[];
  readonly serial?: boolean;
};
