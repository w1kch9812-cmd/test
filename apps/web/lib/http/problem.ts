const TYPE_BASE = "https://gongzzang.com/errors";

export interface ProblemDetailsInput {
  type: string; // e.g. "auth/state-mismatch" → 자동 prefix
  title: string;
  status: number;
  detail?: string;
  instance?: string;
}

export class ProblemDetails {
  readonly type: string;
  readonly title: string;
  readonly status: number;
  readonly detail?: string;
  readonly instance?: string;

  constructor(input: ProblemDetailsInput) {
    this.type = input.type.startsWith("http") ? input.type : `${TYPE_BASE}/${input.type}`;
    this.title = input.title;
    this.status = input.status;
    this.detail = input.detail;
    this.instance = input.instance;
  }

  toJSON(): Record<string, unknown> {
    const out: Record<string, unknown> = {
      type: this.type,
      title: this.title,
      status: this.status,
    };
    if (this.detail !== undefined) out.detail = this.detail;
    if (this.instance !== undefined) out.instance = this.instance;
    return out;
  }

  toResponse(): Response {
    return new Response(JSON.stringify(this.toJSON()), {
      status: this.status,
      headers: { "content-type": "application/problem+json" },
    });
  }
}

export function problem(input: ProblemDetailsInput): ProblemDetails {
  return new ProblemDetails(input);
}
