<script lang="ts">
  import { i18n } from '$lib/i18n';
  import { formatDate } from '$lib/utils/formatters';

  interface Props {
    poll: any;
    class?: string;
  }

  let { poll, class: extraClass = '' }: Props = $props();

  interface NormalizedChoice {
    text: string;
    votes: number;
    percent: number;
  }

  let parsed = $derived.by(() => {
    if (!poll || typeof poll !== 'object') return null;

    const title = String(poll.title || poll.question || poll.prompt || '').trim();
    const description = String(poll.description || poll.body || '').trim();

    const rawChoices = poll.choices || poll.options || poll.answers || [];
    if (!Array.isArray(rawChoices) || rawChoices.length === 0) return null;

    let totalVotes = 0;
    const rawTotal = Number(poll.total_votes ?? poll.total ?? poll.votes);
    if (!isNaN(rawTotal) && rawTotal > 0) {
      totalVotes = rawTotal;
    }

    const items: Array<{ text: string; votes: number }> = [];
    let calculatedSum = 0;

    for (const c of rawChoices) {
      if (!c) continue;
      let text = '';
      let votes = 0;

      if (typeof c === 'string') {
        text = c;
      } else if (typeof c === 'object') {
        text = String(c.text ?? c.choice ?? c.label ?? c.option ?? c.name ?? '').trim();
        const v = Number(c.votes ?? c.num_votes ?? c.count ?? c.vote_count ?? c.voters_count ?? 0);
        votes = isNaN(v) ? 0 : v;
      }

      if (text) {
        items.push({ text, votes });
        calculatedSum += votes;
      }
    }

    if (items.length === 0) return null;

    const finalTotal = totalVotes > 0 ? totalVotes : calculatedSum;

    const choices: NormalizedChoice[] = items.map((item) => {
      const percent = finalTotal > 0 ? Math.round((item.votes / finalTotal) * 1000) / 10 : 0;
      return {
        text: item.text,
        votes: item.votes,
        percent
      };
    });

    const startDate = poll.created_at || poll.start_at || poll.published_at || poll.starts_at || poll.start;
    const endDate = poll.closes_at || poll.closed_at || poll.end_at || poll.ends_at || poll.expires_at || poll.end;

    return {
      title,
      description,
      choices,
      totalVotes: finalTotal,
      startDate: startDate ? formatDate(startDate) : null,
      endDate: endDate ? formatDate(endDate) : null
    };
  });
</script>

{#if parsed}
  <div class="poll-block {extraClass}">
    {#if parsed.title}
      <h3 class="poll-question">{parsed.title}</h3>
    {/if}

    {#if parsed.description}
      <p class="poll-desc">{parsed.description}</p>
    {/if}

    <div class="poll-list">
      {#each parsed.choices as choice}
        <div class="poll-row">
          <div
            class="poll-bar"
            style:width={`${Math.min(100, Math.max(0, choice.percent))}%`}
          ></div>
          <div class="poll-row-content">
            <span class="choice-name">{choice.text}</span>
            <span class="choice-count">{choice.votes.toLocaleString()}</span>
          </div>
        </div>
      {/each}
    </div>

    <div class="poll-meta">
      {#if parsed.startDate || parsed.endDate}
        <span>
          {parsed.startDate || ''}
          {#if parsed.startDate && parsed.endDate} — {/if}
          {parsed.endDate || ''}
        </span>
        {#if parsed.totalVotes > 0}
          <span class="meta-dot">|</span>
        {/if}
      {/if}
      <span>{parsed.totalVotes.toLocaleString()} {i18n.t('post.poll_votes') || 'votes'}</span>
    </div>
  </div>
{/if}

<style>
  .poll-block {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 16px 0;
    width: 100%;
  }

  .poll-question {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.4;
    margin: 0;
  }

  .poll-desc {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0;
  }

  .poll-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }

  .poll-row {
    position: relative;
    overflow: hidden;
    height: 36px;
    border-radius: var(--radius-sm, 6px);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .poll-bar {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    background: rgba(255, 255, 255, 0.12);
    pointer-events: none;
    transition: width 0.35s ease-out;
  }

  .poll-row-content {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 100%;
    padding: 0 12px;
    gap: 12px;
  }

  .choice-name {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .choice-count {
    font-size: 13px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .poll-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .meta-dot {
    opacity: 0.5;
  }
</style>
