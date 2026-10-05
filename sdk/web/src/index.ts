/**
 * 예매 사이트 연동 SDK.
 *
 * - 브라우저 측: `requestGuardToken()`으로 보안 브라우저에서 서명된 토큰을 받아 예매 요청에 첨부한다.
 * - 서버 측: `verifyGuardToken()`으로 검증 API에 토큰을 보내 허용/확인/차단 결정을 받는다.
 */

export type Decision = "allow" | "challenge" | "block";

export interface VerifyResult {
  decision: Decision;
  /** 0~100, 높을수록 사람일 가능성이 높다. */
  score: number;
}

/** 보안 브라우저가 페이지에 주입하는 인터페이스 (apps/browser/bootstrap/collector-bootstrap.js). */
export interface GuardHost {
  version: number;
  getToken(nonce: string): Promise<string>;
}

export class GuardUnavailableError extends Error {
  constructor() {
    super("Guard Browser is not detected on this page");
    this.name = "GuardUnavailableError";
  }
}

function host(scope: unknown): GuardHost | undefined {
  const g = (scope as { __GUARD__?: GuardHost } | undefined)?.__GUARD__;
  return g && typeof g.getToken === "function" ? g : undefined;
}

export function isGuardBrowser(scope: unknown = globalThis): boolean {
  return host(scope) !== undefined;
}

/** `nonce`는 사이트 서버가 요청마다 새로 발급한 값이어야 한다. */
export async function requestGuardToken(nonce: string, scope: unknown = globalThis): Promise<string> {
  const g = host(scope);
  if (!g) throw new GuardUnavailableError();
  return g.getToken(nonce);
}

export interface VerifyOptions {
  /** 검증 API 기본 주소. 예: https://verify.example.com */
  endpoint: string;
  fetch?: typeof fetch;
  /** 지연 상한. 초과하면 차단하지 않고 "challenge"로 처리한다. */
  timeoutMs?: number;
}

/**
 * 사이트 백엔드에서 호출한다.
 * 검증 서버 장애·시간 초과는 정상 사용자 차단으로 이어지지 않도록 "challenge"로 처리한다.
 * 토큰이 거부(401)되면 "block"이 아닌 "challenge"를 반환한다. 차단은 서버가 명시적으로 내린 경우만.
 */
export async function verifyGuardToken(token: string, opts: VerifyOptions): Promise<VerifyResult> {
  const doFetch = opts.fetch ?? fetch;
  const fallback: VerifyResult = { decision: "challenge", score: 0 };
  try {
    const res = await doFetch(new URL("/v1/verify", opts.endpoint), {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ token }),
      signal: AbortSignal.timeout(opts.timeoutMs ?? 300),
    });
    if (!res.ok) return fallback;
    const body = (await res.json()) as Partial<VerifyResult>;
    if (!isDecision(body.decision) || typeof body.score !== "number") return fallback;
    return { decision: body.decision, score: body.score };
  } catch {
    return fallback;
  }
}

function isDecision(v: unknown): v is Decision {
  return v === "allow" || v === "challenge" || v === "block";
}
