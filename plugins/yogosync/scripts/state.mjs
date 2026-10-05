const active = new Set(['thinking','working','waiting']);
export class TaskStates {
  sessions = new Map();
  apply(e) {
    if (!e || typeof e.session !== 'string' || !Number.isFinite(e.at) || Date.now()-e.at > 86400000) return;
    const old = this.sessions.get(e.session);
    if (old && old.at > e.at) return;
    if (e.turn && old?.turn && e.turn !== old.turn && e.event !== 'UserPromptSubmit') return;
    let state;
    switch(e.event) {
      case 'SessionStart': if (old) return; state='idle'; break;
      case 'UserPromptSubmit': state='thinking'; break;
      case 'PreToolUse': state='working'; break;
      case 'PostToolUse': state='thinking'; break;
      case 'PermissionRequest': state='waiting'; break;
      case 'Stop': state='done'; break;
      case 'Interrupt': state='interrupted'; break;
      case 'SessionEnd': this.sessions.delete(e.session); return;
      default: return;
    }
    this.sessions.set(e.session, { state, at:e.at, turn:e.turn || old?.turn || '' });
  }
  snapshot(now = Date.now()) {
    for (const [id,s] of this.sessions) if (now-s.at > 86400000) this.sessions.delete(id);
    const items = [...this.sessions.values()];
    const working = items.filter(s=>active.has(s.state));
    const latest = items.sort((a,b)=>b.at-a.at)[0];
    const state = working.some(s=>s.state==='waiting') ? 'waiting' : working.some(s=>s.state==='working') ? 'working' : working.length ? 'thinking' : latest && now-latest.at<8000 ? latest.state : 'idle';
    return { state, activeTasks: working.length, sessions:items.length, lastEventAt:latest?.at || null };
  }
}
