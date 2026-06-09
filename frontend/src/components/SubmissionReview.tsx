import { useEffect, useState } from 'react';
import { CheckCircle2, Loader2, MessageSquare, RotateCcw } from 'lucide-react';
import { api } from '../lib/api';
import type { SubmissionDetail, SubmissionSummary } from '../lib/types';

type SubmissionReviewProps = {
  submissions: SubmissionSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onReviewed: () => void;
};

export function SubmissionReview({ submissions, selectedId, onSelect, onReviewed }: SubmissionReviewProps) {
  const [detail, setDetail] = useState<SubmissionDetail | null>(null);
  const [feedback, setFeedback] = useState('');
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      setFeedback('');
      return;
    }

    setLoading(true);
    setError('');
    api
      .getSubmission(selectedId)
      .then((submission) => {
        setDetail(submission);
        setFeedback(submission.parent_feedback ?? '');
      })
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : 'Could not load submission');
      })
      .finally(() => setLoading(false));
  }, [selectedId]);

  async function submitReview(status: 'reviewed' | 'needs_work') {
    if (!selectedId) {
      return;
    }
    setSaving(true);
    setError('');
    try {
      const updated = await api.reviewSubmission(selectedId, { feedback, status });
      setDetail(updated);
      onReviewed();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not save review');
    } finally {
      setSaving(false);
    }
  }

  const pendingCount = submissions.filter((item) => item.status === 'pending').length;

  return (
    <section className="review-panel">
      <div className="pane-title">
        <span>
          <MessageSquare size={16} />
          Code Reviews
        </span>
        {pendingCount > 0 ? <span className="badge">{pendingCount} pending</span> : null}
      </div>

      <div className="review-layout">
        <div className="review-list">
          {submissions.length === 0 ? (
            <p className="muted">No submissions yet.</p>
          ) : (
            submissions.map((item) => (
              <button
                key={item.id}
                type="button"
                className={item.id === selectedId ? 'review-item active' : 'review-item'}
                onClick={() => onSelect(item.id)}
              >
                <span>{item.lesson_title}</span>
                <small>
                  {item.submitter_name} · {item.status.replace('_', ' ')}
                </small>
              </button>
            ))
          )}
        </div>

        <div className="review-detail">
          {!selectedId ? (
            <p className="muted">Select a submission to review.</p>
          ) : loading ? (
            <div className="modal-loading">
              <Loader2 className="spin" size={24} />
            </div>
          ) : detail ? (
            <>
              <div className="review-meta">
                <strong>{detail.lesson_title}</strong>
                <span>{detail.submitter_name}</span>
                {detail.note ? <p className="review-note">Note: {detail.note}</p> : null}
              </div>
              <label>
                Code
                <pre className="review-code">{detail.code_snapshot}</pre>
              </label>
              <label>
                Output
                <pre className="review-code">{detail.stdout || '(no output)'}</pre>
              </label>
              <label>
                Feedback
                <textarea
                  value={feedback}
                  onChange={(event) => setFeedback(event.target.value)}
                  rows={4}
                  placeholder="Great job! Next try adding..."
                />
              </label>
              {error ? <p className="error-text">{error}</p> : null}
              <div className="review-actions">
                <button type="button" disabled={saving} onClick={() => void submitReview('needs_work')}>
                  <RotateCcw size={16} />
                  Needs work
                </button>
                <button type="button" disabled={saving} onClick={() => void submitReview('reviewed')}>
                  <CheckCircle2 size={16} />
                  Mark reviewed
                </button>
              </div>
            </>
          ) : null}
        </div>
      </div>
    </section>
  );
}
