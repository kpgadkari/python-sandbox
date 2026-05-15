export type User = {
  id: string;
  username: string;
  display_name: string;
};

export type ProjectSummary = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
};

export type ProjectDetail = ProjectSummary & {
  files: Record<string, string>;
};

export type LessonSummary = {
  id: string;
  title: string;
  prompt: string;
};

export type LessonDetail = LessonSummary & {
  starter_code: string;
  expected_stdout: string;
};

export type RunRequest = {
  type: 'run';
  runId: string;
  files: Record<string, string>;
  entrypoint: 'main.py';
  stdin?: string[];
};

export type WorkerEvent =
  | { type: 'ready' }
  | { type: 'stdout'; runId: string; text: string }
  | { type: 'stderr'; runId: string; text: string }
  | { type: 'input_request'; runId: string; prompt: string }
  | { type: 'result'; runId: string; status: 'ok' | 'error' | 'timeout' | 'stopped'; durationMs: number };
