import { useEffect, useState, type FormEvent } from 'react';
import { Loader2, X } from 'lucide-react';
import { api } from '../lib/api';
import type { CreateLessonPayload, LessonManageDetail, UpdateLessonPayload } from '../lib/types';

type LessonEditorProps = {
  lessonId: string | null;
  onClose: () => void;
  onSaved: () => void;
};

const emptyForm: CreateLessonPayload = {
  id: '',
  title: '',
  prompt: '',
  description: '',
  hint: '',
  difficulty: 'Beginner',
  starter_code: 'print("hello")\n',
  expected_stdout: 'hello\n',
  hidden_tests: '[]',
  is_published: true,
};

export function LessonEditor({ lessonId, onClose, onSaved }: LessonEditorProps) {
  const isEditing = lessonId !== null;
  const [form, setForm] = useState<CreateLessonPayload>(emptyForm);
  const [loading, setLoading] = useState(isEditing);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!lessonId) {
      setForm(emptyForm);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError('');
    api
      .getLessonManage(lessonId)
      .then((lesson: LessonManageDetail) => {
        setForm({
          id: lesson.id,
          title: lesson.title,
          prompt: lesson.prompt,
          description: lesson.description,
          hint: lesson.hint,
          difficulty: lesson.difficulty,
          starter_code: lesson.starter_code,
          expected_stdout: lesson.expected_stdout,
          hidden_tests: lesson.hidden_tests,
          is_published: lesson.is_published,
          sort_order: lesson.sort_order,
        });
      })
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : 'Could not load lesson');
      })
      .finally(() => setLoading(false));
  }, [lessonId]);

  function updateField<K extends keyof CreateLessonPayload>(key: K, value: CreateLessonPayload[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError('');
    try {
      if (isEditing && lessonId) {
        const payload: UpdateLessonPayload = {
          title: form.title,
          prompt: form.prompt,
          description: form.description,
          hint: form.hint,
          difficulty: form.difficulty,
          starter_code: form.starter_code,
          expected_stdout: form.expected_stdout,
          hidden_tests: form.hidden_tests,
          is_published: form.is_published,
          sort_order: form.sort_order,
        };
        await api.updateLesson(lessonId, payload);
      } else {
        await api.createLesson(form);
      }
      onSaved();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not save lesson');
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="modal-backdrop" role="presentation" onClick={onClose}>
      <form
        className="modal-panel lesson-editor"
        role="dialog"
        aria-labelledby="lesson-editor-title"
        onClick={(event) => event.stopPropagation()}
        onSubmit={(event) => void handleSubmit(event)}
      >
        <div className="modal-header">
          <h2 id="lesson-editor-title">{isEditing ? 'Edit Lesson' : 'New Lesson'}</h2>
          <button type="button" className="icon-button" aria-label="Close" onClick={onClose}>
            <X size={18} />
          </button>
        </div>

        {loading ? (
          <div className="modal-loading">
            <Loader2 className="spin" size={24} />
          </div>
        ) : (
          <div className="lesson-editor-grid">
            <label>
              Lesson ID
              <input
                value={form.id}
                onChange={(event) => updateField('id', event.target.value)}
                placeholder="hello-python"
                disabled={isEditing}
                required
              />
            </label>
            <label>
              Title
              <input
                value={form.title}
                onChange={(event) => updateField('title', event.target.value)}
                placeholder="Hello, Python"
                required
              />
            </label>
            <label className="full-width">
              Prompt
              <input
                value={form.prompt}
                onChange={(event) => updateField('prompt', event.target.value)}
                placeholder="Print a greeting."
                required
              />
            </label>
            <label className="full-width">
              Description
              <textarea
                value={form.description}
                onChange={(event) => updateField('description', event.target.value)}
                rows={2}
              />
            </label>
            <label className="full-width">
              Hint
              <textarea value={form.hint} onChange={(event) => updateField('hint', event.target.value)} rows={2} />
            </label>
            <label>
              Difficulty
              <input
                value={form.difficulty ?? 'Beginner'}
                onChange={(event) => updateField('difficulty', event.target.value)}
              />
            </label>
            <label>
              Sort order
              <input
                type="number"
                value={form.sort_order ?? ''}
                onChange={(event) =>
                  updateField('sort_order', event.target.value ? Number(event.target.value) : undefined)
                }
              />
            </label>
            <label className="checkbox-field">
              <input
                type="checkbox"
                checked={form.is_published ?? true}
                onChange={(event) => updateField('is_published', event.target.checked)}
              />
              Published
            </label>
            <label className="full-width">
              Starter code
              <textarea
                value={form.starter_code}
                onChange={(event) => updateField('starter_code', event.target.value)}
                rows={4}
                spellCheck={false}
              />
            </label>
            <label className="full-width">
              Expected output
              <textarea
                value={form.expected_stdout}
                onChange={(event) => updateField('expected_stdout', event.target.value)}
                rows={3}
                spellCheck={false}
                required
              />
            </label>
          </div>
        )}

        {error ? <p className="error-text">{error}</p> : null}

        <div className="modal-actions">
          <button type="button" className="secondary-button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" disabled={loading || saving}>
            {saving ? <Loader2 className="spin" size={16} /> : null}
            {isEditing ? 'Save Lesson' : 'Create Lesson'}
          </button>
        </div>
      </form>
    </div>
  );
}
