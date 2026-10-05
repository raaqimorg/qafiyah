const KNOWN_METHODS: ReadonlySet<string> = new Set([
  'GET',
  'HEAD',
  'POST',
  'PUT',
  'PATCH',
  'DELETE',
  'OPTIONS',
]);

const NULL_BODY_STATUSES: ReadonlySet<number> = new Set([101, 103, 204, 205, 304]);

export type RequestAttributes = {
  readonly 'http.request.method': string;
  readonly 'http.route': string;
  readonly 'http.response.status_code': number;
};

export type RecordDuration = (seconds: number, attributes: RequestAttributes) => void;

export function methodLabel(method: string): string {
  return KNOWN_METHODS.has(method) ? method : '_OTHER';
}

function onBodyDone(response: Response, done: () => void): Response {
  const source = response.body;
  if (source === null || NULL_BODY_STATUSES.has(response.status)) {
    done();
    return response;
  }
  let finished = false;
  const once = (): void => {
    if (finished) return;
    finished = true;
    done();
  };
  const reader = source.getReader();
  const body = new ReadableStream<Uint8Array>({
    async pull(controller) {
      try {
        const chunk = await reader.read();
        if (chunk.done) {
          once();
          controller.close();
          return;
        }
        controller.enqueue(chunk.value);
      } catch (cause) {
        once();
        controller.error(cause);
      }
    },
    cancel(reason) {
      once();
      return reader.cancel(reason);
    },
  });
  return new Response(body, {
    status: response.status,
    statusText: response.statusText,
    headers: response.headers,
  });
}

export async function timeRequest(
  record: RecordDuration,
  request: { readonly method: string; readonly route: string },
  next: () => Promise<Response>,
  now: () => number = () => performance.now()
): Promise<Response> {
  const started = now();
  const finish = (status: number): void => {
    record((now() - started) / 1000, {
      'http.request.method': methodLabel(request.method),
      'http.route': request.route,
      'http.response.status_code': status,
    });
  };
  let response: Response;
  try {
    response = await next();
  } catch (cause) {
    finish(500);
    throw cause;
  }
  return onBodyDone(response, () => finish(response.status));
}
