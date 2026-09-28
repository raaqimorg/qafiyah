import { vi } from 'vitest';

type RouteHandler = () => Response | Promise<Response>;

export function fakeFetch(routes: Readonly<Record<string, RouteHandler>>) {
  const mock = vi.fn((input: string | URL | Request, init?: RequestInit) => {
    const request = input instanceof Request ? input : new Request(input, init);
    const method = request.method;
    const pathname = new URL(request.url).pathname;
    const key = `${method} ${pathname}`;
    const handler = routes[key];
    if (handler === undefined) throw new Error(`no fakeFetch route for ${key} (${request.url})`);
    return handler();
  });
  vi.stubGlobal('fetch', mock);
  return mock;
}

export function failedBody(cause: unknown): Response {
  return new Response(
    new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode('ab'));
      },
      pull(controller) {
        controller.error(cause);
      },
    })
  );
}
