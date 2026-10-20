import { describe, expect, test } from 'vitest';

import { X_IN_APP_BROWSER_AGENT } from './in-app-browser';

const X_IPHONE =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 26_6_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/23G83 Twitter for iPhone/11.20';
const SAFARI_IPHONE =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 26_6_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.6 Mobile/15E148 Safari/604.1';
const TWITTERBOT =
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_1) AppleWebKit/601.2.4 (KHTML, like Gecko) Version/9.0.1 Safari/601.2.4 facebookexternalhit/1.1 Facebot Twitterbot/1.0';
const X_ANDROID =
  'Mozilla/5.0 (Linux; Android 15; NLA-LX2P Build/HONORNLA-L32P; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/141.0.0.0 Mobile Safari/537.36 TwitterAndroid/11.20.0-release.01';

describe('X_IN_APP_BROWSER_AGENT', () => {
  test("matches X's in-app browser on iPhone", () => {
    expect(X_IPHONE.includes(X_IN_APP_BROWSER_AGENT)).toBe(true);
  });

  test.each([
    ['Safari on iPhone', SAFARI_IPHONE],
    ["X's link preview crawler", TWITTERBOT],
    ["X's in-app browser on Android", X_ANDROID],
  ])('does not match %s', (_name, userAgent) => {
    expect(userAgent.includes(X_IN_APP_BROWSER_AGENT)).toBe(false);
  });
});
