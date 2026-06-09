export type User = {
  id: string;
  username: string;
  display_name: string;
  role: 'parent' | 'child';
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
  description: string;
  difficulty: string;
};

export type LessonDetail = LessonSummary & {
  hint: string;
  starter_code: string;
};

export type LessonManageDetail = LessonDetail & {
  expected_stdout: string;
  hidden_tests: string;
  is_published: boolean;
  sort_order: number;
};

export type SubmissionSummary = {
  id: string;
  lesson_id: string;
  lesson_title: string;
  submitter_name: string;
  status: 'pending' | 'reviewed' | 'needs_work';
  note: string;
  parent_feedback: string | null;
  created_at: string;
  reviewed_at: string | null;
};

export type SubmissionDetail = SubmissionSummary & {
  code_snapshot: string;
  stdout: string;
};

export type CreateLessonPayload = {
  id: string;
  title: string;
  prompt: string;
  description: string;
  hint: string;
  difficulty?: string;
  starter_code: string;
  expected_stdout: string;
  hidden_tests?: string;
  is_published?: boolean;
  sort_order?: number;
};

export type UpdateLessonPayload = Partial<Omit<CreateLessonPayload, 'id'>>;

export type CreateSubmissionPayload = {
  lesson_id: string;
  code_snapshot: string;
  stdout: string;
  note?: string;
};

export type ReviewSubmissionPayload = {
  feedback: string;
  status: 'reviewed' | 'needs_work';
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
  | { type: 'started'; runId: string }
  | { type: 'stdout'; runId: string; text: string }
  | { type: 'stderr'; runId: string; text: string }
  | { type: 'input_request'; runId: string; prompt: string }
  | { type: 'result'; runId: string; status: 'ok' | 'error' | 'timeout' | 'stopped'; durationMs: number };
